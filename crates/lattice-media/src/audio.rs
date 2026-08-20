//! Prepared timeline audio shared by export and Studio monitoring.

use lattice_core::{AudioPlan, Time, Timeline, audio_plan_from_timeline};
use serde::Serialize;
use thiserror::Error;

use crate::backend::PcmBuffer;
use crate::decode::collect_audio_sources;
use crate::export::{ExportError, PreviewOptions};
use crate::mix::{MixSpec, mix_plan, time_to_frames};

/// Why timeline audio could not be prepared or addressed.
#[derive(Debug, Error)]
pub enum AudioMixError {
    #[error("invalid audio mix format: sample rate {sample_rate} Hz, {channels} channel(s)")]
    InvalidMixSpec { sample_rate: u32, channels: u16 },
    #[error("audio window {window_index} has no media source")]
    MissingWindowSource { window_index: usize },
    #[error("generated audio `{media_name}` has no readable speech artifact in lattice.lock.json")]
    MissingGeneratedAsset { media_name: String },
    #[error("audio source `{media_name}` is unavailable: {source}")]
    SourceUnavailable {
        media_name: String,
        generated: bool,
        #[source]
        source: Box<ExportError>,
    },
    #[error("audio source `{media_name}` decoded to no PCM frames")]
    EmptySource { media_name: String, generated: bool },
    #[error("timeline audio mix failed: {source}")]
    Mix {
        #[source]
        source: Box<ExportError>,
    },
    #[error("audio playhead {playhead} is outside 0s..={duration}")]
    PlayheadOutOfRange { playhead: Time, duration: Time },
}

impl From<AudioMixError> for ExportError {
    fn from(error: AudioMixError) -> Self {
        match error {
            AudioMixError::InvalidMixSpec {
                sample_rate,
                channels,
            } => Self::Map(format!(
                "invalid audio mix format: sample rate {sample_rate} Hz, {channels} channel(s)"
            )),
            AudioMixError::MissingWindowSource { .. } => Self::MissingSource,
            AudioMixError::MissingGeneratedAsset { media_name } => Self::MissingMedia(format!(
                "generated audio `{media_name}` has no readable speech artifact in lattice.lock.json"
            )),
            AudioMixError::SourceUnavailable { source, .. } | AudioMixError::Mix { source } => {
                *source
            }
            AudioMixError::EmptySource { media_name, .. } => Self::Ffmpeg {
                status: "empty-audio".into(),
                stderr: format!("audio source `{media_name}` decoded to no PCM frames"),
            },
            AudioMixError::PlayheadOutOfRange { .. } => Self::TimeOutOfRange,
        }
    }
}

/// Deterministic facts about a prepared mix, suitable for Studio diagnostics.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AudioMixReport {
    pub duration: Time,
    pub sample_rate: u32,
    pub channels: u16,
    pub frame_count: usize,
    pub window_count: usize,
    pub hold_window_count: usize,
    pub generated_window_count: usize,
    pub decoded_sources: Vec<String>,
}

/// Full interleaved PCM plus diagnostics for one flattened timeline.
#[derive(Clone, Debug, PartialEq)]
pub struct PreparedAudio {
    plan: AudioPlan,
    pcm: PcmBuffer,
    report: AudioMixReport,
}

impl PreparedAudio {
    pub fn plan(&self) -> &AudioPlan {
        &self.plan
    }

    pub fn pcm(&self) -> &PcmBuffer {
        &self.pcm
    }

    pub fn into_pcm(self) -> PcmBuffer {
        self.pcm
    }

    pub fn report(&self) -> &AudioMixReport {
        &self.report
    }

    pub fn into_parts(self) -> (AudioPlan, PcmBuffer, AudioMixReport) {
        (self.plan, self.pcm, self.report)
    }

    /// Borrow frame-aligned interleaved PCM from `playhead` through the mix end.
    ///
    /// `playhead == duration` is valid and returns an empty slice. Conversion to
    /// a frame uses the same floor rule as placement in [`crate::mix_plan`].
    pub fn samples_from(&self, playhead: Time) -> Result<&[f32], AudioMixError> {
        if playhead < Time::ZERO || playhead > self.report.duration {
            return Err(AudioMixError::PlayheadOutOfRange {
                playhead,
                duration: self.report.duration,
            });
        }
        let frame = time_to_frames(playhead, self.pcm.sample_rate).min(self.pcm.frame_count());
        let sample = frame
            .saturating_mul(usize::from(self.pcm.channels))
            .min(self.pcm.samples.len());
        Ok(&self.pcm.samples[sample..])
    }
}

/// Decode and mix timeline audio through the exact path consumed by export.
///
/// `Ok(None)` means the timeline has no audio windows. Once a window exists,
/// missing sources, stale generated-media locks, and decode failures are errors
/// rather than implicit silence.
pub fn mix_timeline_audio(
    timeline: &Timeline,
    options: &PreviewOptions,
    spec: MixSpec,
) -> Result<Option<PreparedAudio>, AudioMixError> {
    if spec.sample_rate == 0 || spec.channels == 0 {
        return Err(AudioMixError::InvalidMixSpec {
            sample_rate: spec.sample_rate,
            channels: spec.channels,
        });
    }

    let plan = audio_plan_from_timeline(timeline);
    if plan.windows.is_empty() {
        return Ok(None);
    }
    let sources = collect_audio_sources(
        &plan,
        &options.media_root,
        &options.output,
        options.lock.as_ref(),
        options.allow_fixtures,
        spec,
    )?;
    let pcm = mix_plan(&plan, &sources, spec).map_err(|source| AudioMixError::Mix {
        source: Box::new(source),
    })?;
    let mut decoded_sources = sources.into_keys().collect::<Vec<_>>();
    decoded_sources.sort();
    let report = AudioMixReport {
        duration: plan.duration,
        sample_rate: pcm.sample_rate,
        channels: pcm.channels,
        frame_count: pcm.frame_count(),
        window_count: plan.windows.len(),
        hold_window_count: plan.windows.iter().filter(|window| window.hold).count(),
        generated_window_count: plan
            .windows
            .iter()
            .filter(|window| window.generated)
            .count(),
        decoded_sources,
    };
    Ok(Some(PreparedAudio { plan, pcm, report }))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use lattice_core::{
        AssetIdentity, LockedAsset, MediaLocator, PlacementKind, ResolveLock, TimeMap, TimeSpan,
        TimelineClip, TimelineSource,
    };

    use super::*;
    use crate::backend::RendererRequest;

    fn options(root: PathBuf, lock: Option<ResolveLock>) -> PreviewOptions {
        PreviewOptions {
            output: root.join("monitor.raw"),
            media_root: root,
            lock,
            spec: crate::OutputSpec::preview(),
            renderer: RendererRequest::RequireCpu,
            allow_fixtures: false,
            font: None,
        }
    }

    fn timeline(clips: Vec<TimelineClip>) -> Timeline {
        Timeline {
            duration: Time::seconds(1),
            clips,
        }
    }

    fn generated_audio(gain_db: i32) -> TimelineClip {
        TimelineClip {
            id: "speech".into(),
            kind: PlacementKind::Audio,
            span: TimeSpan::new(Time::ZERO, Time::seconds(1)),
            source: Some(TimelineSource {
                media_name: "speech-line".into(),
                locator: MediaLocator::Generated {
                    generator: "speech".into(),
                    key: "line".into(),
                },
                time_map: TimeMap::identity(Time::ZERO, Time::seconds(1)),
            }),
            text: None,
            opacity: None,
            fade_in: None,
            fade_out: None,
            position: None,
            scale: None,
            gain_db: Some(gain_db),
        }
    }

    #[test]
    fn no_audio_windows_are_distinct_from_silence() {
        let root = std::env::temp_dir();
        let mixed = mix_timeline_audio(
            &timeline(Vec::new()),
            &options(root, None),
            MixSpec::PREVIEW,
        )
        .expect("no audio is valid");
        assert!(mixed.is_none());
    }

    #[test]
    fn unresolved_generated_audio_is_a_typed_error() {
        let root = std::env::temp_dir();
        let error = mix_timeline_audio(
            &timeline(vec![generated_audio(0)]),
            &options(root, None),
            MixSpec::PREVIEW,
        )
        .expect_err("speech requires a lock artifact");
        assert!(matches!(
            error,
            AudioMixError::MissingGeneratedAsset { ref media_name }
                if media_name == "speech-line"
        ));
    }

    #[test]
    fn playhead_slice_is_frame_aligned_and_range_checked() {
        let prepared = PreparedAudio {
            plan: AudioPlan {
                duration: Time::seconds(2),
                windows: Vec::new(),
            },
            pcm: PcmBuffer {
                sample_rate: 4,
                channels: 2,
                samples: (0_u8..16).map(f32::from).collect(),
            },
            report: AudioMixReport {
                duration: Time::seconds(2),
                sample_rate: 4,
                channels: 2,
                frame_count: 8,
                window_count: 0,
                hold_window_count: 0,
                generated_window_count: 0,
                decoded_sources: vec!["source".into()],
            },
        };
        assert_eq!(
            prepared.samples_from(Time::milliseconds(500)).unwrap(),
            &prepared.pcm.samples[4..]
        );
        assert!(prepared.samples_from(Time::seconds(2)).unwrap().is_empty());
        assert!(matches!(
            prepared.samples_from(Time::milliseconds(-1)),
            Err(AudioMixError::PlayheadOutOfRange { .. })
        ));
        assert!(matches!(
            prepared.samples_from(Time::seconds(3)),
            Err(AudioMixError::PlayheadOutOfRange { .. })
        ));
    }

    #[test]
    fn generated_lock_gain_and_report_share_export_mix() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "lattice-audio-monitor-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        let resolved = crate::generate_av_fixture(root.join("speech.mp4"), 1)
            .expect("generated speech fixture");
        let lock = ResolveLock {
            schema_version: 1,
            assets: vec![LockedAsset {
                id: "media:speech-line".into(),
                generator: Some("speech".into()),
                key: "line".into(),
                path: resolved.display().to_string(),
                identity: AssetIdentity::new("fixture"),
                duration: Some(Time::seconds(1)),
                provider: Some("test".into()),
                provider_version: Some("1".into()),
            }],
        };
        let spec = MixSpec {
            sample_rate: 8_000,
            channels: 1,
        };
        let unity = mix_timeline_audio(
            &timeline(vec![generated_audio(0)]),
            &options(root.clone(), Some(lock.clone())),
            spec,
        )
        .expect("unity mix")
        .expect("audio windows");
        let ducked = mix_timeline_audio(
            &timeline(vec![generated_audio(-15)]),
            &options(root.clone(), Some(lock)),
            spec,
        )
        .expect("ducked mix")
        .expect("audio windows");

        let unity_peak = unity
            .pcm()
            .samples
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        let ducked_peak = ducked
            .pcm()
            .samples
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        assert!(unity_peak > 0.0);
        assert!((ducked_peak / unity_peak - crate::mix::db_to_linear(-15)).abs() < 0.01);
        assert_eq!(ducked.report().generated_window_count, 1);
        assert_eq!(ducked.report().decoded_sources, ["speech-line"]);
        assert_eq!(ducked.report().frame_count, 8_000);
        assert_eq!(ducked.plan().windows[0].gain_db, -15);

        drop(ducked);
        drop(unity);
        let _ = std::fs::remove_dir_all(root);
    }
}

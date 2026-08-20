//! PCM mixer. Gain/placement/mix are Lattice semantics, not `FFmpeg` `volume`/`amix`.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::implicit_hasher,
    clippy::unnecessary_wraps
)]

use std::collections::HashMap;

use lattice_core::{AudioClip, AudioPlan, Time};

use crate::backend::PcmBuffer;
use crate::export::ExportError;

#[derive(Clone, Copy, Debug)]
pub struct MixSpec {
    pub sample_rate: u32,
    pub channels: u16,
}

impl MixSpec {
    pub const PREVIEW: Self = Self {
        sample_rate: 44_100,
        channels: 2,
    };
}

pub fn time_to_frames(time: Time, sample_rate: u32) -> usize {
    if time.den() == 0 {
        return 0;
    }
    let n = i128::from(time.num()) * i128::from(sample_rate);
    let d = i128::from(time.den());
    let q = n.div_euclid(d);
    usize::try_from(q.max(0)).unwrap_or(0)
}

pub fn db_to_linear(gain_db: i32) -> f32 {
    10f32.powf(gain_db as f32 / 20.0)
}

/// Mix already-decoded source buffers. Keys are `AssetRef.media_name`.
pub fn mix_plan(
    plan: &AudioPlan,
    sources: &HashMap<String, PcmBuffer>,
    spec: MixSpec,
) -> Result<PcmBuffer, ExportError> {
    let frames = time_to_frames(plan.duration, spec.sample_rate).max(1);
    let mut mix = PcmBuffer::silence(spec.sample_rate, spec.channels, frames);
    for window in &plan.windows {
        mix_window(&mut mix, window, sources, spec)?;
    }
    for sample in &mut mix.samples {
        *sample = sample.clamp(-1.0, 1.0);
    }
    Ok(mix)
}

fn mix_window(
    mix: &mut PcmBuffer,
    window: &AudioClip,
    sources: &HashMap<String, PcmBuffer>,
    spec: MixSpec,
) -> Result<(), ExportError> {
    if window.hold {
        return Ok(());
    }
    let dest_start = time_to_frames(window.span.start, spec.sample_rate);
    let dest_len = time_to_frames(window.span.duration, spec.sample_rate);
    if dest_len == 0 {
        return Ok(());
    }
    let gain = db_to_linear(window.gain_db);
    let channels = spec.channels as usize;
    let Some(asset) = window.asset.as_ref() else {
        return Ok(());
    };
    let Some(source) = sources.get(&asset.media_name) else {
        return Ok(());
    };
    let src_channels = source.channels.max(1) as usize;
    let src_start = time_to_frames(window.content_start, source.sample_rate);
    for i in 0..dest_len {
        let dest_frame = dest_start + i;
        if dest_frame >= mix.frame_count() {
            break;
        }
        let src_frame = src_start + i;
        for ch in 0..channels {
            let dest_i = dest_frame * channels + ch;
            let src_ch = ch.min(src_channels.saturating_sub(1));
            let sample = if src_frame < source.frame_count() {
                source.samples[src_frame * src_channels + src_ch]
            } else {
                0.0
            };
            mix.samples[dest_i] += sample * gain;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::{AssetRef, MediaLocator, TimeSpan};

    fn named_pcm(name: &str, samples: Vec<f32>) -> (String, PcmBuffer) {
        (
            name.into(),
            PcmBuffer {
                sample_rate: 8,
                channels: 1,
                samples,
            },
        )
    }

    fn clip(name: &str, start: i64, dur: i64, gain_db: i32, content: i64) -> AudioClip {
        AudioClip {
            span: TimeSpan::new(Time::seconds(start), Time::seconds(dur)),
            gain_db,
            generated: false,
            asset: Some(AssetRef {
                media_name: name.into(),
                locator: MediaLocator::File { path: name.into() },
            }),
            content_start: Time::seconds(content),
            hold: false,
        }
    }

    #[test]
    fn places_source_at_offset() {
        let mut sources = HashMap::new();
        sources.insert(
            named_pcm("a", vec![0.25, 0.5, 0.75, 1.0]).0,
            named_pcm("a", vec![0.25, 0.5, 0.75, 1.0]).1,
        );
        let plan = AudioPlan {
            duration: Time::seconds(2),
            windows: vec![clip("a", 1, 1, 0, 0)],
        };
        let mixed = mix_plan(
            &plan,
            &sources,
            MixSpec {
                sample_rate: 8,
                channels: 1,
            },
        )
        .unwrap();
        assert_eq!(mixed.frame_count(), 16);
        assert!(mixed.samples[..8].iter().all(|s| *s == 0.0));
        assert!((mixed.samples[8] - 0.25).abs() < f32::EPSILON);
        assert!((mixed.samples[9] - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn gain_minus_6db_halves() {
        let mut sources = HashMap::new();
        let (k, v) = named_pcm("a", vec![0.8; 8]);
        sources.insert(k, v);
        let plan = AudioPlan {
            duration: Time::seconds(1),
            windows: vec![clip("a", 0, 1, -6, 0)],
        };
        let mixed = mix_plan(
            &plan,
            &sources,
            MixSpec {
                sample_rate: 8,
                channels: 1,
            },
        )
        .unwrap();
        let expected = 0.8 * db_to_linear(-6);
        assert!(
            (mixed.samples[0] - expected).abs() < 0.02,
            "{}",
            mixed.samples[0]
        );
    }

    #[test]
    fn silence_plus_speech_mix() {
        let mut sources = HashMap::new();
        let (k, v) = named_pcm("speech", vec![0.4; 8]);
        sources.insert(k, v);
        let plan = AudioPlan {
            duration: Time::seconds(2),
            windows: vec![clip("speech", 1, 1, 0, 0)],
        };
        let mixed = mix_plan(
            &plan,
            &sources,
            MixSpec {
                sample_rate: 8,
                channels: 1,
            },
        )
        .unwrap();
        assert_eq!(mixed.frame_count(), 16);
        assert!(mixed.samples[..8].iter().all(|s| *s == 0.0));
        assert!(mixed.samples[8..].iter().all(|s| (*s - 0.4).abs() < 0.001));
    }

    #[test]
    fn duration_parity_with_video() {
        let plan = AudioPlan {
            duration: Time::from_decimal_seconds(11, 5, 1).unwrap(),
            windows: vec![],
        };
        let mixed = mix_plan(&plan, &HashMap::new(), MixSpec::PREVIEW).unwrap();
        let expected = time_to_frames(plan.duration, MixSpec::PREVIEW.sample_rate);
        assert_eq!(mixed.frame_count(), expected.max(1));
    }

    #[test]
    fn hold_contributes_silence() {
        let mut sources = HashMap::new();
        let (k, v) = named_pcm("a", vec![1.0; 8]);
        sources.insert(k, v);
        let mut window = clip("a", 0, 1, 0, 0);
        window.hold = true;
        let plan = AudioPlan {
            duration: Time::seconds(1),
            windows: vec![window],
        };
        let mixed = mix_plan(
            &plan,
            &sources,
            MixSpec {
                sample_rate: 8,
                channels: 1,
            },
        )
        .unwrap();
        assert!(mixed.samples.iter().all(|s| *s == 0.0));
    }
}

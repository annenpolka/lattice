use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use lattice_core::{MediaLocator, ResolveLock, Time, TimeError, Timeline, TimelineError};
use serde::Serialize;
use thiserror::Error;

use crate::fixture::{DEFAULT_SOURCE_DURATION_SECS, FixtureError, generate_test_source};
use crate::plan::{RenderPlan, plan_from_timeline};
use crate::probe::{ProbeError, probe_duration};
use crate::{PREVIEW_HEIGHT, PREVIEW_WIDTH};

#[derive(Debug, Error)]
pub enum ExportError {
    #[error(transparent)]
    Timeline(#[from] TimelineError),
    #[error(transparent)]
    Fixture(#[from] FixtureError),
    #[error(transparent)]
    Probe(#[from] ProbeError),
    #[error("failed to run ffmpeg: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("ffmpeg export failed (status {status}): {stderr}")]
    Ffmpeg { status: String, stderr: String },
    #[error("video clip has no media source")]
    MissingSource,
    #[error(transparent)]
    Time(#[from] TimeError),
}

#[derive(Clone, Debug)]
pub struct PreviewOptions {
    pub output: PathBuf,
    pub media_root: PathBuf,
    pub lock: Option<ResolveLock>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ExportReport {
    pub output: PathBuf,
    pub duration: Time,
    pub plan: PlanSummary,
}

#[derive(Clone, Debug, Serialize)]
pub struct PlanSummary {
    pub hold_segments: usize,
    pub overlays: usize,
}

pub fn extract_frame(video: &Path, at: Time, ppm: &Path) -> Result<PathBuf, ExportError> {
    if let Some(parent) = ppm.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let output = Command::new(ffmpeg_bin())
        .args(["-y", "-ss", &ffmpeg_seconds(at), "-i"])
        .arg(video)
        .args(["-frames:v", "1"])
        .arg(ppm)
        .output()?;
    if !output.status.success() {
        return Err(ExportError::Ffmpeg {
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(ppm.to_path_buf())
}

pub fn ffmpeg_bin() -> PathBuf {
    tool_bin("LATTICE_FFMPEG", "ffmpeg")
}

pub fn ffprobe_bin() -> PathBuf {
    tool_bin("LATTICE_FFPROBE", "ffprobe")
}

fn tool_bin(env_key: &str, name: &str) -> PathBuf {
    if let Some(path) = std::env::var_os(env_key) {
        return PathBuf::from(path);
    }
    let exe = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    if let Ok(home) = std::env::var("USERPROFILE") {
        let scoop = PathBuf::from(home).join("scoop").join("shims").join(&exe);
        if scoop.is_file() {
            return scoop;
        }
    }
    PathBuf::from(name)
}

/// Encode a flattened timeline with `FFmpeg`. Missing source files get a generated test clip.
pub fn export_preview(
    timeline: &Timeline,
    options: &PreviewOptions,
) -> Result<ExportReport, ExportError> {
    let plan = plan_from_timeline(timeline)?;
    let video = timeline
        .video_clips()
        .next()
        .ok_or(TimelineError::NoVideo)?;
    let source = video.source.as_ref().ok_or(ExportError::MissingSource)?;
    let input = resolve_or_generate_source(&source.locator, &options.media_root, &options.output)?;
    let speech = locked_speech_path(options.lock.as_ref(), &plan);
    let filter = filter_complex(&plan, speech.is_some())?;
    if let Some(parent) = options.output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let fps = format!("{}/{}", plan.fps_num, plan.fps_den);
    let mut command = Command::new(ffmpeg_bin());
    command.args(["-y", "-i"]).arg(&input);
    if let Some(speech) = &speech {
        command.args(["-i"]).arg(speech);
    }
    command.args(["-filter_complex", &filter, "-map", "[outv]"]);
    if speech.is_some() {
        command.args(["-map", "[outa]"]);
    } else {
        command.arg("-an");
    }
    command
        .args([
            "-r",
            &fps,
            "-t",
            &ffmpeg_seconds(plan.duration),
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&options.output);
    let output = command.output()?;
    if !output.status.success() {
        return Err(ExportError::Ffmpeg {
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let duration = probe_duration(&options.output)?;
    Ok(ExportReport {
        output: options.output.clone(),
        duration,
        plan: PlanSummary {
            hold_segments: plan.segments.iter().filter(|segment| segment.hold).count(),
            overlays: plan.overlays.len(),
        },
    })
}

fn resolve_or_generate_source(
    locator: &MediaLocator,
    media_root: &Path,
    output: &Path,
) -> Result<PathBuf, ExportError> {
    let MediaLocator::File { path } = locator else {
        return generate_beside_output(output).map_err(Into::into);
    };
    let candidate = Path::new(path);
    let resolved = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        media_root.join(candidate)
    };
    if resolved.is_file() {
        return Ok(resolved);
    }
    generate_beside_output(output).map_err(Into::into)
}

fn generate_beside_output(output: &Path) -> Result<PathBuf, FixtureError> {
    let dir = output.parent().unwrap_or_else(|| Path::new("."));
    generate_test_source(
        dir.join("_lattice_source.mp4"),
        DEFAULT_SOURCE_DURATION_SECS,
    )
}

fn locked_speech_path(lock: Option<&ResolveLock>, plan: &RenderPlan) -> Option<PathBuf> {
    let lock = lock?;
    let window = plan.audio.iter().find(|audio| audio.generated)?;
    let media_name = window.media_name.as_deref()?;
    lock.assets
        .iter()
        .find(|asset| {
            asset.generator.as_deref() == Some("speech")
                && (asset.id.contains(media_name) || asset.path.contains(media_name))
        })
        .or_else(|| {
            lock.assets
                .iter()
                .find(|asset| asset.generator.as_deref() == Some("speech"))
        })
        .map(|asset| PathBuf::from(&asset.path))
        .filter(|path| path.is_file())
}

fn filter_complex(plan: &RenderPlan, with_speech: bool) -> Result<String, ExportError> {
    let n = plan.segments.len();
    if n == 0 {
        return Err(TimelineError::NoVideo.into());
    }
    let fps = format!("{}/{}", plan.fps_num, plan.fps_den);
    let one_frame = ffmpeg_seconds(Time::from_frames(1, plan.fps_num, plan.fps_den)?);
    let mut parts = Vec::new();
    parts.push(format!("[0:v]split={n}{}", split_labels(n, "s")));
    let mut concat_in = String::new();
    for (i, segment) in plan.segments.iter().enumerate() {
        let start = ffmpeg_seconds(segment.content_start);
        if segment.hold {
            let frames = segment
                .local
                .duration
                .exact_frame_count(plan.fps_num, plan.fps_den)?;
            let loops = frames.saturating_sub(1);
            parts.push(format!(
                "[s{i}]trim=start={start}:duration={one_frame},setpts=PTS-STARTPTS,loop=loop={loops}:size=1:start=0,fps={fps}[v{i}]"
            ));
        } else {
            let end = ffmpeg_seconds(segment.content_start + segment.local.duration);
            parts.push(format!(
                "[s{i}]trim=start={start}:end={end},setpts=PTS-STARTPTS,fps={fps}[v{i}]"
            ));
        }
        write!(&mut concat_in, "[v{i}]").expect("label");
    }
    let after_concat = if plan.fade_in.is_some() {
        "fadedin"
    } else if plan.overlays.is_empty() {
        "rated"
    } else {
        "base"
    };
    parts.push(format!("{concat_in}concat=n={n}:v=1:a=0[{after_concat}]"));
    let mut last = after_concat.to_string();
    if let Some(fade) = plan.fade_in {
        let next = if plan.overlays.is_empty() {
            "rated".to_string()
        } else {
            "base".to_string()
        };
        parts.push(format!(
            "[{last}]fade=t=in:st=0:d={}[{next}]",
            ffmpeg_seconds(fade)
        ));
        last = next;
    }
    for (i, overlay) in plan.overlays.iter().enumerate() {
        let start = ffmpeg_seconds(overlay.span.start);
        let end = ffmpeg_seconds(overlay.span.end());
        let enable = format!("between(t\\,{start}\\,{end})");
        let next = if i + 1 == plan.overlays.len() {
            "rated".to_string()
        } else {
            format!("ov{i}")
        };
        let alpha = overlay
            .opacity
            .map_or(1.0, |value| f64::from(value) / 100.0);
        let (y, color) = if overlay.callout {
            (0, format!("cyan@{alpha}"))
        } else {
            (PREVIEW_HEIGHT.saturating_sub(8), format!("yellow@{alpha}"))
        };
        parts.push(format!(
            "[{last}]drawbox=x=0:y={y}:w={PREVIEW_WIDTH}:h=8:color={color}:t=fill:enable='{enable}'[{next}]"
        ));
        last = next;
    }
    // Pin fps in-graph. Some ffmpeg builds (johnvansickle) otherwise encode at 25fps
    // and probe reports 11.48s for an 11.5s / 10fps timeline.
    parts.push(format!("[{last}]fps={fps}[outv]"));
    if with_speech {
        let speech = plan
            .audio
            .iter()
            .find(|audio| audio.generated)
            .ok_or_else(|| ExportError::Ffmpeg {
                status: "plan".into(),
                stderr: "speech requested without an audio window".into(),
            })?;
        let delay_ms = {
            let t = speech.span.start;
            if t.den() == 0 {
                0
            } else {
                i64::try_from(i128::from(t.num()) * 1000 / i128::from(t.den()))
                    .unwrap_or(0)
                    .max(0)
            }
        };
        parts.push(format!(
            "[1:a]adelay={delay_ms}|{delay_ms},apad,atrim=0:{}[outa]",
            ffmpeg_seconds(plan.duration)
        ));
    }
    Ok(parts.join(";"))
}

fn split_labels(n: usize, prefix: &str) -> String {
    let mut out = String::new();
    for i in 0..n {
        write!(&mut out, "[{prefix}{i}]").expect("label");
    }
    out
}

fn ffmpeg_seconds(time: Time) -> String {
    if time.den() == 0 {
        return "0".into();
    }
    if time.num() % time.den() == 0 {
        return (time.num() / time.den()).to_string();
    }
    let scaled = (i128::from(time.num()) * 1_000_000_000) / i128::from(time.den());
    let whole = scaled / 1_000_000_000;
    let frac = (scaled % 1_000_000_000).unsigned_abs();
    let mut frac_text = format!("{frac:09}");
    while frac_text.ends_with('0') && frac_text.len() > 1 {
        frac_text.pop();
    }
    format!("{whole}.{frac_text}")
}

#[cfg(test)]
mod tests {
    use lattice_core::{Time, TimeSpan};

    use super::ffmpeg_seconds;
    use crate::plan::{PlanSegment, RenderPlan};

    #[test]
    fn formats_hold_duration() {
        let t = Time::from_decimal_seconds(1, 5, 1).unwrap();
        assert_eq!(ffmpeg_seconds(t), "1.5");
        assert_eq!(
            ffmpeg_seconds(Time::from_decimal_seconds(5, 2, 1).unwrap()),
            "5.2"
        );
    }

    #[test]
    fn filter_complex_pins_preview_fps() {
        let plan = RenderPlan {
            duration: Time::from_decimal_seconds(11, 5, 1).unwrap(),
            fps_num: 10,
            fps_den: 1,
            segments: vec![PlanSegment {
                local: TimeSpan::new(Time::ZERO, Time::seconds(1)),
                content_start: Time::ZERO,
                hold: false,
            }],
            overlays: vec![],
            fade_in: None,
            audio: vec![],
        };
        let filter = super::filter_complex(&plan, false).unwrap();
        assert!(
            filter.contains("fps=10/1"),
            "expected pinned fps in {filter}"
        );
        assert!(filter.contains("[outv]"), "{filter}");
    }
}

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use lattice_core::{MediaLocator, Time, TimeError, Timeline, TimelineError};
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
    let filter = filter_complex(&plan)?;
    if let Some(parent) = options.output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let output = Command::new(ffmpeg_bin())
        .args(["-y", "-i"])
        .arg(&input)
        .args([
            "-filter_complex",
            &filter,
            "-map",
            "[outv]",
            "-an",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&options.output)
        .output()?;
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

fn filter_complex(plan: &RenderPlan) -> Result<String, ExportError> {
    let n = plan.segments.len();
    if n == 0 {
        return Err(TimelineError::NoVideo.into());
    }
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
                "[s{i}]trim=start={start}:duration=0.1,setpts=PTS-STARTPTS,loop=loop={loops}:size=1:start=0,setpts=N/{fps}/TB[v{i}]",
                fps = plan.fps_num
            ));
        } else {
            let end = ffmpeg_seconds(segment.content_start + segment.local.duration);
            parts.push(format!(
                "[s{i}]trim=start={start}:end={end},setpts=PTS-STARTPTS[v{i}]"
            ));
        }
        write!(&mut concat_in, "[v{i}]").expect("label");
    }
    let after_concat = if plan.overlays.is_empty() {
        "outv"
    } else {
        "base"
    };
    parts.push(format!("{concat_in}concat=n={n}:v=1:a=0[{after_concat}]"));
    let mut last = after_concat.to_string();
    for (i, overlay) in plan.overlays.iter().enumerate() {
        let start = ffmpeg_seconds(overlay.span.start);
        let end = ffmpeg_seconds(overlay.span.end());
        let enable = format!("between(t\\,{start}\\,{end})");
        let next = if i + 1 == plan.overlays.len() {
            "outv".to_string()
        } else {
            format!("ov{i}")
        };
        let bar_y = PREVIEW_HEIGHT.saturating_sub(8);
        parts.push(format!(
            "[{last}]drawbox=x=0:y={bar_y}:w={PREVIEW_WIDTH}:h=8:color=yellow:t=fill:enable='{enable}'[{next}]"
        ));
        last = next;
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
    use lattice_core::Time;

    use super::ffmpeg_seconds;

    #[test]
    fn formats_hold_duration() {
        let t = Time::from_decimal_seconds(1, 5, 1).unwrap();
        assert_eq!(ffmpeg_seconds(t), "1.5");
        assert_eq!(
            ffmpeg_seconds(Time::from_decimal_seconds(5, 2, 1).unwrap()),
            "5.2"
        );
    }
}

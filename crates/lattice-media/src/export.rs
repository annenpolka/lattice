use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use lattice_core::{MediaLocator, ResolveLock, Time, TimeError, Timeline, TimelineError};
use serde::Serialize;
use thiserror::Error;

use crate::fixture::{DEFAULT_SOURCE_DURATION_SECS, FixtureError, generate_test_source};
use crate::plan::{AudioWindow, OverlayWindow, RenderPlan, plan_from_timeline};
use crate::probe::{ProbeError, find_font, probe_media};

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
    #[error("referenced media is missing: {0}")]
    MissingMedia(String),
    #[error("no usable font for title/callout text (set LATTICE_FONT to a .ttf)")]
    MissingFont,
    #[error("timeline time is outside the video")]
    TimeOutOfRange,
    #[error("time map: {0}")]
    Map(String),
    #[error(transparent)]
    Time(#[from] TimeError),
}

#[derive(Clone, Debug)]
pub struct PreviewOptions {
    pub output: PathBuf,
    pub media_root: PathBuf,
    pub lock: Option<ResolveLock>,
    /// When true, missing files may be replaced with a generated testsrc fixture.
    /// Production render must leave this false.
    pub allow_fixtures: bool,
    /// Optional font override for drawtext. Production uses [`find_font`].
    pub font: Option<PathBuf>,
}

impl PreviewOptions {
    pub fn new(output: PathBuf, media_root: PathBuf) -> Self {
        Self {
            output,
            media_root,
            lock: None,
            allow_fixtures: false,
            font: None,
        }
    }
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
    run_ffmpeg_extract_frame(video, at, ppm, None)
}

pub(crate) fn run_ffmpeg_extract_frame(
    video: &Path,
    at: Time,
    ppm: &Path,
    scale: Option<(u32, u32)>,
) -> Result<PathBuf, ExportError> {
    if let Some(parent) = ppm.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut command = Command::new(ffmpeg_bin());
    command
        .args(["-y", "-ss", &ffmpeg_seconds(at), "-i"])
        .arg(video)
        .args(["-frames:v", "1"]);
    if let Some((width, height)) = scale {
        command.args(["-vf", &format!("scale={width}:{height}")]);
    }
    command.arg(ppm);
    let output = command.output()?;
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

/// Encode a flattened timeline with `FFmpeg`.
///
/// Missing user media is an error unless [`PreviewOptions::allow_fixtures`] is set.
pub fn export_preview(
    timeline: &Timeline,
    options: &PreviewOptions,
) -> Result<ExportReport, ExportError> {
    let plan = plan_from_timeline(timeline)?;
    if plan.segments.is_empty() {
        return Err(TimelineError::NoVideo.into());
    }
    let resolved = resolve_plan(&plan, options)?;
    let filter = filter_complex(&plan, &resolved)?;
    if let Some(parent) = options.output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let fps = format!("{}/{}", plan.fps_num, plan.fps_den);
    let mut command = Command::new(ffmpeg_bin());
    command.arg("-y");
    if let Some(dir) = options.output.parent() {
        command.current_dir(dir);
    }
    for input in &resolved.inputs {
        command.args(["-i"]).arg(input);
    }
    command.args(["-filter_complex", &filter, "-map", "[outv]"]);
    if resolved.has_audio {
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
    let duration = crate::probe::probe_duration(&options.output)?;
    Ok(ExportReport {
        output: options.output.clone(),
        duration,
        plan: PlanSummary {
            hold_segments: plan.segments.iter().filter(|segment| segment.hold).count(),
            overlays: plan.overlays.len(),
        },
    })
}

struct ResolvedPlan {
    inputs: Vec<PathBuf>,
    segment_inputs: Vec<usize>,
    audio: Vec<(AudioWindow, AudioInput)>,
    has_audio: bool,
    font: Option<PathBuf>,
}

#[derive(Clone, Debug)]
enum AudioInput {
    File { index: usize },
    Silence,
}

fn resolve_plan(plan: &RenderPlan, options: &PreviewOptions) -> Result<ResolvedPlan, ExportError> {
    let mut inputs: Vec<PathBuf> = Vec::new();
    let mut segment_inputs = Vec::new();
    for segment in &plan.segments {
        let path = resolve_media_path(
            &segment.locator,
            &options.media_root,
            &options.output,
            options.allow_fixtures,
        )?;
        let index = push_unique(&mut inputs, path);
        segment_inputs.push(index);
    }

    let needs_text = plan
        .overlays
        .iter()
        .any(|overlay| overlay.text.as_ref().is_some_and(|text| !text.is_empty()));
    let font = if needs_text {
        let found = options
            .font
            .clone()
            .or_else(find_font)
            .ok_or(ExportError::MissingFont)?;
        // Copy beside the output so the filtergraph can use a colon-free name.
        let local = options
            .output
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("_lattice_overlay.ttf");
        if found != local {
            std::fs::copy(&found, &local)?;
        }
        Some(local)
    } else {
        None
    };

    let mut audio = Vec::new();
    for window in &plan.audio {
        if window.generated {
            if let Some(path) = locked_generated_path(options.lock.as_ref(), window) {
                let index = push_unique(&mut inputs, path);
                audio.push((window.clone(), AudioInput::File { index }));
            }
            continue;
        }
        let Some(locator) = &window.locator else {
            audio.push((window.clone(), AudioInput::Silence));
            continue;
        };
        let path = match resolve_media_path(
            locator,
            &options.media_root,
            &options.output,
            options.allow_fixtures,
        ) {
            Ok(path) => path,
            Err(ExportError::MissingMedia(_)) if options.allow_fixtures => {
                audio.push((window.clone(), AudioInput::Silence));
                continue;
            }
            Err(err) => return Err(err),
        };
        let has_audio = probe_media(&path).is_ok_and(|info| info.has_audio);
        if window.hold || !has_audio {
            audio.push((window.clone(), AudioInput::Silence));
        } else {
            let index = push_unique(&mut inputs, path);
            audio.push((window.clone(), AudioInput::File { index }));
        }
    }

    let has_audio = !audio.is_empty();
    Ok(ResolvedPlan {
        inputs,
        segment_inputs,
        audio,
        has_audio,
        font,
    })
}

fn strip_verbatim(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    let trimmed = raw
        .strip_prefix(r"\\?\UNC\")
        .map(|rest| format!(r"\\{rest}"))
        .or_else(|| raw.strip_prefix(r"\\?\").map(str::to_string))
        .unwrap_or_else(|| raw.into_owned());
    PathBuf::from(trimmed)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn push_unique(inputs: &mut Vec<PathBuf>, path: PathBuf) -> usize {
    if let Some((index, _)) = inputs
        .iter()
        .enumerate()
        .find(|(_, existing)| *existing == &path)
    {
        return index;
    }
    inputs.push(path);
    inputs.len() - 1
}

pub(crate) fn resolve_media_path(
    locator: &MediaLocator,
    media_root: &Path,
    output: &Path,
    allow_fixtures: bool,
) -> Result<PathBuf, ExportError> {
    match locator {
        MediaLocator::File { path } => {
            let candidate = Path::new(path);
            let media_root = strip_verbatim(media_root);
            let resolved = if candidate.is_absolute() {
                strip_verbatim(candidate)
            } else {
                media_root.join(candidate)
            };
            if let Ok(canonical) = std::fs::canonicalize(&resolved)
                && canonical.is_file()
            {
                return Ok(canonical);
            }
            let resolved = normalize_path(&resolved);
            if resolved.is_file() {
                return Ok(std::fs::canonicalize(&resolved).unwrap_or(resolved));
            }
            if allow_fixtures {
                return generate_beside_output(output).map_err(Into::into);
            }
            Err(ExportError::MissingMedia(resolved.display().to_string()))
        }
        MediaLocator::Generated { generator, key } => Err(ExportError::MissingMedia(format!(
            "generated `{generator}:{key}` is not a file locator"
        ))),
        MediaLocator::Url { url } => Err(ExportError::MissingMedia(url.clone())),
    }
}

fn generate_beside_output(output: &Path) -> Result<PathBuf, FixtureError> {
    let dir = output.parent().unwrap_or_else(|| Path::new("."));
    generate_test_source(
        dir.join("_lattice_source.mp4"),
        DEFAULT_SOURCE_DURATION_SECS,
    )
}

fn locked_generated_path(lock: Option<&ResolveLock>, window: &AudioWindow) -> Option<PathBuf> {
    let lock = lock?;
    let media_name = window.media_name.as_deref();
    lock.assets
        .iter()
        .find(|asset| {
            asset.generator.as_deref() == Some("speech")
                && media_name
                    .is_some_and(|name| asset.id.contains(name) || asset.path.contains(name))
        })
        .or_else(|| {
            lock.assets
                .iter()
                .find(|asset| asset.generator.as_deref() == Some("speech"))
        })
        .map(|asset| PathBuf::from(&asset.path))
        .filter(|path| path.is_file())
}

#[allow(clippy::too_many_lines)]
fn filter_complex(plan: &RenderPlan, resolved: &ResolvedPlan) -> Result<String, ExportError> {
    let n = plan.segments.len();
    if n == 0 {
        return Err(TimelineError::NoVideo.into());
    }
    let fps = format!("{}/{}", plan.fps_num, plan.fps_den);
    let one_frame = ffmpeg_seconds(Time::from_frames(1, plan.fps_num, plan.fps_den)?);
    let mut parts = Vec::new();

    // Split each unique video input as needed, then trim each segment.
    let mut splits: Vec<Vec<usize>> = vec![Vec::new(); resolved.inputs.len()];
    for (seg_i, input_i) in resolved.segment_inputs.iter().enumerate() {
        splits[*input_i].push(seg_i);
    }
    for (input_i, segs) in splits.iter().enumerate() {
        if segs.is_empty() {
            continue;
        }
        if segs.len() == 1 {
            parts.push(format!("[{input_i}:v]null[s{}]", segs[0]));
        } else {
            let mut labels = String::new();
            for i in segs {
                write!(&mut labels, "[s{i}]").expect("label");
            }
            parts.push(format!("[{input_i}:v]split={}{labels}", segs.len()));
        }
    }

    let mut concat_in = String::new();
    for (i, segment) in plan.segments.iter().enumerate() {
        let start = ffmpeg_seconds(segment.content_start);
        let mut chain = if segment.hold {
            // Convert to output fps *before* looping. Looping a 60fps frame and
            // then resampling shortens the hold (video ends before audio).
            let frames = output_frame_count(segment.local.duration, plan.fps_num, plan.fps_den)?;
            let loops = frames.saturating_sub(1);
            format!(
                "[s{i}]trim=start={start}:duration={one_frame},setpts=PTS-STARTPTS,fps={fps},loop=loop={loops}:size=1:start=0,setpts=N*{den}/{num}/TB",
                den = plan.fps_den,
                num = plan.fps_num,
            )
        } else {
            let end = ffmpeg_seconds(segment.content_start + segment.local.duration);
            format!("[s{i}]trim=start={start}:end={end},setpts=PTS-STARTPTS,fps={fps}")
        };
        if let Some(fade) = segment.fade_in {
            write!(&mut chain, ",fade=t=in:st=0:d={}", ffmpeg_seconds(fade)).expect("fade");
        }
        write!(&mut chain, "[v{i}]").expect("label");
        parts.push(chain);
        write!(&mut concat_in, "[v{i}]").expect("label");
    }
    let after_concat = if plan.overlays.is_empty() {
        "rated"
    } else {
        "base"
    };
    parts.push(format!("{concat_in}concat=n={n}:v=1:a=0[{after_concat}]"));
    let mut last = after_concat.to_string();
    for (i, overlay) in plan.overlays.iter().enumerate() {
        let next = if i + 1 == plan.overlays.len() {
            "rated".to_string()
        } else {
            format!("ov{i}")
        };
        parts.push(overlay_filter(
            &last,
            &next,
            overlay,
            resolved.font.as_deref(),
        )?);
        last = next;
    }
    parts.push(format!(
        "[{last}]fps={fps},tpad=stop_mode=clone:stop_duration=1[outv]"
    ));

    if resolved.has_audio {
        parts.extend(audio_filters(plan, resolved));
    }
    Ok(parts.join(";"))
}

fn overlay_filter(
    last: &str,
    next: &str,
    overlay: &OverlayWindow,
    font: Option<&Path>,
) -> Result<String, ExportError> {
    let start = ffmpeg_seconds(overlay.span.start);
    let end = ffmpeg_seconds(overlay.span.end());
    let enable = format!("between(t\\,{start}\\,{end})");
    let alpha = overlay
        .opacity
        .map_or(1.0, |value| f64::from(value) / 100.0);
    let text = overlay.text.as_deref().unwrap_or("");
    if text.is_empty() {
        let (y, color) = if overlay.callout {
            (0, format!("cyan@{alpha}"))
        } else {
            (PREVIEW_HEIGHT.saturating_sub(8), format!("yellow@{alpha}"))
        };
        return Ok(format!(
            "[{last}]drawbox=x=0:y={y}:w={PREVIEW_WIDTH}:h=8:color={color}:t=fill:enable='{enable}'[{next}]"
        ));
    }
    let font = font.ok_or(ExportError::MissingFont)?;
    let fontfile = ffmpeg_fontfile(font);
    let escaped = escape_drawtext(text);
    let fontsize = if overlay.callout {
        "if(lt(h\\,400)\\,18\\,h/20)"
    } else {
        "if(lt(h\\,400)\\,22\\,h/16)"
    };
    let y = if overlay.callout {
        "if(lt(h\\,400)\\,12\\,h/20)"
    } else {
        "if(lt(h\\,400)\\,h-th-12\\,h-th-h/18)"
    };
    let box_color = if overlay.callout {
        format!("cyan@{:.2}", (alpha * 0.45).min(1.0))
    } else {
        format!("black@{:.2}", (alpha * 0.55).min(1.0))
    };
    let border = if overlay.callout { 8 } else { 16 };
    Ok(format!(
        "[{last}]drawtext=fontfile={fontfile}:text='{escaped}':x=(w-text_w)/2:y={y}:fontsize={fontsize}:fontcolor=white@{alpha}:box=1:boxcolor={box_color}:boxborderw={border}:enable='{enable}'[{next}]"
    ))
}

fn audio_filters(plan: &RenderPlan, resolved: &ResolvedPlan) -> Vec<String> {
    let mut parts = Vec::new();
    // Reuse each file's audio as many times as we have windows.
    let mut uses: Vec<Vec<usize>> = vec![Vec::new(); resolved.inputs.len()];
    for (i, (_, input)) in resolved.audio.iter().enumerate() {
        if let AudioInput::File { index } = input {
            uses[*index].push(i);
        }
    }
    for (input_i, windows) in uses.iter().enumerate() {
        if windows.is_empty() {
            continue;
        }
        if windows.len() == 1 {
            parts.push(format!("[{input_i}:a]anull[as{}]", windows[0]));
        } else {
            let mut labels = String::new();
            for i in windows {
                write!(&mut labels, "[as{i}]").expect("label");
            }
            parts.push(format!("[{input_i}:a]asplit={}{labels}", windows.len()));
        }
    }

    let mut concat_in = String::new();
    let mut source_count = 0usize;
    let mut speech_pads = Vec::new();
    for (i, (window, input)) in resolved.audio.iter().enumerate() {
        let dur = ffmpeg_seconds(window.span.duration);
        if window.generated {
            let delay_ms = delay_ms(window.span.start);
            match input {
                AudioInput::File { .. } => {
                    let volume = window
                        .gain_db
                        .map_or_else(|| "0dB".into(), |db| format!("{db}dB"));
                    parts.push(format!(
                        "[as{i}]atrim=0:{dur},asetpts=PTS-STARTPTS,volume={volume},aformat=sample_fmts=fltp:sample_rates=44100:channel_layouts=stereo,adelay={delay_ms}|{delay_ms},apad,atrim=0:{}[sp{i}]",
                        ffmpeg_seconds(plan.duration)
                    ));
                    speech_pads.push(format!("[sp{i}]"));
                }
                AudioInput::Silence => {}
            }
            continue;
        }
        match input {
            AudioInput::Silence => {
                parts.push(format!(
                    "anullsrc=r=44100:cl=stereo,atrim=0:{dur},asetpts=PTS-STARTPTS,aformat=sample_fmts=fltp:sample_rates=44100:channel_layouts=stereo[ac{i}]"
                ));
            }
            AudioInput::File { .. } => {
                let start = ffmpeg_seconds(window.content_start);
                let end = ffmpeg_seconds(window.content_start + window.span.duration);
                let volume = window
                    .gain_db
                    .map_or_else(|| "0dB".into(), |db| format!("{db}dB"));
                parts.push(format!(
                    "[as{i}]atrim=start={start}:end={end},asetpts=PTS-STARTPTS,volume={volume},aformat=sample_fmts=fltp:sample_rates=44100:channel_layouts=stereo,atrim=0:{dur},asetpts=PTS-STARTPTS[ac{i}]"
                ));
            }
        }
        write!(&mut concat_in, "[ac{i}]").expect("label");
        source_count += 1;
    }
    let plan_dur = ffmpeg_seconds(plan.duration);
    if source_count == 0 {
        parts.push(format!(
            "anullsrc=r=44100:cl=stereo,atrim=0:{plan_dur},aformat=sample_fmts=fltp:sample_rates=44100:channel_layouts=stereo[srca]"
        ));
    } else if source_count == 1 {
        parts.push(format!("{concat_in}apad,atrim=0:{plan_dur}[srca]"));
    } else {
        parts.push(format!(
            "{concat_in}concat=n={source_count}:v=0:a=1,apad,atrim=0:{plan_dur}[srca]"
        ));
    }
    if speech_pads.is_empty() {
        parts.push("[srca]anull[outa]".into());
    } else {
        let n = speech_pads.len() + 1;
        parts.push(format!(
            "[srca]{}amix=inputs={n}:duration=first:dropout_transition=0:normalize=0,atrim=0:{plan_dur}[outa]",
            speech_pads.join("")
        ));
    }
    parts
}

fn output_frame_count(duration: Time, fps_num: i64, fps_den: i64) -> Result<u64, ExportError> {
    if let Ok(frames) = duration.exact_frame_count(fps_num, fps_den) {
        return Ok(frames.max(1));
    }
    let n = i128::from(duration.num())
        .checked_mul(i128::from(fps_num))
        .ok_or(ExportError::TimeOutOfRange)?;
    let d = i128::from(duration.den())
        .checked_mul(i128::from(fps_den))
        .ok_or(ExportError::TimeOutOfRange)?;
    if d == 0 {
        return Err(ExportError::TimeOutOfRange);
    }
    let frames = (n + d / 2) / d;
    u64::try_from(frames.max(1)).map_err(|_| ExportError::TimeOutOfRange)
}

fn delay_ms(time: Time) -> i64 {
    if time.den() == 0 {
        0
    } else {
        i64::try_from(i128::from(time.num()) * 1000 / i128::from(time.den()))
            .unwrap_or(0)
            .max(0)
    }
}

fn escape_drawtext(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "\\'")
        .replace('%', "%%")
}

fn ffmpeg_fontfile(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("font.ttf")
        .replace(':', "\\:")
        .replace('\\', "/")
}

pub(crate) fn ffmpeg_seconds(time: Time) -> String {
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
    use std::path::PathBuf;

    use lattice_core::{MediaLocator, Time, TimeSpan};

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
    fn resolves_parent_relative_file_locator() {
        let dir = std::env::temp_dir().join("lattice-rel-loc");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("proj")).unwrap();
        std::fs::write(dir.join("gameplay.mp4"), b"not-a-video").unwrap();
        let path = super::resolve_media_path(
            &MediaLocator::File {
                path: "../gameplay.mp4".into(),
            },
            &dir.join("proj"),
            &dir.join("out.mp4"),
            false,
        )
        .expect("parent-relative locator");
        assert!(
            path.ends_with("gameplay.mp4") && path.is_file(),
            "{}",
            path.display()
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
                media_name: "game".into(),
                locator: MediaLocator::File {
                    path: "capture.mp4".into(),
                },
                fade_in: None,
            }],
            overlays: vec![],
            fade_in: None,
            audio: vec![],
        };
        let resolved = super::ResolvedPlan {
            inputs: vec![PathBuf::from("capture.mp4")],
            segment_inputs: vec![0],
            audio: vec![],
            has_audio: false,
            font: None,
        };
        let filter = super::filter_complex(&plan, &resolved).unwrap();
        assert!(
            filter.contains("fps=10/1"),
            "expected pinned fps in {filter}"
        );
        assert!(filter.contains("[outv]"), "{filter}");
    }
}

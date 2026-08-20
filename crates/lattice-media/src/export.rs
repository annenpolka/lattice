use std::path::{Path, PathBuf};
use std::process::Command;

use lattice_core::{MediaLocator, ResolveLock, Time, TimeError, Timeline, TimelineError};
use serde::Serialize;
use thiserror::Error;

use crate::backend::{
    OutputSpec, RendererInitError, RendererRenderError, RendererRequest, RendererSelection,
};
use crate::fixture::{DEFAULT_SOURCE_DURATION_SECS, FixtureError, generate_av_fixture};
#[cfg(test)]
use crate::plan::AudioWindow;
use crate::probe::ProbeError;

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
    #[error("font lock is stale: {0}")]
    StaleFont(String),
    #[error("font is missing glyphs for {0}")]
    MissingGlyph(String),
    #[error("timeline time is outside the video")]
    TimeOutOfRange,
    #[error("time map: {0}")]
    Map(String),
    #[error(transparent)]
    Time(#[from] TimeError),
    #[error(transparent)]
    Renderer(#[from] RendererInitError),
    #[error(transparent)]
    RendererRender(#[from] RendererRenderError),
    #[error("invalid output spec `{field}={value}`: {reason}")]
    InvalidOutputSpec {
        field: &'static str,
        value: String,
        reason: &'static str,
    },
}

#[derive(Clone, Debug)]
pub struct PreviewOptions {
    pub output: PathBuf,
    pub media_root: PathBuf,
    pub lock: Option<ResolveLock>,
    /// Explicit video/audio contract for the complete export session.
    pub spec: OutputSpec,
    /// Renderer required for the whole sample/export session. Never falls back.
    pub renderer: RendererRequest,
    /// When true, missing files may be replaced with a generated testsrc fixture.
    /// Production render must leave this false.
    pub allow_fixtures: bool,
    /// Optional font override. Production uses project-local / lock / fixture.
    pub font: Option<PathBuf>,
}

impl PreviewOptions {
    pub fn new(output: PathBuf, media_root: PathBuf) -> Self {
        Self {
            output,
            media_root,
            lock: None,
            spec: OutputSpec::preview(),
            renderer: RendererRequest::RequireCpu,
            allow_fixtures: false,
            font: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ExportReport {
    pub output: PathBuf,
    pub duration: Time,
    pub spec: OutputSpecReport,
    pub plan: PlanSummary,
    pub renderer: RendererSelection,
}

/// Serializable form of the backend-neutral output contract used for an export.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct OutputSpecReport {
    pub width: u32,
    pub height: u32,
    pub fps_num: i64,
    pub fps_den: i64,
    pub sample_rate: u32,
    pub channels: u16,
}

impl From<OutputSpec> for OutputSpecReport {
    fn from(spec: OutputSpec) -> Self {
        Self {
            width: spec.width,
            height: spec.height,
            fps_num: spec.fps_num,
            fps_den: spec.fps_den,
            sample_rate: spec.sample_rate,
            channels: spec.channels,
        }
    }
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

/// Encode a flattened timeline through Lattice compositor + codec mux.
///
/// Missing user media is an error unless [`PreviewOptions::allow_fixtures`] is set.
pub fn export_preview(
    timeline: &Timeline,
    options: &PreviewOptions,
) -> Result<ExportReport, ExportError> {
    crate::sample::render_timeline(timeline, options)
}

pub(crate) fn validate_export_spec(spec: OutputSpec) -> Result<(), ExportError> {
    for (field, value) in [("width", spec.width), ("height", spec.height)] {
        if value == 0 {
            return Err(ExportError::InvalidOutputSpec {
                field,
                value: value.to_string(),
                reason: "must be greater than zero",
            });
        }
        if value % 2 != 0 {
            return Err(ExportError::InvalidOutputSpec {
                field,
                value: value.to_string(),
                reason: "must be even for the yuv420p output contract",
            });
        }
    }
    for (field, value) in [("fps_num", spec.fps_num), ("fps_den", spec.fps_den)] {
        if value <= 0 {
            return Err(ExportError::InvalidOutputSpec {
                field,
                value: value.to_string(),
                reason: "must be greater than zero",
            });
        }
    }
    if spec.sample_rate == 0 {
        return Err(ExportError::InvalidOutputSpec {
            field: "sample_rate",
            value: spec.sample_rate.to_string(),
            reason: "must be greater than zero",
        });
    }
    if spec.channels == 0 {
        return Err(ExportError::InvalidOutputSpec {
            field: "channels",
            value: spec.channels.to_string(),
            reason: "must be greater than zero",
        });
    }
    Ok(())
}

#[cfg(test)]
fn output_parent(output: &Path) -> Option<&Path> {
    output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
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
    generate_av_fixture(
        dir.join("_lattice_source.mp4"),
        DEFAULT_SOURCE_DURATION_SECS,
    )
}

#[cfg(test)]
fn locked_generated_path(
    lock: Option<&ResolveLock>,
    window: &AudioWindow,
    media_root: &Path,
) -> Option<PathBuf> {
    let lock = lock?;
    let media_name = window.media_name.as_deref()?;
    let want_id = format!("media:{media_name}");
    lock.assets
        .iter()
        .find(|asset| {
            asset.generator.as_deref() == Some("speech")
                && (asset.id == want_id || asset.id == media_name)
        })
        .and_then(|asset| existing_lock_file(&asset.path, media_root))
}

#[cfg(test)]
fn existing_lock_file(stored: &str, media_root: &Path) -> Option<PathBuf> {
    let candidate = Path::new(stored);
    if candidate.is_absolute() {
        return existing_file(candidate);
    }
    existing_file(&media_root.join(candidate)).or_else(|| existing_file(candidate))
}

#[cfg(test)]
fn existing_file(path: &Path) -> Option<PathBuf> {
    if let Ok(canonical) = std::fs::canonicalize(path)
        && canonical.is_file()
    {
        return Some(strip_verbatim(&canonical));
    }
    path.is_file().then(|| path.to_path_buf())
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
    use std::path::Path;

    use lattice_core::{AssetIdentity, LockedAsset, MediaLocator, ResolveLock, Time, TimeSpan};

    use super::{ExportError, ffmpeg_seconds, validate_export_spec};
    use crate::OutputSpec;
    use crate::plan::AudioWindow;

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
    fn validates_backend_neutral_output_contract() {
        validate_export_spec(OutputSpec::preview()).expect("default preview spec");

        let mut spec = OutputSpec::preview();
        spec.width = 1920;
        spec.height = 1080;
        spec.fps_num = 30_000;
        spec.fps_den = 1001;
        validate_export_spec(spec).expect("1080p fractional-rate spec");

        spec.width = 1919;
        assert!(matches!(
            validate_export_spec(spec),
            Err(ExportError::InvalidOutputSpec { field: "width", .. })
        ));
        spec.width = 1920;
        spec.fps_den = 0;
        assert!(matches!(
            validate_export_spec(spec),
            Err(ExportError::InvalidOutputSpec {
                field: "fps_den",
                ..
            })
        ));
    }

    #[test]
    fn output_parent_skips_bare_filename() {
        assert_eq!(super::output_parent(Path::new("preview.mp4")), None);
        assert_eq!(
            super::output_parent(Path::new("examples/warframe-cut/preview.mp4")),
            Some(Path::new("examples/warframe-cut"))
        );
    }

    #[test]
    fn locked_speech_path_resolves_against_media_root() {
        let dir = std::env::temp_dir().join("lattice-lock-speech-rel");
        let _ = std::fs::remove_dir_all(&dir);
        let artifacts = dir.join(".lattice");
        std::fs::create_dir_all(&artifacts).unwrap();
        let wav = artifacts.join("speech.wav");
        std::fs::write(&wav, b"RIFF").unwrap();
        let lock = ResolveLock {
            schema_version: 1,
            assets: vec![LockedAsset {
                id: "media:speech-nice-freeze".into(),
                generator: Some("speech".into()),
                key: "k".into(),
                path: ".lattice/speech.wav".into(),
                identity: AssetIdentity::new("x"),
                duration: None,
                provider: None,
                provider_version: None,
            }],
        };
        let window = AudioWindow {
            span: TimeSpan::new(Time::ZERO, Time::seconds(1)),
            gain_db: None,
            generated: true,
            media_name: Some("speech-nice-freeze".into()),
            locator: None,
            content_start: Time::ZERO,
            hold: false,
        };
        let found = super::locked_generated_path(Some(&lock), &window, &dir).expect("lock wav");
        assert!(found.is_file(), "{}", found.display());
        assert!(
            found.is_absolute(),
            "ffmpeg current_dir needs an absolute input"
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
    fn export_path_has_no_filtergraph_compositor() {
        let src = include_str!("export.rs");
        let code = src.split("#[cfg(test)]").next().expect("src");
        for token in ["drawtext", "drawbox", "filter_complex", "amix", "volume="] {
            assert!(
                !code.contains(token),
                "legacy compositor token `{token}` still in export.rs"
            );
        }
    }
}

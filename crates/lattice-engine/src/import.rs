use std::path::{Path, PathBuf};

use lattice_core::Time;
use lattice_media::{MediaInfo, probe_media};

use crate::EngineError;

#[derive(Clone, Debug)]
pub struct ImportResult {
    pub project_dir: PathBuf,
    pub vel_path: PathBuf,
    pub source: String,
    pub media_info: MediaInfo,
    pub locator: String,
}

/// Create a text-first VEL project that references `media` in place (no copy).
pub fn import_media(media: &Path, out_dir: Option<&Path>) -> Result<ImportResult, EngineError> {
    if !media.is_file() {
        return Err(EngineError::Edit(format!(
            "media file is missing: {}",
            media.display()
        )));
    }
    let info = probe_media(media).map_err(|err| EngineError::Edit(err.to_string()))?;
    if !info.has_video {
        return Err(EngineError::Edit(format!(
            "media has no video stream: {}",
            media.display()
        )));
    }
    let stem = media
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("project");
    let project_dir = out_dir.map_or_else(|| PathBuf::from(stem), Path::to_path_buf);
    std::fs::create_dir_all(&project_dir)?;
    let vel_path = project_dir.join("main.vel");
    let locator = relative_locator(&project_dir, media)?;
    let media_name = sanitize_ident(stem);
    let source = render_imported_vel(&media_name, &locator, info.duration);
    crate::atomic::write_source_atomic(&vel_path, &source)?;
    Ok(ImportResult {
        project_dir,
        vel_path,
        source,
        media_info: info,
        locator,
    })
}

fn render_imported_vel(media_name: &str, locator: &str, duration: Time) -> String {
    format!(
        "project \"{media_name}\"\n\n\
         convention commentary\n\n\
         media {media_name} \"{locator}\"\n\n\
         sequence main {{\n  clip\n}}\n\n\
         scene clip {{\n  {media_name}[0s..{duration}] as video\n}}\n"
    )
}

fn sanitize_ident(stem: &str) -> String {
    let mut out = String::new();
    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    let out = out.trim_matches('_').to_ascii_lowercase();
    if out.is_empty() || out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        format!("media_{out}")
    } else {
        out
    }
}

fn relative_locator(project_dir: &Path, media: &Path) -> Result<String, EngineError> {
    let project_dir = strip_verbatim(&std::fs::canonicalize(project_dir)?);
    let media = strip_verbatim(&std::fs::canonicalize(media)?);
    let relative = pathdiff(&project_dir, &media).unwrap_or_else(|| media.display().to_string());
    Ok(relative.replace('\\', "/"))
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

fn pathdiff(base: &Path, target: &Path) -> Option<String> {
    let base: Vec<_> = base.components().collect();
    let target: Vec<_> = target.components().collect();
    let mut i = 0;
    while i < base.len() && i < target.len() && base[i] == target[i] {
        i += 1;
    }
    if i == 0 {
        return None;
    }
    let mut parts = vec![".."; base.len().saturating_sub(i)];
    for component in &target[i..] {
        parts.push(component.as_os_str().to_str()?);
    }
    if parts.is_empty() {
        Some(".".into())
    } else {
        Some(parts.join("/"))
    }
}

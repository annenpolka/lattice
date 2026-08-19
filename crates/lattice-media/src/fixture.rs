use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

use crate::export::ffmpeg_bin;
use crate::{PREVIEW_FPS_DEN, PREVIEW_FPS_NUM, PREVIEW_HEIGHT, PREVIEW_WIDTH};

/// Default generated source length: covers `game[10s..20s]`.
pub const DEFAULT_SOURCE_DURATION_SECS: i64 = 21;

#[derive(Debug, Error)]
pub enum FixtureError {
    #[error("failed to run ffmpeg to generate test source: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("ffmpeg test-source generation failed (status {status}): {stderr}")]
    Ffmpeg { status: String, stderr: String },
}

/// Write a deterministic `testsrc` MP4 long enough for the walking-skeleton trim.
pub fn generate_test_source(
    path: impl AsRef<Path>,
    duration_secs: i64,
) -> Result<PathBuf, FixtureError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let duration = duration_secs.max(1);
    let size = format!("{PREVIEW_WIDTH}x{PREVIEW_HEIGHT}");
    let rate = format!("{PREVIEW_FPS_NUM}/{PREVIEW_FPS_DEN}");
    let lavfi = format!("testsrc=duration={duration}:size={size}:rate={rate}");
    let output = Command::new(ffmpeg_bin())
        .args([
            "-y", "-f", "lavfi", "-i", &lavfi, "-pix_fmt", "yuv420p", "-an",
        ])
        .arg(path)
        .output()?;
    if !output.status.success() {
        return Err(FixtureError::Ffmpeg {
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(path.to_path_buf())
}

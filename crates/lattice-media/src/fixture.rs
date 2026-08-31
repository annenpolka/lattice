use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

use crate::export::ffmpeg_bin;
use crate::runtime::FfmpegRuntimeError;
use crate::{PREVIEW_FPS_DEN, PREVIEW_FPS_NUM, PREVIEW_HEIGHT, PREVIEW_WIDTH};

/// Default generated source length: covers `game[10s..20s]`.
pub const DEFAULT_SOURCE_DURATION_SECS: i64 = 21;

#[derive(Debug, Error)]
pub enum FixtureError {
    #[error(transparent)]
    Runtime(#[from] FfmpegRuntimeError),
    #[error("fixture I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

/// Write a deterministic `testsrc` MP4 long enough for the walking-skeleton trim.
/// Video only. Prefer [`generate_av_fixture`] when source audio is required.
pub fn generate_test_source(
    path: impl AsRef<Path>,
    duration_secs: i64,
) -> Result<PathBuf, FixtureError> {
    write_lavfi_mp4(path, duration_secs, false)
}

/// Deterministic audio+video fixture for import / cut / export tests.
///
/// Explicit test hook — production render must not call this for missing user media.
pub fn generate_av_fixture(
    path: impl AsRef<Path>,
    duration_secs: i64,
) -> Result<PathBuf, FixtureError> {
    generate_av_fixture_rate(path, duration_secs, PREVIEW_FPS_NUM, PREVIEW_FPS_DEN)
}

/// Like [`generate_av_fixture`] but with an explicit video frame rate (e.g. 30/1).
pub fn generate_av_fixture_rate(
    path: impl AsRef<Path>,
    duration_secs: i64,
    fps_num: i64,
    fps_den: i64,
) -> Result<PathBuf, FixtureError> {
    write_lavfi_mp4_rate(
        path,
        duration_secs,
        true,
        fps_num,
        fps_den,
        PREVIEW_WIDTH,
        PREVIEW_HEIGHT,
    )
}

/// Like [`generate_av_fixture`] with an explicit frame size (for aspect tests).
pub fn generate_av_fixture_size(
    path: impl AsRef<Path>,
    duration_secs: i64,
    width: u32,
    height: u32,
) -> Result<PathBuf, FixtureError> {
    write_lavfi_mp4_rate(
        path,
        duration_secs,
        true,
        PREVIEW_FPS_NUM,
        PREVIEW_FPS_DEN,
        width.max(2),
        height.max(2),
    )
}

fn write_lavfi_mp4(
    path: impl AsRef<Path>,
    duration_secs: i64,
    with_audio: bool,
) -> Result<PathBuf, FixtureError> {
    write_lavfi_mp4_rate(
        path,
        duration_secs,
        with_audio,
        PREVIEW_FPS_NUM,
        PREVIEW_FPS_DEN,
        PREVIEW_WIDTH,
        PREVIEW_HEIGHT,
    )
}

fn write_lavfi_mp4_rate(
    path: impl AsRef<Path>,
    duration_secs: i64,
    with_audio: bool,
    fps_num: i64,
    fps_den: i64,
    width: u32,
    height: u32,
) -> Result<PathBuf, FixtureError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let duration = duration_secs.max(1);
    let size = format!("{width}x{height}");
    let rate = format!("{fps_num}/{fps_den}");
    let video = format!("testsrc=duration={duration}:size={size}:rate={rate}");
    let executable = ffmpeg_bin();
    let mut command = Command::new(&executable);
    command.args(["-y", "-f", "lavfi", "-i", &video]);
    if with_audio {
        let tone = format!("sine=frequency=440:sample_rate=44100:duration={duration}");
        command.args(["-f", "lavfi", "-i", &tone, "-shortest"]);
    } else {
        command.arg("-an");
    }
    command.args(["-pix_fmt", "yuv420p"]).arg(path);
    let output = command.output().map_err(|source| {
        FfmpegRuntimeError::ffmpeg_unavailable(
            &executable,
            "generating a test media fixture",
            source,
        )
    })?;
    if !output.status.success() {
        return Err(FfmpegRuntimeError::ffmpeg_failed(
            "generating a test media fixture",
            output.status.to_string(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
        .into());
    }
    Ok(path.to_path_buf())
}

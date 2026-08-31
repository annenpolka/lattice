//! Typed diagnostics for the external `FFmpeg` runtime boundary.

use std::io;
use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FfmpegRuntimeError {
    #[error(
        "cannot run {tool} executable `{executable}` while {operation}: {source}; install FFmpeg and ensure `{tool}` is on PATH, or set {env_var} to an executable path"
    )]
    Unavailable {
        tool: &'static str,
        executable: String,
        operation: &'static str,
        env_var: &'static str,
        #[source]
        source: io::Error,
    },
    #[error("{tool} failed while {operation} (status {status}): {stderr}")]
    Failed {
        tool: &'static str,
        operation: &'static str,
        status: String,
        stderr: String,
    },
}

impl FfmpegRuntimeError {
    pub(crate) fn unavailable(
        tool: &'static str,
        executable: &Path,
        operation: &'static str,
        env_var: &'static str,
        source: io::Error,
    ) -> Self {
        Self::Unavailable {
            tool,
            executable: executable.display().to_string(),
            operation,
            env_var,
            source,
        }
    }

    pub(crate) fn ffmpeg_unavailable(
        executable: &Path,
        operation: &'static str,
        source: io::Error,
    ) -> Self {
        Self::unavailable("ffmpeg", executable, operation, "LATTICE_FFMPEG", source)
    }

    pub(crate) fn ffprobe_unavailable(
        executable: &Path,
        operation: &'static str,
        source: io::Error,
    ) -> Self {
        Self::unavailable("ffprobe", executable, operation, "LATTICE_FFPROBE", source)
    }

    pub(crate) fn failed(
        tool: &'static str,
        operation: &'static str,
        status: String,
        stderr: impl Into<String>,
    ) -> Self {
        Self::Failed {
            tool,
            operation,
            status,
            stderr: stderr.into(),
        }
    }

    pub(crate) fn ffmpeg_failed(
        operation: &'static str,
        status: String,
        stderr: impl Into<String>,
    ) -> Self {
        Self::failed("ffmpeg", operation, status, stderr)
    }

    pub(crate) fn ffprobe_failed(
        operation: &'static str,
        status: String,
        stderr: impl Into<String>,
    ) -> Self {
        Self::failed("ffprobe", operation, status, stderr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_error_names_tool_path_operation_and_override() {
        let error = FfmpegRuntimeError::ffmpeg_unavailable(
            Path::new("/missing/ffmpeg"),
            "decoding a video frame",
            io::Error::new(io::ErrorKind::NotFound, "missing"),
        );
        let message = error.to_string();
        assert!(message.contains("/missing/ffmpeg"));
        assert!(message.contains("decoding a video frame"));
        assert!(message.contains("LATTICE_FFMPEG"));
    }
}

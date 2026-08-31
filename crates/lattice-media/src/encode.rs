//! Encode/mux already-drawn RGBA frames and mixed PCM. No compositor filtergraph.
#![allow(clippy::cast_possible_truncation)]

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};

use lattice_core::Time;

use crate::backend::{Encoder, OutputSpec, PcmBuffer, RawFrame};
use crate::export::{ExportError, ffmpeg_bin, ffmpeg_seconds};
use crate::runtime::FfmpegRuntimeError;

pub struct FfmpegEncoder {
    child: Child,
    executable: PathBuf,
    stdin: Option<ChildStdin>,
    spec: OutputSpec,
    audio_path: Option<PathBuf>,
}

impl FfmpegEncoder {
    pub fn start(
        output: &Path,
        spec: OutputSpec,
        duration: Time,
        audio: Option<&PcmBuffer>,
    ) -> Result<Self, ExportError> {
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let fps = format!("{}/{}", spec.fps_num, spec.fps_den);
        let size = format!("{}x{}", spec.width, spec.height);
        let audio_path = if let Some(pcm) = audio {
            let path = output.with_extension("pcm");
            let bytes = pcm.to_s16le();
            std::fs::write(&path, bytes)?;
            Some(path)
        } else {
            None
        };
        let executable = ffmpeg_bin();
        let mut command = Command::new(&executable);
        command
            .args(["-y", "-hide_banner", "-loglevel", "error"])
            .args([
                "-f", "rawvideo", "-pix_fmt", "rgba", "-s", &size, "-r", &fps, "-i", "-",
            ]);
        if let Some(path) = &audio_path {
            command.args([
                "-f",
                "s16le",
                "-ar",
                &spec.sample_rate.to_string(),
                "-ac",
                &spec.channels.to_string(),
                "-i",
            ]);
            command.arg(path);
        }
        command.args([
            "-t",
            &ffmpeg_seconds(duration),
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ]);
        if audio_path.is_some() {
            command.args(["-c:a", "aac"]);
        } else {
            command.arg("-an");
        }
        command.arg(output);
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|source| {
            FfmpegRuntimeError::ffmpeg_unavailable(&executable, "starting the MP4 encoder", source)
        })?;
        let stdin = child.stdin.take();
        Ok(Self {
            child,
            executable,
            stdin,
            spec,
            audio_path,
        })
    }
}

impl Encoder for FfmpegEncoder {
    fn push_frame(&mut self, frame: &RawFrame) -> Result<(), ExportError> {
        if frame.width != self.spec.width || frame.height != self.spec.height {
            return Err(FfmpegRuntimeError::ffmpeg_failed(
                "encoding an MP4 frame",
                "size".into(),
                format!(
                    "frame {}x{} != spec {}x{}",
                    frame.width, frame.height, self.spec.width, self.spec.height
                ),
            )
            .into());
        }
        let stdin = self.stdin.as_mut().ok_or_else(|| {
            FfmpegRuntimeError::ffmpeg_failed(
                "encoding an MP4 frame",
                "stdin".into(),
                "encoder stdin closed",
            )
        })?;
        stdin.write_all(&frame.rgba)?;
        Ok(())
    }

    fn set_audio(&mut self, pcm: &PcmBuffer) -> Result<(), ExportError> {
        let path = self.audio_path.clone().ok_or_else(|| {
            FfmpegRuntimeError::ffmpeg_failed(
                "attaching PCM audio to the MP4 encoder",
                "audio".into(),
                "encoder was started without an audio slot",
            )
        })?;
        std::fs::write(path, pcm.to_s16le())?;
        Ok(())
    }

    fn finish(mut self) -> Result<(), ExportError> {
        drop(self.stdin.take());
        let output = self.child.wait_with_output().map_err(|source| {
            FfmpegRuntimeError::ffmpeg_unavailable(
                &self.executable,
                "waiting for the MP4 encoder",
                source,
            )
        })?;
        if let Some(path) = self.audio_path {
            let _ = std::fs::remove_file(path);
        }
        if !output.status.success() {
            return Err(FfmpegRuntimeError::ffmpeg_failed(
                "encoding the MP4 output",
                output.status.to_string(),
                String::from_utf8_lossy(&output.stderr).into_owned(),
            )
            .into());
        }
        Ok(())
    }
}

pub fn write_png(frame: &RawFrame, path: &Path) -> Result<(), ExportError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    let mut encoder = png::Encoder::new(file, frame.width, frame.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|err| ExportError::Map(format!("PNG header: {err}")))?;
    writer
        .write_image_data(&frame.rgba)
        .map_err(|err| ExportError::Map(format!("PNG image data: {err}")))?;
    Ok(())
}

pub fn write_frame_image(frame: &RawFrame, path: &Path) -> Result<(), ExportError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("ppm")
        .to_ascii_lowercase();
    if ext == "png" {
        write_png(frame, path)
    } else {
        frame.write_ppm(path).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn encoder_source_is_rawvideo_not_filtergraph() {
        let src = include_str!("encode.rs");
        let code = src.split("#[cfg(test)]").next().expect("src");
        assert!(code.contains("rawvideo"));
        assert!(code.contains("libx264"));
        assert!(!code.contains("drawtext"));
        assert!(!code.contains("drawbox"));
        assert!(!code.contains("filter_complex"));
        assert!(!code.contains("amix"));
        assert!(!code.contains("volume="));
    }
}

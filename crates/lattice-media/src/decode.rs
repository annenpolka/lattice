//! `FFmpeg` decode/probe backend. Visual/audio semantics live in evaluate + mixer.

use std::collections::{HashMap, VecDeque};
use std::io::{Read, Result as IoResult};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};

use lattice_core::{AssetRef, MediaLocator, ResolveLock, Time};

use crate::audio::AudioMixError;
use crate::backend::{PcmBuffer, RawFrame, VideoDecoder};
use crate::export::{ExportError, ffmpeg_bin, ffmpeg_seconds, resolve_media_path};
use crate::runtime::FfmpegRuntimeError;

type FrameKey = (String, String, u32, u32);
type StreamKey = (PathBuf, u32, u32);

const MAX_CACHED_FRAMES: usize = 12;
const MAX_STREAM_SKIP_FRAMES: u64 = 120;

pub struct FfmpegVideoDecoder {
    pub media_root: PathBuf,
    pub output_hint: PathBuf,
    pub allow_fixtures: bool,
    cache: HashMap<FrameKey, RawFrame>,
    order: VecDeque<FrameKey>,
    stream: Option<SequentialStream>,
    fps_num: i64,
    fps_den: i64,
    #[cfg(test)]
    stream_starts: usize,
}

/// One `FFmpeg` rawvideo pipe following monotonically increasing `sample(t)` calls.
///
/// This stream deliberately owns no playback clock. `SampleSession` still decides which exact
/// content time to request; this object only avoids starting a decoder process for every adjacent
/// frame.
struct SequentialStream {
    key: StreamKey,
    next_time: Time,
    child: Child,
    stdout: ChildStdout,
}

impl SequentialStream {
    fn read_frame(&mut self, width: u32, height: u32) -> IoResult<RawFrame> {
        let expected = frame_byte_len(width, height);
        let mut rgba = vec![0; expected];
        self.stdout.read_exact(&mut rgba)?;
        Ok(RawFrame {
            width,
            height,
            rgba,
        })
    }
}

impl Drop for SequentialStream {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl FfmpegVideoDecoder {
    pub fn with_frame_rate(
        media_root: PathBuf,
        output_hint: PathBuf,
        allow_fixtures: bool,
        fps_num: i64,
        fps_den: i64,
    ) -> Self {
        Self {
            media_root,
            output_hint,
            allow_fixtures,
            cache: HashMap::new(),
            order: VecDeque::new(),
            stream: None,
            fps_num: fps_num.max(1),
            fps_den: fps_den.max(1),
            #[cfg(test)]
            stream_starts: 0,
        }
    }

    fn remember(&mut self, key: FrameKey, frame: RawFrame) {
        if !self.cache.contains_key(&key) {
            while self.cache.len() >= MAX_CACHED_FRAMES {
                if let Some(old) = self.order.pop_front() {
                    self.cache.remove(&old);
                } else {
                    break;
                }
            }
            self.order.push_back(key.clone());
        }
        self.cache.insert(key, frame);
    }

    fn sample_sequential(
        &mut self,
        path: &Path,
        content_time: Time,
        width: u32,
        height: u32,
    ) -> Result<RawFrame, ExportError> {
        let stream_key = (path.to_path_buf(), width, height);
        let frame_duration = Time::new(self.fps_den, self.fps_num)
            .map_err(|err| ExportError::Map(err.to_string()))?;
        let skip_frames = self.stream.as_ref().and_then(|stream| {
            if stream.key != stream_key || content_time < stream.next_time {
                return None;
            }
            let delta = content_time.checked_sub(stream.next_time).ok()?;
            let frames = delta.exact_frame_count(self.fps_num, self.fps_den).ok()?;
            (frames <= MAX_STREAM_SKIP_FRAMES).then_some(frames)
        });
        let skip_frames = if let Some(frames) = skip_frames {
            frames
        } else {
            self.stream = Some(self.start_stream(stream_key, content_time)?);
            0
        };

        let stream = self.stream.as_mut().ok_or_else(|| {
            FfmpegRuntimeError::ffmpeg_failed(
                "decoding sequential rawvideo",
                "decoder-missing".into(),
                "sequential decoder was not created",
            )
        })?;
        for _ in 0..skip_frames {
            let _ = stream.read_frame(width, height).map_err(|err| {
                FfmpegRuntimeError::ffmpeg_failed(
                    "decoding sequential rawvideo",
                    "short-read".into(),
                    format!("sequential rawvideo skip failed: {err}"),
                )
            })?;
            stream.next_time = stream
                .next_time
                .checked_add(frame_duration)
                .unwrap_or(stream.next_time);
        }
        let frame = stream.read_frame(width, height).map_err(|err| {
            FfmpegRuntimeError::ffmpeg_failed(
                "decoding sequential rawvideo",
                "short-read".into(),
                format!("sequential rawvideo read failed: {err}"),
            )
        })?;
        stream.next_time = content_time
            .checked_add(frame_duration)
            .unwrap_or(content_time);
        Ok(frame)
    }

    fn start_stream(
        &mut self,
        key: StreamKey,
        content_time: Time,
    ) -> Result<SequentialStream, ExportError> {
        let (path, width, height) = &key;
        let fps = format!("{}/{}", self.fps_num, self.fps_den);
        let filter = format!("fps={fps}:round=near,scale={width}:{height}:flags=bilinear");
        let executable = ffmpeg_bin();
        let mut child = Command::new(&executable)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-ss",
                &ffmpeg_seconds(content_time),
                "-i",
            ])
            .arg(path)
            .args([
                "-an", "-vf", &filter, "-f", "rawvideo", "-pix_fmt", "rgba", "-",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|source| {
                FfmpegRuntimeError::ffmpeg_unavailable(
                    &executable,
                    "starting the sequential video decoder",
                    source,
                )
            })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            FfmpegRuntimeError::ffmpeg_failed(
                "starting the sequential video decoder",
                "decoder-pipe".into(),
                "FFmpeg did not expose its rawvideo stdout",
            )
        })?;
        #[cfg(test)]
        {
            self.stream_starts = self.stream_starts.saturating_add(1);
        }
        Ok(SequentialStream {
            key,
            next_time: content_time,
            child,
            stdout,
        })
    }
}

impl VideoDecoder for FfmpegVideoDecoder {
    fn sample(
        &mut self,
        asset: &AssetRef,
        content_time: Time,
        width: u32,
        height: u32,
    ) -> Result<RawFrame, ExportError> {
        let path = resolve_media_path(
            &asset.locator,
            &self.media_root,
            &self.output_hint,
            self.allow_fixtures,
        )?;
        let key = (
            path.display().to_string(),
            format!("{}/{}", content_time.num(), content_time.den()),
            width,
            height,
        );
        if let Some(frame) = self.cache.get(&key) {
            return Ok(frame.clone());
        }
        let frame = if let Ok(frame) = self.sample_sequential(&path, content_time, width, height) {
            frame
        } else {
            // A corrupt or exhausted pipe must not poison a later seek/scrub. Preserve the
            // established exact one-shot path as decoder recovery; renderer selection remains
            // explicit and is never changed here.
            self.stream = None;
            decode_rgba_frame(&path, content_time, width, height)?
        };
        self.remember(key, frame.clone());
        Ok(frame)
    }
}

fn frame_byte_len(width: u32, height: u32) -> usize {
    (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4)
}

pub fn decode_rgba_frame(
    path: &Path,
    at: Time,
    width: u32,
    height: u32,
) -> Result<RawFrame, ExportError> {
    let executable = ffmpeg_bin();
    let output = Command::new(&executable)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-ss",
            &ffmpeg_seconds(at),
            "-i",
        ])
        .arg(path)
        .args([
            "-frames:v",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-s",
            &format!("{width}x{height}"),
            "-",
        ])
        .output()
        .map_err(|source| {
            FfmpegRuntimeError::ffmpeg_unavailable(&executable, "decoding a video frame", source)
        })?;
    if !output.status.success() {
        return Err(FfmpegRuntimeError::ffmpeg_failed(
            "decoding a video frame",
            output.status.to_string(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
        .into());
    }
    let expected = frame_byte_len(width, height);
    if output.stdout.len() < expected {
        return Err(FfmpegRuntimeError::ffmpeg_failed(
            "decoding a video frame",
            "short-read".into(),
            format!("expected {expected} bytes, got {}", output.stdout.len()),
        )
        .into());
    }
    Ok(RawFrame {
        width,
        height,
        rgba: output.stdout[..expected].to_vec(),
    })
}

pub fn decode_pcm_f32(
    path: &Path,
    sample_rate: u32,
    channels: u16,
) -> Result<PcmBuffer, ExportError> {
    let executable = ffmpeg_bin();
    let output = Command::new(&executable)
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(path)
        .args([
            "-vn",
            "-ac",
            &channels.to_string(),
            "-ar",
            &sample_rate.to_string(),
            "-f",
            "f32le",
            "-",
        ])
        .output()
        .map_err(|source| {
            FfmpegRuntimeError::ffmpeg_unavailable(&executable, "decoding PCM audio", source)
        })?;
    if !output.status.success() {
        return Err(FfmpegRuntimeError::ffmpeg_failed(
            "decoding PCM audio",
            output.status.to_string(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
        .into());
    }
    let mut samples = Vec::with_capacity(output.stdout.len() / 4);
    for chunk in output.stdout.chunks_exact(4) {
        samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(PcmBuffer {
        sample_rate,
        channels,
        samples,
    })
}

pub fn collect_audio_sources(
    plan: &lattice_core::AudioPlan,
    media_root: &Path,
    output: &Path,
    lock: Option<&ResolveLock>,
    allow_fixtures: bool,
    spec: crate::mix::MixSpec,
) -> Result<HashMap<String, PcmBuffer>, AudioMixError> {
    let mut sources = HashMap::new();
    for (window_index, window) in plan.windows.iter().enumerate() {
        let asset = window
            .asset
            .as_ref()
            .ok_or(AudioMixError::MissingWindowSource { window_index })?;
        if sources.contains_key(&asset.media_name) {
            continue;
        }
        let path =
            match &asset.locator {
                MediaLocator::Generated { .. } => {
                    locked_generated_file(lock, &asset.media_name, media_root).ok_or_else(|| {
                        AudioMixError::MissingGeneratedAsset {
                            media_name: asset.media_name.clone(),
                        }
                    })?
                }
                other => resolve_media_path(other, media_root, output, allow_fixtures).map_err(
                    |source| AudioMixError::SourceUnavailable {
                        media_name: asset.media_name.clone(),
                        generated: window.generated,
                        source: Box::new(source),
                    },
                )?,
            };
        let pcm = decode_pcm_f32(&path, spec.sample_rate, spec.channels).map_err(|source| {
            AudioMixError::SourceUnavailable {
                media_name: asset.media_name.clone(),
                generated: window.generated,
                source: Box::new(source),
            }
        })?;
        if pcm.frame_count() == 0 {
            return Err(AudioMixError::EmptySource {
                media_name: asset.media_name.clone(),
                generated: window.generated,
            });
        }
        sources.insert(asset.media_name.clone(), pcm);
    }
    Ok(sources)
}

fn locked_generated_file(
    lock: Option<&ResolveLock>,
    media_name: &str,
    media_root: &Path,
) -> Option<PathBuf> {
    let lock = lock?;
    let want_id = format!("media:{media_name}");
    lock.assets
        .iter()
        .find(|asset| {
            asset.generator.as_deref() == Some("speech")
                && (asset.id == want_id || asset.id == media_name)
        })
        .and_then(|asset| {
            let candidate = Path::new(&asset.path);
            let path = if candidate.is_absolute() {
                candidate.to_path_buf()
            } else {
                media_root.join(candidate)
            };
            path.is_file().then_some(path)
        })
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use lattice_core::{AssetRef, MediaLocator};

    use super::*;

    #[test]
    fn adjacent_sample_times_reuse_one_ffmpeg_decoder() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "lattice-sequential-decode-{}-{nonce}",
            std::process::id()
        ));
        let source = root.join("source.mp4");
        crate::generate_test_source(&source, 2).expect("fixture");
        let asset = AssetRef {
            media_name: "clip".into(),
            locator: MediaLocator::File {
                path: "source.mp4".into(),
            },
        };
        let mut decoder = FfmpegVideoDecoder::with_frame_rate(
            root.clone(),
            root.join("preview.raw"),
            false,
            10,
            1,
        );

        let first = decoder
            .sample(&asset, Time::ZERO, 96, 54)
            .expect("first frame");
        let second = decoder
            .sample(&asset, Time::new(1, 10).expect("time"), 96, 54)
            .expect("adjacent frame");
        assert_eq!(decoder.stream_starts, 1, "adjacent frames share a pipe");
        assert_ne!(first.rgba, second.rgba, "fixture must visibly advance");

        // A cached repeated hold does not disturb the already-open sequential cursor.
        let held = decoder
            .sample(&asset, Time::ZERO, 96, 54)
            .expect("held frame");
        assert_eq!(held, first);
        let third = decoder
            .sample(&asset, Time::new(2, 10).expect("time"), 96, 54)
            .expect("next frame");
        assert_ne!(third.rgba, second.rgba);
        assert_eq!(decoder.stream_starts, 1, "cache hit keeps pipe position");

        let jumped = decoder
            .sample(&asset, Time::new(4, 10).expect("time"), 96, 54)
            .expect("dropped display frame");
        assert_ne!(jumped.rgba, third.rgba);
        assert_eq!(
            decoder.stream_starts, 1,
            "forward display-frame drops are consumed from the same pipe"
        );

        drop(decoder);
        let _ = std::fs::remove_dir_all(root);
    }
}

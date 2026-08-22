//! Decoder / `FrameRenderer` / Encoder boundaries. `FFmpeg` stays behind these traits.

use std::fmt;

use lattice_core::{AssetRef, RenderScene, Time};
use serde::Serialize;
use thiserror::Error;

use crate::export::ExportError;

/// Renderer choice fixed when a [`crate::SampleSession`] is created.
///
/// There is deliberately no `Auto` variant: a required backend either becomes
/// active or session creation fails without silently changing semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RendererRequest {
    RequireCpu,
    RequireGpuDx12,
}

impl fmt::Display for RendererRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::RequireCpu => "require_cpu",
            Self::RequireGpuDx12 => "require_gpu_dx12",
        })
    }
}

/// Backend that actually owns rendering for a sample session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RendererBackend {
    Cpu,
    GpuDx12,
}

impl fmt::Display for RendererBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Cpu => "cpu",
            Self::GpuDx12 => "gpu_dx12",
        })
    }
}

/// Observable result of resolving an explicit renderer request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RendererSelection {
    pub requested: RendererRequest,
    pub active: Option<RendererBackend>,
    /// Stable adapter name for an active hardware renderer.
    pub adapter: Option<String>,
    pub reason: String,
}

impl RendererSelection {
    pub(crate) fn cpu() -> Self {
        Self {
            requested: RendererRequest::RequireCpu,
            active: Some(RendererBackend::Cpu),
            adapter: None,
            reason: "explicit CPU renderer request".into(),
        }
    }

    pub(crate) fn unavailable(requested: RendererRequest, reason: impl Into<String>) -> Self {
        Self {
            requested,
            active: None,
            adapter: None,
            reason: reason.into(),
        }
    }

    pub(crate) fn gpu_dx12(adapter: &str) -> Self {
        Self {
            requested: RendererRequest::RequireGpuDx12,
            active: Some(RendererBackend::GpuDx12),
            adapter: Some(adapter.into()),
            reason: format!("explicit DX12 renderer request on `{adapter}`"),
        }
    }
}

impl fmt::Display for RendererSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let active = self.active.map_or("none", |backend| match backend {
            RendererBackend::Cpu => "cpu",
            RendererBackend::GpuDx12 => "gpu_dx12",
        });
        let adapter = self.adapter.as_deref().unwrap_or("none");
        write!(
            f,
            "requested={}, active={active}, adapter={adapter}, reason={}",
            self.requested, self.reason
        )
    }
}

/// Typed session-creation failure for an explicitly required renderer.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum RendererInitError {
    #[error("renderer unavailable ({selection})")]
    Unavailable { selection: RendererSelection },
    #[error("renderer initialization failed at {stage} ({selection}): {message}")]
    Initialization {
        selection: RendererSelection,
        stage: RendererInitStage,
        message: String,
    },
}

impl RendererInitError {
    #[must_use]
    pub fn selection(&self) -> &RendererSelection {
        match self {
            Self::Unavailable { selection } | Self::Initialization { selection, .. } => selection,
        }
    }
}

/// Stable initialization stage, so callers do not have to parse wgpu text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RendererInitStage {
    Adapter,
    Device,
    Pipeline,
}

impl fmt::Display for RendererInitStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Adapter => "adapter",
            Self::Device => "device",
            Self::Pipeline => "pipeline",
        })
    }
}

/// Typed failure after a renderer session has been created.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum RendererRenderError {
    #[error("GPU compositor does not support scene node `{node}`: {reason}")]
    UnsupportedScene { node: String, reason: String },
    #[error(
        "invalid RGBA frame {width}x{height}: expected {expected_bytes} bytes, got {actual_bytes}"
    )]
    InvalidFrame {
        width: u32,
        height: u32,
        expected_bytes: usize,
        actual_bytes: usize,
    },
    #[error("GPU command validation failed: {message}")]
    Validation { message: String },
    #[error("GPU device poll failed: {message}")]
    DevicePoll { message: String },
    #[error("GPU readback mapping failed: {message}")]
    Readback { message: String },
    #[error("GPU device was lost ({reason}): {message}")]
    DeviceLost { reason: String, message: String },
    #[error("GPU renderer ran out of memory: {message}")]
    OutOfMemory { message: String },
    #[error("GPU renderer internal failure: {message}")]
    Internal { message: String },
}

impl RendererRenderError {
    /// Stable machine-readable failure kind. Callers must not parse `Display` text.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::UnsupportedScene { .. } => "unsupported_scene",
            Self::InvalidFrame { .. } => "invalid_frame",
            Self::Validation { .. } => "validation",
            Self::DevicePoll { .. } => "device_poll",
            Self::Readback { .. } => "readback",
            Self::DeviceLost { .. } => "device_lost",
            Self::OutOfMemory { .. } => "out_of_memory",
            Self::Internal { .. } => "internal",
        }
    }

    /// Persistent failures require recreating the renderer session before retrying.
    #[must_use]
    pub const fn is_persistent(&self) -> bool {
        matches!(
            self,
            Self::DeviceLost { .. } | Self::OutOfMemory { .. } | Self::Internal { .. }
        )
    }
}

/// Packed RGBA8, top-left origin, no GPU types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl RawFrame {
    #[allow(clippy::many_single_char_names)]
    pub fn filled(width: u32, height: u32, r: u8, g: u8, b: u8, a: u8) -> Self {
        let n = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
        let mut rgba = Vec::with_capacity(n);
        for _ in 0..width.saturating_mul(height) {
            rgba.extend_from_slice(&[r, g, b, a]);
        }
        Self {
            width,
            height,
            rgba,
        }
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = ((y * self.width + x) * 4) as usize;
        Some([
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ])
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, px: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let i = ((y * self.width + x) * 4) as usize;
        self.rgba[i..i + 4].copy_from_slice(&px);
    }

    pub fn write_ppm(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut body = format!("P6\n{} {}\n255\n", self.width, self.height).into_bytes();
        body.reserve((self.width * self.height * 3) as usize);
        for chunk in self.rgba.chunks_exact(4) {
            body.extend_from_slice(&chunk[..3]);
        }
        std::fs::write(path, body)
    }
}

/// Interleaved float PCM in `-1.0..=1.0`. Mixer output; encoder converts to s16le.
#[derive(Clone, Debug, PartialEq)]
pub struct PcmBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

impl PcmBuffer {
    pub fn silence(sample_rate: u32, channels: u16, frames: usize) -> Self {
        Self {
            sample_rate,
            channels,
            samples: vec![0.0; frames.saturating_mul(channels as usize)],
        }
    }

    pub fn frame_count(&self) -> usize {
        if self.channels == 0 {
            0
        } else {
            self.samples.len() / self.channels as usize
        }
    }

    pub fn to_s16le(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.samples.len() * 2);
        for sample in &self.samples {
            let clipped = sample.clamp(-1.0, 1.0);
            #[allow(clippy::cast_possible_truncation)]
            let value = (clipped * 32767.0).round() as i16;
            out.extend_from_slice(&value.to_le_bytes());
        }
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputSpec {
    pub width: u32,
    pub height: u32,
    pub fps_num: i64,
    pub fps_den: i64,
    pub sample_rate: u32,
    pub channels: u16,
}

impl OutputSpec {
    pub fn preview() -> Self {
        Self {
            width: crate::PREVIEW_WIDTH,
            height: crate::PREVIEW_HEIGHT,
            fps_num: crate::PREVIEW_FPS_NUM,
            fps_den: crate::PREVIEW_FPS_DEN,
            sample_rate: 44_100,
            channels: 2,
        }
    }
}

pub trait VideoDecoder {
    fn sample(
        &mut self,
        asset: &AssetRef,
        content_time: Time,
        width: u32,
        height: u32,
    ) -> Result<RawFrame, ExportError>;
}

/// Draws a resolved `RenderScene`. Must not accept a visual filtergraph.
pub trait FrameRenderer {
    fn render(
        &mut self,
        scene: &RenderScene,
        sampler: &mut dyn VideoDecoder,
    ) -> Result<RawFrame, ExportError>;
}

pub trait AudioRenderer {
    fn mix(&self, plan: &lattice_core::AudioPlan) -> Result<PcmBuffer, ExportError>;
}

/// Mux already-drawn frames and already-mixed PCM. No scene semantics.
pub trait Encoder {
    fn push_frame(&mut self, frame: &RawFrame) -> Result<(), ExportError>;
    fn set_audio(&mut self, pcm: &PcmBuffer) -> Result<(), ExportError>;
    fn finish(self) -> Result<(), ExportError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoder_trait_takes_frames_not_filtergraph() {
        let src = include_str!("encode.rs");
        let code = src.split("#[cfg(test)]").next().expect("src");
        assert!(
            code.contains("push_frame"),
            "encoder must accept already-drawn frames"
        );
        assert!(
            !code.contains("drawtext")
                && !code.contains("drawbox")
                && !code.contains("filter_complex"),
            "encoder must not build a visual filtergraph"
        );
        assert!(!code.contains("amix") && !code.contains("volume="));
    }

    #[test]
    fn pcm_s16le_roundtrip_amplitude() {
        let pcm = PcmBuffer {
            sample_rate: 8000,
            channels: 1,
            samples: vec![0.0, 0.5, -0.5, 1.0],
        };
        let bytes = pcm.to_s16le();
        assert_eq!(bytes.len(), 8);
        let mid = i16::from_le_bytes([bytes[2], bytes[3]]);
        assert!((16_000..17_000).contains(&mid), "{mid}");
    }

    #[test]
    fn renderer_selection_serializes_adapter_and_displays_it() {
        let cpu = RendererSelection::cpu();
        let cpu_json = serde_json::to_value(&cpu).unwrap();
        assert!(cpu_json["adapter"].is_null());
        assert!(cpu.to_string().contains("adapter=none"));

        let gpu = RendererSelection::gpu_dx12("Adapter 1");
        let gpu_json = serde_json::to_value(&gpu).unwrap();
        assert_eq!(gpu_json["adapter"], "Adapter 1");
        assert!(gpu.to_string().contains("adapter=Adapter 1"));

        let unavailable =
            RendererSelection::unavailable(RendererRequest::RequireGpuDx12, "no DX12 adapter");
        assert!(unavailable.adapter.is_none());
    }

    #[test]
    fn persistent_render_failures_have_stable_kinds() {
        let lost = RendererRenderError::DeviceLost {
            reason: "destroyed".into(),
            message: "queue submission failed".into(),
        };
        assert_eq!(lost.kind(), "device_lost");
        assert!(lost.is_persistent());
        assert_eq!(
            RendererRenderError::OutOfMemory {
                message: "allocation failed".into()
            }
            .kind(),
            "out_of_memory"
        );
        assert!(
            RendererRenderError::Internal {
                message: "invariant".into()
            }
            .is_persistent()
        );
        let validation = RendererRenderError::Validation {
            message: "bad command".into(),
        };
        assert_eq!(validation.kind(), "validation");
        assert!(!validation.is_persistent());
    }
}

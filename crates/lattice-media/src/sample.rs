//! Shared sample/render-at-t path used by Studio preview and export.

use std::path::{Path, PathBuf};

use lattice_core::{
    Canvas, EvaluateOpts, FontIdentity, FontSpec, RenderScene, Time, Timeline, evaluate,
};
use thiserror::Error;

use crate::audio::mix_timeline_audio;
#[cfg(test)]
use crate::backend::RendererInitError;
use crate::backend::{
    Encoder, FrameRenderer, OutputSpec, RawFrame, RendererRequest, RendererSelection,
};
use crate::composite::CpuCompositor;
use crate::decode::FfmpegVideoDecoder;
use crate::encode::{FfmpegEncoder, write_frame_image};
use crate::export::{ExportError, ExportReport, PreviewOptions, validate_export_spec};
use crate::font::{FontResolution, resolve_font};
use crate::gpu::GpuCompositor;
use crate::mix::MixSpec;
use crate::plan::plan_from_timeline_with_spec;

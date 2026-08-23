//! CPU compositor: video blit + shape + text + transform + opacity. No `FFmpeg` filters.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::too_many_arguments
)]

use lattice_core::{
    BlendMode, GroupNode, Rect, RenderNode, RenderScene, Rgba, ShapeNode, TextNode, Transform,
    VideoNode,
};

use crate::backend::{FrameRenderer, RawFrame, VideoDecoder};
use crate::export::ExportError;
use crate::font::{FontResolution, resolve_font};
use crate::text::TextRasterizer;

pub struct CpuCompositor {
    pub font: Option<FontResolution>,
    rasterizer: Option<TextRasterizer>,
}

impl CpuCompositor {
    pub fn new(font: Option<FontResolution>) -> Self {
        let rasterizer = font.as_ref().map(TextRasterizer::new);
        Self { font, rasterizer }
    }

    pub fn from_paths(
        media_root: &std::path::Path,
        lock: Option<&lattice_core::ResolveLock>,
        override_font: Option<&std::path::Path>,
    ) -> Result<Self, ExportError> {
        let spec = lattice_core::FontSpec::preview_sans(18);
        let font = resolve_font(&spec, media_root, lock, override_font)?;
        Ok(Self::new(Some(font)))
    }
}

impl FrameRenderer for CpuCompositor {
    fn render(
        &mut self,
        scene: &RenderScene,
        sampler: &mut dyn VideoDecoder,
    ) -> Result<RawFrame, ExportError> {
        let mut frame = RawFrame::filled(scene.canvas.width, scene.canvas.height, 0, 0, 0, 255);
        let mut nodes: Vec<&RenderNode> = scene.nodes.iter().collect();
        nodes.sort_by_key(|node| node.z());
        if scene.has_text() && self.rasterizer.is_none() {
            return Err(ExportError::MissingFont);
        }
        for node in nodes {
            draw_node(
                &mut frame,
                node,
                Transform::IDENTITY,
                100,
                None,
                sampler,
                self,
            )?;
        }
        Ok(frame)
    }
}

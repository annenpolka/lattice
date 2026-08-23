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

#[allow(clippy::too_many_arguments)]
fn draw_node(
    frame: &mut RawFrame,
    node: &RenderNode,
    parent: Transform,
    parent_opacity: u8,
    clip: Option<Rect>,
    sampler: &mut dyn VideoDecoder,
    compositor: &mut CpuCompositor,
) -> Result<(), ExportError> {
    match node {
        RenderNode::Group(group) => draw_group(
            frame,
            group,
            parent,
            parent_opacity,
            clip,
            sampler,
            compositor,
        ),
        RenderNode::Video(video) => draw_video(frame, video, parent, parent_opacity, clip, sampler),
        RenderNode::Shape(shape) => {
            draw_shape(frame, shape, parent, parent_opacity, clip);
            Ok(())
        }
        RenderNode::Text(text) => draw_text(frame, text, parent, parent_opacity, clip, compositor),
        RenderNode::Image(image) => {
            let video = VideoNode {
                props: image.props.clone(),
                bounds: image.bounds,
                asset: image.asset.clone(),
                content_time: lattice_core::Time::ZERO,
                hold: true,
                time_map: lattice_core::TimeMap::identity(
                    lattice_core::Time::ZERO,
                    lattice_core::Time::ONE,
                ),
            };
            draw_video(frame, &video, parent, parent_opacity, clip, sampler)
        }
        RenderNode::Mask(mask) => draw_node(
            frame,
            mask.content.as_ref(),
            parent,
            parent_opacity,
            clip,
            sampler,
            compositor,
        ),
        RenderNode::Effect(effect) => draw_node(
            frame,
            effect.child.as_ref(),
            parent,
            parent_opacity,
            clip,
            sampler,
            compositor,
        ),
    }
}

fn draw_group(
    frame: &mut RawFrame,
    group: &GroupNode,
    parent: Transform,
    parent_opacity: u8,
    clip: Option<Rect>,
    sampler: &mut dyn VideoDecoder,
    compositor: &mut CpuCompositor,
) -> Result<(), ExportError> {
    let transform = compose_transform(parent, group.props.transform);
    let opacity = mul_opacity(parent_opacity, group.props.opacity);
    let clip = intersect_clip(clip, group.props.clip);
    let mut children: Vec<&RenderNode> = group.children.iter().collect();
    children.sort_by_key(|node| node.z());
    for child in children {
        draw_node(frame, child, transform, opacity, clip, sampler, compositor)?;
    }
    Ok(())
}

fn draw_video(
    frame: &mut RawFrame,
    video: &VideoNode,
    parent: Transform,
    parent_opacity: u8,
    clip: Option<Rect>,
    sampler: &mut dyn VideoDecoder,
) -> Result<(), ExportError> {
    let decoded = sampler.sample(
        &video.asset,
        video.content_time,
        video.bounds.width.max(1),
        video.bounds.height.max(1),
    )?;
    let transform = compose_transform(parent, video.props.transform);
    let opacity = mul_opacity(parent_opacity, video.props.opacity);
    let clip = intersect_clip(clip, video.props.clip);
    blit_transformed(
        frame,
        &decoded,
        video.bounds,
        video.bounds,
        transform,
        opacity,
        clip,
        video.props.blend,
        None,
    );
    Ok(())
}

fn draw_shape(
    frame: &mut RawFrame,
    shape: &ShapeNode,
    parent: Transform,
    parent_opacity: u8,
    clip: Option<Rect>,
) {
    let transform = compose_transform(parent, shape.props.transform);
    let opacity = mul_opacity(parent_opacity, shape.props.opacity);
    let clip = intersect_clip(clip, shape.props.clip);
    blit_transformed(
        frame,
        &solid_frame(shape.bounds, shape.fill),
        shape.bounds,
        shape.bounds,
        transform,
        opacity,
        clip,
        shape.props.blend,
        Some(shape.fill),
    );
}

fn draw_text(
    frame: &mut RawFrame,
    text: &TextNode,
    parent: Transform,
    parent_opacity: u8,
    clip: Option<Rect>,
    compositor: &mut CpuCompositor,
) -> Result<(), ExportError> {
    let opacity = mul_opacity(parent_opacity, text.props.opacity);
    if opacity == 0 || text.text.is_empty() {
        return Ok(());
    }
    if compositor.rasterizer.is_none() {
        return Err(ExportError::MissingFont);
    }
    if let (Some(expected), Some(loaded)) = (&text.resolved_font, compositor.font.as_ref())
        && expected.identity != loaded.identity.identity
    {
        return Err(ExportError::StaleFont(expected.path.clone()));
    }
    let transform = compose_transform(parent, text.props.transform);
    let clip = intersect_clip(clip, text.props.clip);
    let layer = compositor
        .rasterizer
        .as_mut()
        .expect("rasterizer")
        .rasterize_layer(text)?;
    blit_transformed(
        frame,
        &layer,
        text.bounds,
        Rect {
            x: 0,
            y: 0,
            width: frame.width,
            height: frame.height,
        },
        transform,
        opacity,
        clip,
        text.props.blend,
        None,
    );
    Ok(())
}

fn solid_frame(bounds: Rect, fill: Rgba) -> RawFrame {
    RawFrame::filled(
        bounds.width.max(1),
        bounds.height.max(1),
        fill.r,
        fill.g,
        fill.b,
        fill.a,
    )
}

fn mul_opacity(a: u8, b: u8) -> u8 {
    ((u16::from(a) * u16::from(b)) / 100) as u8
}

fn compose_transform(parent: Transform, child: Transform) -> Transform {
    Transform {
        translate_x: parent.translate_x + child.translate_x,
        translate_y: parent.translate_y + child.translate_y,
        scale_x: (i64::from(parent.scale_x) * i64::from(child.scale_x) / 1000) as i32,
        scale_y: (i64::from(parent.scale_y) * i64::from(child.scale_y) / 1000) as i32,
        rotation_mdeg: parent.rotation_mdeg + child.rotation_mdeg,
    }
}

fn intersect_clip(a: Option<Rect>, b: Option<Rect>) -> Option<Rect> {
    match (a, b) {
        (None, other) | (other, None) => other,
        (Some(a), Some(b)) => {
            let x = a.x.max(b.x);
            let y = a.y.max(b.y);
            let ax2 =
                a.x.saturating_add(i32::try_from(a.width).unwrap_or(i32::MAX));
            let ay2 =
                a.y.saturating_add(i32::try_from(a.height).unwrap_or(i32::MAX));
            let bx2 =
                b.x.saturating_add(i32::try_from(b.width).unwrap_or(i32::MAX));
            let by2 =
                b.y.saturating_add(i32::try_from(b.height).unwrap_or(i32::MAX));
            let x2 = ax2.min(bx2);
            let y2 = ay2.min(by2);
            if x2 <= x || y2 <= y {
                return Some(Rect {
                    x,
                    y,
                    width: 0,
                    height: 0,
                });
            }
            Some(Rect {
                x,
                y,
                width: (x2 - x) as u32,
                height: (y2 - y) as u32,
            })
        }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::too_many_arguments
)]
fn blit_transformed(
    dest: &mut RawFrame,
    src: &RawFrame,
    bounds: Rect,
    transform_pivot: Rect,
    transform: Transform,
    opacity: u8,
    clip: Option<Rect>,
    blend: BlendMode,
    solid: Option<Rgba>,
) {
    if opacity == 0 || src.width == 0 || src.height == 0 {
        return;
    }
    let (min_x, min_y, max_x, max_y) = dest_bounds(dest, bounds, transform_pivot, transform, clip);
    let sx = (f64::from(transform.scale_x) / 1000.0).max(0.001);
    let sy = (f64::from(transform.scale_y) / 1000.0).max(0.001);
    let theta = f64::from(transform.rotation_mdeg) * std::f64::consts::PI / 180_000.0;
    let (sin, cos) = theta.sin_cos();
    let cx = f64::from(transform_pivot.x) + f64::from(transform_pivot.width) / 2.0;
    let cy = f64::from(transform_pivot.y) + f64::from(transform_pivot.height) / 2.0;
    let tx = f64::from(transform.translate_x);
    let ty = f64::from(transform.translate_y);
    for y in min_y..max_y {
        for x in min_x..max_x {
            if let Some(clip) = clip
                && !clip.contains_point(x, y)
            {
                continue;
            }
            let dx = f64::from(x) - cx - tx;
            let dy = f64::from(y) - cy - ty;
            let rx = (cos * dx + sin * dy) / sx + cx;
            let ry = (-sin * dx + cos * dy) / sy + cy;
            let local_x = rx - f64::from(bounds.x);
            let local_y = ry - f64::from(bounds.y);
            if local_x < 0.0 || local_y < 0.0 {
                continue;
            }
            let u = local_x / f64::from(bounds.width.max(1));
            let v = local_y / f64::from(bounds.height.max(1));
            if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
                continue;
            }
            let src_px = if let Some(fill) = solid {
                [fill.r, fill.g, fill.b, fill.a]
            } else {
                let sx = stable_texel_coordinate(u * f64::from(src.width), src.width);
                let sy = stable_texel_coordinate(v * f64::from(src.height), src.height);
                src.pixel(sx, sy).unwrap_or([0, 0, 0, 0])
            };
            let a = (u16::from(src_px[3]) * u16::from(opacity) / 100) as u8;
            if a == 0 {
                continue;
            }
            let mut px = src_px;
            px[3] = a;
            match blend {
                BlendMode::SrcOver => src_over(dest, x as u32, y as u32, px),
                BlendMode::Multiply => multiply(dest, x as u32, y as u32, px),
            }
        }
    }
}

fn stable_texel_coordinate(value: f64, extent: u32) -> u32 {
    let rounded = value.round();
    let snapped = if (value - rounded).abs() <= 0.0001 {
        rounded
    } else {
        value
    };
    snapped.floor().clamp(0.0, f64::from(extent - 1)) as u32
}

fn dest_bounds(
    dest: &RawFrame,
    bounds: Rect,
    transform_pivot: Rect,
    transform: Transform,
    clip: Option<Rect>,
) -> (i32, i32, i32, i32) {
    let pad = ((transform.scale_x.abs().max(transform.scale_y.abs()) / 1000) + 2).max(2);
    let x0 = f64::from(bounds.x);
    let y0 = f64::from(bounds.y);
    let x1 = x0 + f64::from(bounds.width);
    let y1 = y0 + f64::from(bounds.height);
    let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
        .map(|(x, y)| transformed_point(transform_pivot, transform, x, y));
    let mut min_x = corners
        .iter()
        .map(|point| point.0)
        .fold(f64::INFINITY, f64::min)
        .floor() as i32
        - pad;
    let mut min_y = corners
        .iter()
        .map(|point| point.1)
        .fold(f64::INFINITY, f64::min)
        .floor() as i32
        - pad;
    let mut max_x = corners
        .iter()
        .map(|point| point.0)
        .fold(f64::NEG_INFINITY, f64::max)
        .ceil() as i32
        + pad;
    let mut max_y = corners
        .iter()
        .map(|point| point.1)
        .fold(f64::NEG_INFINITY, f64::max)
        .ceil() as i32
        + pad;
    min_x = min_x.max(0);
    min_y = min_y.max(0);
    max_x = max_x.min(i32::try_from(dest.width).unwrap_or(i32::MAX));
    max_y = max_y.min(i32::try_from(dest.height).unwrap_or(i32::MAX));
    if let Some(clip) = clip {
        min_x = min_x.max(clip.x);
        min_y = min_y.max(clip.y);
        max_x = max_x.min(clip.x + i32::try_from(clip.width).unwrap_or(0));
        max_y = max_y.min(clip.y + i32::try_from(clip.height).unwrap_or(0));
    }
    (min_x, min_y, max_x.max(min_x), max_y.max(min_y))
}

fn transformed_point(bounds: Rect, transform: Transform, x: f64, y: f64) -> (f64, f64) {
    let cx = f64::from(bounds.x) + f64::from(bounds.width) / 2.0;
    let cy = f64::from(bounds.y) + f64::from(bounds.height) / 2.0;
    let sx = (f64::from(transform.scale_x) / 1000.0).max(0.001);
    let sy = (f64::from(transform.scale_y) / 1000.0).max(0.001);
    let theta = f64::from(transform.rotation_mdeg) * std::f64::consts::PI / 180_000.0;
    let (sin, cos) = theta.sin_cos();
    let dx = (x - cx) * sx;
    let dy = (y - cy) * sy;
    (
        cos * dx - sin * dy + cx + f64::from(transform.translate_x),
        sin * dx + cos * dy + cy + f64::from(transform.translate_y),
    )
}

fn src_over(frame: &mut RawFrame, x: u32, y: u32, src: [u8; 4]) {
    let Some(dst) = frame.pixel(x, y) else {
        return;
    };
    let a = u16::from(src[3]);
    let inv = 255 - a;
    let blend = |s: u8, d: u8| ((u16::from(s) * a + u16::from(d) * inv) / 255) as u8;
    frame.set_pixel(
        x,
        y,
        [
            blend(src[0], dst[0]),
            blend(src[1], dst[1]),
            blend(src[2], dst[2]),
            255,
        ],
    );
}

fn multiply(frame: &mut RawFrame, x: u32, y: u32, src: [u8; 4]) {
    let Some(dst) = frame.pixel(x, y) else {
        return;
    };
    let mul = |s: u8, d: u8| ((u16::from(s) * u16::from(d)) / 255) as u8;
    let mixed = [
        mul(src[0], dst[0]),
        mul(src[1], dst[1]),
        mul(src[2], dst[2]),
        src[3],
    ];
    src_over(frame, x, y, mixed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    use lattice_core::{
        AssetRef, BlendMode, Canvas, MediaLocator, NodeProps, OverlayAlign, RenderScene, ShapeKind,
        TextNode, Time, TimeMap,
    };

    struct SolidDecoder {
        color: [u8; 4],
    }

    impl VideoDecoder for SolidDecoder {
        fn sample(
            &mut self,
            _asset: &lattice_core::AssetRef,
            _content_time: Time,
            width: u32,
            height: u32,
        ) -> Result<RawFrame, ExportError> {
            Ok(RawFrame::filled(
                width,
                height,
                self.color[0],
                self.color[1],
                self.color[2],
                self.color[3],
            ))
        }
    }

    fn shape_scene(opacity: u8, transform: Transform) -> RenderScene {
        RenderScene {
            canvas: Canvas {
                width: 32,
                height: 32,
            },
            nodes: vec![
                RenderNode::Video(VideoNode {
                    props: NodeProps::opaque(0),
                    bounds: Rect {
                        x: 0,
                        y: 0,
                        width: 32,
                        height: 32,
                    },
                    asset: AssetRef {
                        media_name: "v".into(),
                        locator: MediaLocator::File {
                            path: "v.mp4".into(),
                        },
                    },
                    content_time: Time::ZERO,
                    hold: false,
                    time_map: TimeMap::identity(Time::ZERO, Time::ONE),
                }),
                RenderNode::Shape(ShapeNode {
                    props: NodeProps {
                        transform,
                        opacity,
                        clip: None,
                        z: 1,
                        blend: BlendMode::SrcOver,
                    },
                    bounds: Rect {
                        x: 8,
                        y: 8,
                        width: 8,
                        height: 8,
                    },
                    kind: ShapeKind::Rectangle,
                    fill: Rgba::YELLOW,
                }),
            ],
        }
    }

    #[test]
    fn composites_video_and_rectangle() {
        let mut cpu = CpuCompositor::new(None);
        let mut decoder = SolidDecoder {
            color: [10, 20, 200, 255],
        };
        let frame = cpu
            .render(&shape_scene(100, Transform::IDENTITY), &mut decoder)
            .unwrap();
        let bg = frame.pixel(0, 0).unwrap();
        assert!(bg[2] > 150, "video blue {bg:?}");
        let yellow = frame.pixel(10, 10).unwrap();
        assert!(
            yellow[0] > 200 && yellow[1] > 180 && yellow[2] < 40,
            "{yellow:?}"
        );
    }

    #[test]
    fn opacity_blends_shape() {
        let mut cpu = CpuCompositor::new(None);
        let mut decoder = SolidDecoder {
            color: [0, 0, 0, 255],
        };
        let frame = cpu
            .render(&shape_scene(50, Transform::IDENTITY), &mut decoder)
            .unwrap();
        let px = frame.pixel(10, 10).unwrap();
        assert!(px[0] > 80 && px[0] < 200, "50% yellow on black {px:?}");
    }

    #[test]
    fn japanese_text_rasterizes_glyphs() {
        let font = crate::font::resolve_font(
            &lattice_core::FontSpec::preview_sans(18),
            Path::new("."),
            None,
            crate::font::fixture_font_path().as_deref(),
        );
        let Ok(font) = font else {
            eprintln!("skip japanese text: no fixture font");
            return;
        };
        let identity = font.identity.clone();
        let mut cpu = CpuCompositor::new(Some(font));
        let mut decoder = SolidDecoder {
            color: [0, 0, 0, 255],
        };
        let scene = RenderScene {
            canvas: Canvas {
                width: 64,
                height: 32,
            },
            nodes: vec![RenderNode::Text(TextNode {
                props: NodeProps::opaque(1),
                bounds: Rect {
                    x: 0,
                    y: 0,
                    width: 64,
                    height: 32,
                },
                text: "日本語".into(),
                font: lattice_core::FontSpec::preview_sans(18),
                resolved_font: Some(identity),
                color: Rgba::WHITE,
                align: OverlayAlign::Left,
            })],
        };
        let frame = cpu.render(&scene, &mut decoder).unwrap();
        let lit = frame
            .rgba
            .chunks_exact(4)
            .filter(|px| px[0] > 20 || px[1] > 20 || px[2] > 20)
            .count();
        assert!(lit > 20, "expected Japanese glyphs, lit={lit}");
    }

    #[test]
    fn node_local_text_keeps_the_canvas_transform_pivot() {
        let font = crate::font::resolve_font(
            &lattice_core::FontSpec::preview_sans(18),
            Path::new("."),
            None,
            crate::font::fixture_font_path().as_deref(),
        )
        .expect("fixture font");
        let identity = font.identity.clone();
        let mut cpu = CpuCompositor::new(Some(font));
        let canvas = Canvas {
            width: 64,
            height: 32,
        };
        let original = Rect {
            x: 4,
            y: 4,
            width: 24,
            height: 20,
        };
        let scene = RenderScene {
            canvas,
            nodes: vec![RenderNode::Text(TextNode {
                props: NodeProps {
                    transform: Transform {
                        rotation_mdeg: 180_000,
                        ..Transform::IDENTITY
                    },
                    ..NodeProps::opaque(1)
                },
                bounds: original,
                text: "Hi".into(),
                font: lattice_core::FontSpec::preview_sans(18),
                resolved_font: Some(identity),
                color: Rgba::WHITE,
                align: OverlayAlign::Left,
            })],
        };
        let frame = cpu
            .render(
                &scene,
                &mut SolidDecoder {
                    color: [0, 0, 0, 255],
                },
            )
            .unwrap();
        let lit_in = |rect: Rect| {
            (rect.y..rect.y + rect.height as i32)
                .flat_map(|y| (rect.x..rect.x + rect.width as i32).map(move |x| (x, y)))
                .filter(|(x, y)| {
                    frame
                        .pixel(*x as u32, *y as u32)
                        .is_some_and(|pixel| pixel[0] > 20)
                })
                .count()
        };
        let mirrored = Rect {
            x: 36,
            y: 8,
            width: 24,
            height: 20,
        };
        assert_eq!(
            lit_in(original),
            0,
            "rotation must not pivot around the node"
        );
        assert!(lit_in(mirrored) > 10, "text must rotate around the canvas");
    }

    #[test]
    fn clip_hides_shape_outside_rect() {
        let mut cpu = CpuCompositor::new(None);
        let mut decoder = SolidDecoder {
            color: [0, 0, 0, 255],
        };
        let scene = RenderScene {
            canvas: Canvas {
                width: 32,
                height: 32,
            },
            nodes: vec![RenderNode::Shape(ShapeNode {
                props: NodeProps {
                    transform: Transform::IDENTITY,
                    opacity: 100,
                    clip: Some(Rect {
                        x: 0,
                        y: 0,
                        width: 8,
                        height: 8,
                    }),
                    z: 1,
                    blend: BlendMode::SrcOver,
                },
                bounds: Rect {
                    x: 0,
                    y: 0,
                    width: 32,
                    height: 32,
                },
                kind: ShapeKind::Rectangle,
                fill: Rgba::YELLOW,
            })],
        };
        let frame = cpu.render(&scene, &mut decoder).unwrap();
        let inside = frame.pixel(2, 2).unwrap();
        let outside = frame.pixel(20, 20).unwrap();
        assert!(inside[0] > 200, "{inside:?}");
        assert!(outside[0] < 40, "{outside:?}");
    }

    #[test]
    fn translate_moves_shape() {
        let mut cpu = CpuCompositor::new(None);
        let mut decoder = SolidDecoder {
            color: [0, 0, 0, 255],
        };
        let frame = cpu
            .render(&shape_scene(100, Transform::translate(10, 0)), &mut decoder)
            .unwrap();
        let origin = frame.pixel(10, 10).unwrap();
        let moved = frame.pixel(20, 10).unwrap();
        assert!(origin[0] < 40, "old slot should be empty {origin:?}");
        assert!(moved[0] > 200, "translated yellow {moved:?}");
    }

    #[test]
    fn scale_expands_shape_bounds() {
        let mut cpu = CpuCompositor::new(None);
        let mut decoder = SolidDecoder {
            color: [0, 0, 0, 255],
        };
        let transform = Transform {
            scale_x: 2_000,
            scale_y: 2_000,
            ..Transform::IDENTITY
        };
        let frame = cpu
            .render(&shape_scene(100, transform), &mut decoder)
            .unwrap();
        assert!(frame.pixel(5, 10).unwrap()[0] > 200);
        assert!(frame.pixel(19, 10).unwrap()[0] > 200);
    }

    fn text_scene(text: &str, font: Option<lattice_core::FontIdentity>) -> RenderScene {
        RenderScene {
            canvas: Canvas {
                width: 64,
                height: 32,
            },
            nodes: vec![RenderNode::Text(TextNode {
                props: NodeProps::opaque(1),
                bounds: Rect {
                    x: 0,
                    y: 0,
                    width: 64,
                    height: 32,
                },
                text: text.into(),
                font: lattice_core::FontSpec::preview_sans(18),
                resolved_font: font,
                color: Rgba::WHITE,
                align: OverlayAlign::Left,
            })],
        }
    }

    #[test]
    fn missing_font_blocks_text_render() {
        let mut cpu = CpuCompositor::new(None);
        let mut decoder = SolidDecoder {
            color: [0, 0, 0, 255],
        };
        let err = cpu
            .render(&text_scene("Hello", None), &mut decoder)
            .unwrap_err();
        assert!(matches!(err, ExportError::MissingFont), "{err}");
    }

    #[test]
    fn missing_glyph_is_observable() {
        let font = crate::font::resolve_font(
            &lattice_core::FontSpec::preview_sans(18),
            Path::new("."),
            None,
            crate::font::fixture_font_path().as_deref(),
        )
        .expect("fixture font");
        let identity = font.identity.clone();
        let mut cpu = CpuCompositor::new(Some(font));
        let mut decoder = SolidDecoder {
            color: [0, 0, 0, 255],
        };
        let err = cpu
            .render(&text_scene("\u{10FFFF}", Some(identity)), &mut decoder)
            .unwrap_err();
        assert!(matches!(err, ExportError::MissingGlyph(_)), "{err}");
    }
}

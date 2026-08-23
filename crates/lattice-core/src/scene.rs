//! Backend-neutral per-frame scene graph. No wgpu, GPUI, or `FFmpeg` types.

use serde::{Deserialize, Serialize};

use crate::locator::MediaLocator;
use crate::property::{Interpolate, Property};
use crate::resolve::AssetIdentity;
use crate::time::Time;
use crate::time_map::TimeMap;

/// Output canvas in pixels. Coordinates are canvas space, not GPUI/wgpu pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
}

impl Canvas {
    pub const PREVIEW: Self = Self {
        width: 320,
        height: 180,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn from_canvas(canvas: Canvas) -> Self {
        Self {
            x: 0,
            y: 0,
            width: canvas.width,
            height: canvas.height,
        }
    }

    pub fn contains_point(self, x: i32, y: i32) -> bool {
        x >= self.x
            && y >= self.y
            && x < self
                .x
                .saturating_add(i32::try_from(self.width).unwrap_or(i32::MAX))
            && y < self
                .y
                .saturating_add(i32::try_from(self.height).unwrap_or(i32::MAX))
    }
}

/// 8-bit sRGB + alpha. Not a GPU format enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const YELLOW: Self = Self {
        r: 255,
        g: 255,
        b: 0,
        a: 255,
    };
    pub const CYAN: Self = Self {
        r: 0,
        g: 220,
        b: 255,
        a: 255,
    };

    /// Quoted overlay `color` / `bar` literal: `#RRGGBB` only.
    #[must_use]
    pub fn from_hex_rrggbb(text: &str) -> Option<Self> {
        let hex = text.strip_prefix('#')?;
        if hex.len() != 6 || !hex.as_bytes().iter().all(u8::is_ascii_hexdigit) {
            return None;
        }
        Some(Self {
            r: u8::from_str_radix(&hex[0..2], 16).ok()?,
            g: u8::from_str_radix(&hex[2..4], 16).ok()?,
            b: u8::from_str_radix(&hex[4..6], 16).ok()?,
            a: 255,
        })
    }

    #[must_use]
    pub fn to_hex_rrggbb(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

/// Translate in pixels, scale in millipercent (1000 = 1.0), rotation in millidegrees.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transform {
    pub translate_x: i32,
    pub translate_y: i32,
    pub scale_x: i32,
    pub scale_y: i32,
    pub rotation_mdeg: i32,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translate_x: 0,
        translate_y: 0,
        scale_x: 1000,
        scale_y: 1000,
        rotation_mdeg: 0,
    };

    pub fn translate(x: i32, y: i32) -> Self {
        Self {
            translate_x: x,
            translate_y: y,
            ..Self::IDENTITY
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Interpolate for Transform {
    fn interpolate(a: &Self, b: &Self, num: i64, den: i64) -> Self {
        Self {
            translate_x: i32::interpolate(&a.translate_x, &b.translate_x, num, den),
            translate_y: i32::interpolate(&a.translate_y, &b.translate_y, num, den),
            scale_x: i32::interpolate(&a.scale_x, &b.scale_x, num, den),
            scale_y: i32::interpolate(&a.scale_y, &b.scale_y, num, den),
            rotation_mdeg: i32::interpolate(&a.rotation_mdeg, &b.rotation_mdeg, num, den),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BlendMode {
    #[default]
    SrcOver,
    Multiply,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShapeKind {
    #[default]
    Rectangle,
    Ellipse,
}

/// Family/face intent. Concrete file identity is a Resolve concern.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontSpec {
    pub family: String,
    pub weight: u16,
    pub italic: bool,
    pub size_px: u32,
}

impl FontSpec {
    pub fn preview_sans(size_px: u32) -> Self {
        Self {
            family: "LatticeSans".into(),
            weight: 400,
            italic: false,
            size_px,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontSource {
    ProjectLocal,
    Lock,
    Fixture,
    System { portable: bool },
}

/// Resolved font file identity. Bytes stay out of Core IR.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontIdentity {
    pub path: String,
    pub face_index: u32,
    pub identity: AssetIdentity,
    pub source: FontSource,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRef {
    pub media_name: String,
    pub locator: MediaLocator,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeProps {
    pub transform: Transform,
    pub opacity: u8,
    pub clip: Option<Rect>,
    pub z: i32,
    pub blend: BlendMode,
}

impl NodeProps {
    pub fn opaque(z: i32) -> Self {
        Self {
            transform: Transform::IDENTITY,
            opacity: 100,
            clip: None,
            z,
            blend: BlendMode::SrcOver,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupNode {
    pub props: NodeProps,
    pub children: Vec<RenderNode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoNode {
    pub props: NodeProps,
    pub bounds: Rect,
    pub asset: AssetRef,
    pub content_time: Time,
    pub hold: bool,
    pub time_map: TimeMap,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageNode {
    pub props: NodeProps,
    pub bounds: Rect,
    pub asset: AssetRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextNode {
    pub props: NodeProps,
    pub bounds: Rect,
    pub text: String,
    pub font: FontSpec,
    pub resolved_font: Option<FontIdentity>,
    pub color: Rgba,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShapeNode {
    pub props: NodeProps,
    pub bounds: Rect,
    pub kind: ShapeKind,
    pub fill: Rgba,
}

/// Extension point. Compositor may skip an unknown mask.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaskNode {
    pub props: NodeProps,
    pub mask: Box<RenderNode>,
    pub content: Box<RenderNode>,
}

/// Extension point. Compositor may skip an unknown effect.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectNode {
    pub props: NodeProps,
    pub name: String,
    pub child: Box<RenderNode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RenderNode {
    Group(GroupNode),
    Video(VideoNode),
    Image(ImageNode),
    Text(TextNode),
    Shape(ShapeNode),
    Mask(MaskNode),
    Effect(EffectNode),
}

impl RenderNode {
    pub fn props(&self) -> &NodeProps {
        match self {
            Self::Group(node) => &node.props,
            Self::Video(node) => &node.props,
            Self::Image(node) => &node.props,
            Self::Text(node) => &node.props,
            Self::Shape(node) => &node.props,
            Self::Mask(node) => &node.props,
            Self::Effect(node) => &node.props,
        }
    }

    pub fn z(&self) -> i32 {
        self.props().z
    }
}

/// Immutable per-frame snapshot. Renderer must not mutate it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderScene {
    pub canvas: Canvas,
    pub nodes: Vec<RenderNode>,
}

impl RenderScene {
    pub fn sorted_nodes(&self) -> Vec<&RenderNode> {
        let mut nodes: Vec<&RenderNode> = self.nodes.iter().collect();
        nodes.sort_by_key(|node| node.z());
        nodes
    }

    pub fn has_text(&self) -> bool {
        nodes_have_text(&self.nodes)
    }
}

fn nodes_have_text(nodes: &[RenderNode]) -> bool {
    nodes.iter().any(|node| match node {
        RenderNode::Text(text) => !text.text.is_empty(),
        RenderNode::Group(group) => nodes_have_text(&group.children),
        RenderNode::Mask(mask) => {
            nodes_have_text(std::slice::from_ref(mask.content.as_ref()))
                || nodes_have_text(std::slice::from_ref(mask.mask.as_ref()))
        }
        RenderNode::Effect(effect) => nodes_have_text(std::slice::from_ref(effect.child.as_ref())),
        _ => false,
    })
}

/// Backend-neutral mix plan. Encoder receives already-mixed PCM.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioPlan {
    pub duration: Time,
    pub windows: Vec<AudioClip>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioClip {
    pub span: crate::ir::TimeSpan,
    pub gain_db: i32,
    pub generated: bool,
    pub asset: Option<AssetRef>,
    pub content_start: Time,
    pub hold: bool,
}

/// Timed transform/opacity before snapshot evaluation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimatedStyle {
    pub opacity: Property<u8>,
    pub translate_x: Property<i32>,
    pub translate_y: Property<i32>,
    pub scale_x: Property<i32>,
    pub scale_y: Property<i32>,
    pub rotation_mdeg: Property<i32>,
}

impl AnimatedStyle {
    pub fn static_opacity(opacity: u8) -> Self {
        Self {
            opacity: Property::Static(opacity),
            translate_x: Property::Static(0),
            translate_y: Property::Static(0),
            scale_x: Property::Static(1000),
            scale_y: Property::Static(1000),
            rotation_mdeg: Property::Static(0),
        }
    }

    pub fn snapshot(&self, local: Time) -> (Transform, u8) {
        (
            Transform {
                translate_x: self.translate_x.at(local),
                translate_y: self.translate_y.at(local),
                scale_x: self.scale_x.at(local),
                scale_y: self.scale_y.at(local),
                rotation_mdeg: self.rotation_mdeg.at(local),
            },
            self.opacity.at(local),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_json_has_no_gpu_tokens() {
        let scene = RenderScene {
            canvas: Canvas::PREVIEW,
            nodes: vec![RenderNode::Shape(ShapeNode {
                props: NodeProps::opaque(1),
                bounds: Rect::from_canvas(Canvas::PREVIEW),
                kind: ShapeKind::Rectangle,
                fill: Rgba::YELLOW,
            })],
        };
        let json = serde_json::to_string(&scene).expect("json");
        for forbidden in ["wgpu", "gpui", "ffmpeg", "filtergraph", "drawtext"] {
            assert!(
                !json.to_ascii_lowercase().contains(forbidden),
                "{forbidden} leaked into scene json: {json}"
            );
        }
    }

    #[test]
    fn z_order_sorts_back_to_front() {
        let scene = RenderScene {
            canvas: Canvas::PREVIEW,
            nodes: vec![
                RenderNode::Shape(ShapeNode {
                    props: NodeProps::opaque(5),
                    bounds: Rect {
                        x: 0,
                        y: 0,
                        width: 10,
                        height: 10,
                    },
                    kind: ShapeKind::Rectangle,
                    fill: Rgba::WHITE,
                }),
                RenderNode::Shape(ShapeNode {
                    props: NodeProps::opaque(1),
                    bounds: Rect {
                        x: 0,
                        y: 0,
                        width: 10,
                        height: 10,
                    },
                    kind: ShapeKind::Rectangle,
                    fill: Rgba::BLACK,
                }),
            ],
        };
        let zs: Vec<i32> = scene.sorted_nodes().iter().map(|n| n.z()).collect();
        assert_eq!(zs, vec![1, 5]);
    }

    #[test]
    fn animated_style_midpoint() {
        use crate::property::{Curve, Easing, Keyframe, Property};
        let style = AnimatedStyle {
            opacity: Property::Animated(Curve {
                keyframes: vec![
                    Keyframe {
                        time: Time::ZERO,
                        value: 0,
                        easing: Easing::Linear,
                    },
                    Keyframe {
                        time: Time::seconds(2),
                        value: 100,
                        easing: Easing::Linear,
                    },
                ],
            }),
            translate_x: Property::Static(0),
            translate_y: Property::Static(0),
            scale_x: Property::Static(1000),
            scale_y: Property::Static(1000),
            rotation_mdeg: Property::Static(0),
        };
        let (_, opacity) = style.snapshot(Time::seconds(1));
        assert_eq!(opacity, 50);
    }

    #[test]
    fn hex_rrggbb_is_quoted_six_digit_only() {
        assert_eq!(
            Rgba::from_hex_rrggbb("#00FF00"),
            Some(Rgba {
                r: 0,
                g: 255,
                b: 0,
                a: 255
            })
        );
        assert_eq!(
            Rgba::from_hex_rrggbb("#00ff00").unwrap().to_hex_rrggbb(),
            "#00FF00"
        );
        assert!(Rgba::from_hex_rrggbb("green").is_none());
        assert!(Rgba::from_hex_rrggbb("00FF00").is_none());
        assert!(Rgba::from_hex_rrggbb("#FFF").is_none());
        assert!(Rgba::from_hex_rrggbb("#00FF00AA").is_none());
        assert!(Rgba::from_hex_rrggbb("#GG0000").is_none());
    }
}

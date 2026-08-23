use serde::{Deserialize, Serialize};

use crate::scene::Rgba;

/// In-box text alignment for overlay `TextNode`s (CHI-90).
///
/// This is **not** a Visual.position move: evaluate leaves the overlay
/// group transform alone and raster aligns inside `TextNode.bounds`.
/// Omitted body word => `None` here; evaluate defaults to [`OverlayAlign::Left`].
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OverlayAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Scale / placement pivot for overlay evaluate (CHI-91).
///
/// Named chair points only — no px and no second coordinate space.
/// `None` on Visual/TimelineClip means today's top-left scale pivot.
/// Never named `origin` (provenance clash); do not alias `origin`.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OverlayAnchor {
    #[default]
    TopLeft,
    TopRight,
    Center,
    BottomLeft,
    BottomRight,
}

impl OverlayAnchor {
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "top-left" => Self::TopLeft,
            "top-right" => Self::TopRight,
            "center" => Self::Center,
            "bottom-left" => Self::BottomLeft,
            "bottom-right" => Self::BottomRight,
            _ => return None,
        })
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TopLeft => "top-left",
            Self::TopRight => "top-right",
            Self::Center => "center",
            Self::BottomLeft => "bottom-left",
            Self::BottomRight => "bottom-right",
        }
    }

    /// Pixel scale-about point in overlay-local canvas space (`base_y` is the
    /// unscaled box top). Evaluate owns this; it is not a public Origin API.
    #[must_use]
    pub fn scale_pivot(self, overlay_width: u32, overlay_height: u32, base_y: i32) -> (i32, i32) {
        let w = i32::try_from(overlay_width).unwrap_or(i32::MAX);
        let h = i32::try_from(overlay_height).unwrap_or(i32::MAX);
        let (dx, dy) = match self {
            Self::TopLeft => (0, 0),
            Self::TopRight => (w, 0),
            Self::Center => (w / 2, h / 2),
            Self::BottomLeft => (0, h),
            Self::BottomRight => (w, h),
        };
        (dx, base_y.saturating_add(dy))
    }

    #[must_use]
    pub fn places_scaled_top_left(self) -> bool {
        matches!(self, Self::TopLeft)
    }
}

/// Explicit overlay typeface / color / bar / align. Absent fields fall through
/// explicit > preset > convention > default at evaluate.
///
/// `align` is a CHI-90 body word — not part of the CHI-87 color/size/weight/
/// family/bar list.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlayStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Rgba>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<OverlaySize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar: Option<OverlayBar>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<OverlayAlign>,
}

impl OverlayStyle {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.color.is_none()
            && self.size.is_none()
            && self.weight.is_none()
            && self.family.is_none()
            && self.bar.is_none()
            && self.align.is_none()
    }

    #[must_use]
    pub fn into_option(self) -> Option<Self> {
        if self.is_empty() { None } else { Some(self) }
    }
}

/// Type size as a convention ratio or an explicit pixel lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum OverlaySize {
    /// Thousandths of the convention size (`1000` = `100%`).
    Percent {
        milli: u16,
    },
    Px {
        px: u32,
    },
}

/// Overlay bar fill. `Off` omits the shape; omitted style keeps the convention color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum OverlayBar {
    Off,
    Fill { color: Rgba },
}

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

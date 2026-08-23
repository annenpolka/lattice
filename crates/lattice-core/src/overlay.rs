use serde::{Deserialize, Serialize};

use crate::scene::Rgba;

/// Explicit overlay typeface / color / bar. Absent fields fall through
/// explicit > preset > convention > default at evaluate.
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
}

impl OverlayStyle {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.color.is_none()
            && self.size.is_none()
            && self.weight.is_none()
            && self.family.is_none()
            && self.bar.is_none()
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

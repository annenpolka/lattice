//! Overlay-body lowering for `title` / `callout`.
//!
//! Shared by in-process builtins and the Wasm host so invalid `position` /
//! `scale` / style and unknown body words produce the same diagnostics.

use std::fmt::Write as _;

use lattice_core::{
    Diagnostic, NormalizedPosition, NormalizedScale, OverlayAlign, OverlayAnchor, OverlayBar,
    OverlaySize, OverlayStyle, Rgba,
};

use crate::view::{BodyItem, InvocationView, SceneDraft, ValueView};

/// Overlay body invocations that already have a lowering, plus `at`/`for`.
/// CHI-91 added `anchor` (placement pivot — not typeface).
const OVERLAY_BODY_ALLOWLIST: &[&str] = &[
    "opacity", "position", "scale", "anchor", "at", "for", "color", "size", "weight", "family",
    "bar", "align",
];
/// Generic parser modifiers that are already consumed as timing. Others
/// (`over` / `using` / `by` / `from` / `to`) must not silent-drop.
const OVERLAY_MODIFIER_ALLOWLIST: &[&str] = &["at", "for"];

pub const INVALID_POSITION: &str = "LAT-OVL-001";
pub const INVALID_SCALE: &str = "LAT-OVL-002";
pub const UNKNOWN_BODY_WORD: &str = "LAT-OVL-003";
pub const INVALID_COLOR: &str = "LAT-OVL-004";
pub const INVALID_SIZE: &str = "LAT-OVL-005";
pub const INVALID_WEIGHT: &str = "LAT-OVL-006";
pub const INVALID_FAMILY: &str = "LAT-OVL-007";
pub const INVALID_BAR: &str = "LAT-OVL-008";
pub const INVALID_ALIGN: &str = "LAT-OVL-009";
pub const INVALID_ANCHOR: &str = "LAT-OVL-010";

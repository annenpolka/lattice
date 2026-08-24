//! Runtime IDENT → [`OverlayStyle`] map for `using`.
//!
//! Core never stores a preset name. Lookup priority among registered styles:
//! DSL > wasm > builtin. Explicit title-body fields still win over the result.

use std::collections::BTreeMap;

use lattice_core::{OverlayBar, OverlaySize, OverlayStyle, Rgba};

/// v0 title preset IDENT. Not a parser word and not a Core kind.
pub const LOWER_THIRD: &str = "lower-third";

/// Compact name-plate size vs bare title convention (`100%` of `height/16`).
pub const LOWER_THIRD_SIZE_MILLI: u16 = 900;

/// Convention family, filled so the expansion is visible on Core `OverlayStyle`.
pub const LOWER_THIRD_FAMILY: &str = "LatticeSans";

/// Where an IDENT was registered. Same-layer redefinition is a diag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayPresetSource {
    Dsl,
    Wasm,
    Builtin,
}

impl OverlayPresetSource {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dsl => "dsl",
            Self::Wasm => "wasm",
            Self::Builtin => "builtin",
        }
    }
}

/// Layered title-preset registry. Empty [`Default`] has no entries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OverlayPresetRegistry {
    dsl: BTreeMap<String, OverlayStyle>,
    wasm: BTreeMap<String, OverlayStyle>,
    builtin: BTreeMap<String, OverlayStyle>,
}

impl OverlayPresetRegistry {
    /// Builtin `lower-third` only. Used when wasm does not load and as fallback.
    #[must_use]
    pub fn builtin() -> Self {
        let mut registry = Self::default();
        registry
            .register(
                LOWER_THIRD,
                lower_third_style(),
                OverlayPresetSource::Builtin,
            )
            .expect("empty registry accepts the builtin IDENT");
        registry
    }

    /// Register `name` in `source`. Same-layer redefinition returns the IDENT.
    pub fn register(
        &mut self,
        name: impl Into<String>,
        style: OverlayStyle,
        source: OverlayPresetSource,
    ) -> Result<(), String> {
        let name = name.into();
        let layer = match source {
            OverlayPresetSource::Dsl => &mut self.dsl,
            OverlayPresetSource::Wasm => &mut self.wasm,
            OverlayPresetSource::Builtin => &mut self.builtin,
        };
        if layer.contains_key(&name) {
            return Err(name);
        }
        layer.insert(name, style);
        Ok(())
    }

    /// DSL > wasm > builtin.
    #[must_use]
    pub fn lookup(&self, name: &str) -> Option<&OverlayStyle> {
        self.lookup_entry(name).map(|(style, _)| style)
    }

    /// Style plus the layer that won. Core still never sees the IDENT.
    #[must_use]
    pub fn lookup_entry(&self, name: &str) -> Option<(&OverlayStyle, OverlayPresetSource)> {
        if let Some(style) = self.dsl.get(name) {
            return Some((style, OverlayPresetSource::Dsl));
        }
        if let Some(style) = self.wasm.get(name) {
            return Some((style, OverlayPresetSource::Wasm));
        }
        self.builtin
            .get(name)
            .map(|style| (style, OverlayPresetSource::Builtin))
    }

    /// Winning layer strictly below `source` (used for shadow explain).
    #[must_use]
    pub fn winning_below(
        &self,
        name: &str,
        source: OverlayPresetSource,
    ) -> Option<OverlayPresetSource> {
        match source {
            OverlayPresetSource::Dsl => self
                .wasm
                .contains_key(name)
                .then_some(OverlayPresetSource::Wasm)
                .or_else(|| {
                    self.builtin
                        .contains_key(name)
                        .then_some(OverlayPresetSource::Builtin)
                }),
            OverlayPresetSource::Wasm => self
                .builtin
                .contains_key(name)
                .then_some(OverlayPresetSource::Builtin),
            OverlayPresetSource::Builtin => None,
        }
    }

    #[must_use]
    pub fn known_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .dsl
            .keys()
            .chain(self.wasm.keys())
            .chain(self.builtin.keys())
            .cloned()
            .collect();
        names.sort();
        names.dedup();
        names
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.dsl.is_empty() && self.wasm.is_empty() && self.builtin.is_empty()
    }

    #[must_use]
    pub fn contains_in(&self, name: &str, source: OverlayPresetSource) -> bool {
        match source {
            OverlayPresetSource::Dsl => self.dsl.contains_key(name),
            OverlayPresetSource::Wasm => self.wasm.contains_key(name),
            OverlayPresetSource::Builtin => self.builtin.contains_key(name),
        }
    }
}

/// v0 `lower-third` expansion for title.
///
/// Still [`lattice_core::PlacementKind::Title`] + one `TextNode`. Does not set
/// `position` / `scale` / `anchor`: title evaluate already places the yellow
/// bar at the bottom of the canvas.
///
/// Fills omitted [`OverlayStyle`] fields only (explicit body wins):
/// - `bar`: Fill `#FFFF00` (title convention yellow, explicit bar-on)
/// - `size`: `90%` of title convention (`height/16`) — documented compact delta
///   vs a bare title, which leaves `style` empty and evaluates at `100%`
/// - `family`: `LatticeSans` (same family as evaluate convention, explicit)
#[must_use]
pub fn lower_third_style() -> OverlayStyle {
    OverlayStyle {
        size: Some(OverlaySize::Percent {
            milli: LOWER_THIRD_SIZE_MILLI,
        }),
        family: Some(LOWER_THIRD_FAMILY.into()),
        bar: Some(OverlayBar::Fill {
            color: Rgba::YELLOW,
        }),
        ..OverlayStyle::default()
    }
}

/// Merge preset under explicit body fields. Does not touch geometry.
#[must_use]
pub fn merge_explicit_over_preset(explicit: OverlayStyle, preset: OverlayStyle) -> OverlayStyle {
    OverlayStyle {
        color: explicit.color.or(preset.color),
        size: explicit.size.or(preset.size),
        weight: explicit.weight.or(preset.weight),
        family: explicit.family.or(preset.family),
        bar: explicit.bar.or(preset.bar),
        align: explicit.align.or(preset.align),
    }
}

//! v0 overlay presets (`using IDENT`). Title only.
//!
//! The VEL parser keeps `using` generic. This registry owns the IDENT map,
//! expansion, and priority. Core sees the filled [`OverlayStyle`] only — never
//! a preset name.

use lattice_core::{Diagnostic, OverlayBar, OverlaySize, OverlayStyle, Rgba};

use crate::view::{InvocationView, SceneDraft, ValueView};

/// Unknown or inapplicable `using IDENT`.
pub const UNKNOWN_PRESET: &str = "LAT-OVL-013";

/// v0 title preset IDENT. Not a parser word and not a Core kind.
pub const LOWER_THIRD: &str = "lower-third";

/// Compact name-plate size vs bare title convention (`100%` of `height/16`).
pub const LOWER_THIRD_SIZE_MILLI: u16 = 900;

/// Convention family, filled so the expansion is visible on Core `OverlayStyle`.
pub const LOWER_THIRD_FAMILY: &str = "LatticeSans";

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
///
/// Priority: explicit body > this preset > evaluate convention > default.
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

/// Title-only IDENT map. Unknown names stay `None`.
#[must_use]
pub fn title_preset_style(name: &str) -> Option<OverlayStyle> {
    match name {
        LOWER_THIRD => Some(lower_third_style()),
        _ => None,
    }
}

/// First invocation-level `using` value. Body `using` stays an unknown word.
#[must_use]
pub fn invocation_using(inv: &InvocationView) -> Option<&ValueView> {
    inv.modifiers
        .iter()
        .find(|(name, _)| name == "using")
        .map(|(_, value)| value)
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

/// Consume invocation-level `using IDENT` for overlays.
///
/// `title` applies [`LOWER_THIRD`]. Any other IDENT, a non-ident value, or
/// `using` on callout/caption is [`UNKNOWN_PRESET`]. Returns the applied IDENT
/// when the preset was merged.
pub fn apply_using_preset(
    inv: &InvocationView,
    draft: &mut SceneDraft,
    applies_to_title: bool,
    style: &mut OverlayStyle,
) -> Option<&'static str> {
    let value = invocation_using(inv)?;
    let Some(name) = value.as_name() else {
        draft.diagnostics.push(Diagnostic::error(
            UNKNOWN_PRESET,
            "invalid overlay preset; expected `using` identifier",
            Some(inv.span),
        ));
        return None;
    };
    if !applies_to_title {
        draft.diagnostics.push(Diagnostic::error(
            UNKNOWN_PRESET,
            format!("overlay preset `{name}` applies to title only"),
            Some(inv.span),
        ));
        return None;
    }
    let Some(preset) = title_preset_style(name) else {
        draft.diagnostics.push(Diagnostic::error(
            UNKNOWN_PRESET,
            format!("unknown overlay preset `{name}`; v0 title presets: `{LOWER_THIRD}`"),
            Some(inv.span),
        ));
        return None;
    };
    let explicit = std::mem::take(style);
    *style = merge_explicit_over_preset(explicit, preset);
    // v0 registers only `lower-third`; `title_preset_style` already matched it.
    Some(LOWER_THIRD)
}

#[cfg(test)]
mod tests {
    use lattice_core::{PlacementKind, Severity, Span, Visual};

    use super::*;
    use crate::overlay_body::{
        OverlayBody, OverlayConvention, UNKNOWN_BODY_WORD, parse_overlay_body,
        parse_overlay_body_for,
    };
    use crate::registry::LoweringRegistry;
    use crate::view::{BodyItem, InvocationView, SceneDraft, ValueView};

    fn span() -> Span {
        Span::new(0, 1, 1, 1)
    }

    fn draft() -> SceneDraft {
        SceneDraft {
            name: "intro".into(),
            over: None,
            sources: Vec::new(),
            placements: Vec::new(),
            media: Vec::new(),
            source_fade_in: Vec::new(),
            source_gain_db: Vec::new(),
            explain: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn title_using(ident: &str, body: Vec<BodyItem>) -> InvocationView {
        InvocationView {
            command: "title".into(),
            args: vec![ValueView::String("Ada Lovelace\nEditor".into())],
            modifiers: vec![("using".into(), ValueView::Name(ident.into()))],
            body,
            span: span(),
        }
    }

    fn callout_using(ident: &str) -> InvocationView {
        InvocationView {
            command: "callout".into(),
            args: vec![ValueView::String("Hold".into())],
            modifiers: vec![("using".into(), ValueView::Name(ident.into()))],
            body: Vec::new(),
            span: span(),
        }
    }

    fn body_command(command: &str, args: Vec<ValueView>) -> BodyItem {
        BodyItem::Invocation(InvocationView {
            command: command.into(),
            args,
            modifiers: Vec::new(),
            body: Vec::new(),
            span: span(),
        })
    }

    fn quoted(text: &str) -> ValueView {
        ValueView::String(text.into())
    }

    fn percent(digits: i64) -> ValueView {
        ValueView::Quantity {
            negative: false,
            digits,
            scale: 0,
            unit: Some("%".into()),
        }
    }

    fn overlay_visual(draft: &SceneDraft) -> &Visual {
        draft
            .placements
            .iter()
            .find_map(|placement| placement.visual.as_ref())
            .expect("overlay visual")
    }

    #[test]
    fn registry_maps_lower_third_on_title_only() {
        assert!(title_preset_style(LOWER_THIRD).is_some());
        assert!(title_preset_style("upper-third").is_none());
        assert!(title_preset_style("callout").is_none());
    }

    #[test]
    fn lower_third_fills_omitted_style_without_preset_name() {
        let inv = title_using(LOWER_THIRD, Vec::new());
        let mut draft = draft();
        let parsed = parse_overlay_body(&inv, &mut draft);
        assert!(draft.diagnostics.is_empty(), "{:?}", draft.diagnostics);
        assert_eq!(parsed.applied_preset, Some(LOWER_THIRD));
        assert_eq!(
            parsed.style.bar,
            Some(OverlayBar::Fill {
                color: Rgba::YELLOW
            })
        );
        assert_eq!(
            parsed.style.size,
            Some(OverlaySize::Percent {
                milli: LOWER_THIRD_SIZE_MILLI
            })
        );
        assert_eq!(parsed.style.family.as_deref(), Some(LOWER_THIRD_FAMILY));
        assert!(parsed.style.color.is_none());
        assert!(parsed.style.weight.is_none());
        assert!(parsed.style.align.is_none());
        assert!(parsed.position.is_none());
        assert!(parsed.scale.is_none());
        assert!(parsed.anchor.is_none());
        let dumped = format!("{:?}", parsed.style);
        assert!(
            !dumped.contains(LOWER_THIRD),
            "Core style must not store the IDENT: {dumped}"
        );
    }

    #[test]
    fn explicit_body_wins_over_preset() {
        let inv = title_using(
            LOWER_THIRD,
            vec![
                body_command("color", vec![quoted("#00FF00")]),
                body_command("size", vec![percent(50)]),
                body_command("bar", vec![ValueView::Name("off".into())]),
                body_command("family", vec![quoted("OtherSans")]),
            ],
        );
        let mut draft = draft();
        let parsed = parse_overlay_body(&inv, &mut draft);
        assert!(draft.diagnostics.is_empty(), "{:?}", draft.diagnostics);
        assert_eq!(parsed.style.color, Rgba::from_hex_rrggbb("#00FF00"));
        assert_eq!(parsed.style.size, Some(OverlaySize::Percent { milli: 500 }));
        assert_eq!(parsed.style.bar, Some(OverlayBar::Off));
        assert_eq!(parsed.style.family.as_deref(), Some("OtherSans"));
    }

    #[test]
    fn unknown_ident_is_lowering_diag_not_silent() {
        let inv = title_using("upper-third", Vec::new());
        let mut draft = draft();
        let parsed = parse_overlay_body(&inv, &mut draft);
        assert!(
            draft.diagnostics.iter().any(|diag| {
                diag.code == UNKNOWN_PRESET
                    && diag.severity == Severity::Error
                    && diag.message.contains("`upper-third`")
                    && diag.message.contains(LOWER_THIRD)
            }),
            "{:?}",
            draft.diagnostics
        );
        assert!(parsed.style.is_empty());
        assert_eq!(parsed.applied_preset, None);
    }

    #[test]
    fn non_ident_using_value_diags() {
        let inv = InvocationView {
            command: "title".into(),
            args: vec![ValueView::String("Hello".into())],
            modifiers: vec![("using".into(), ValueView::String(LOWER_THIRD.into()))],
            body: Vec::new(),
            span: span(),
        };
        let mut draft = draft();
        parse_overlay_body(&inv, &mut draft);
        assert!(
            draft
                .diagnostics
                .iter()
                .any(|diag| diag.code == UNKNOWN_PRESET && diag.message.contains("identifier")),
            "{:?}",
            draft.diagnostics
        );
    }

    #[test]
    fn callout_using_lower_third_diags_and_does_not_expand() {
        let inv = callout_using(LOWER_THIRD);
        let mut draft = draft();
        let parsed = parse_overlay_body_for(&inv, &mut draft, OverlayConvention::Callout);
        assert!(
            draft.diagnostics.iter().any(|diag| {
                diag.code == UNKNOWN_PRESET
                    && diag.message.contains(LOWER_THIRD)
                    && diag.message.contains("title only")
            }),
            "{:?}",
            draft.diagnostics
        );
        assert!(parsed.style.is_empty());
        assert_eq!(parsed.applied_preset, None);
    }

    #[test]
    fn body_using_stays_unknown_body_word() {
        let inv = InvocationView {
            command: "title".into(),
            args: vec![ValueView::String("Hello".into())],
            modifiers: Vec::new(),
            body: vec![BodyItem::Modifier {
                name: "using".into(),
                value: ValueView::Name(LOWER_THIRD.into()),
            }],
            span: span(),
        };
        let mut draft = draft();
        let parsed = parse_overlay_body(&inv, &mut draft);
        assert!(
            draft
                .diagnostics
                .iter()
                .any(|diag| { diag.code == UNKNOWN_BODY_WORD && diag.message.contains("`using`") }),
            "{:?}",
            draft.diagnostics
        );
        assert!(
            !draft
                .diagnostics
                .iter()
                .any(|diag| diag.code == UNKNOWN_PRESET)
        );
        assert!(parsed.style.is_empty());
        assert_eq!(parsed.applied_preset, None);
    }

    #[test]
    fn registry_title_lower_third_writes_title_visual_without_preset_string() {
        let registry = LoweringRegistry::stdlib();
        let inv = title_using(LOWER_THIRD, Vec::new());
        let mut draft = draft();
        registry.lower(&inv, &mut draft).unwrap();
        assert!(draft.diagnostics.is_empty(), "{:?}", draft.diagnostics);
        let placement = draft
            .placements
            .iter()
            .find(|p| p.kind == PlacementKind::Title);
        let placement = placement.expect("title placement");
        let visual = placement.visual.as_ref().expect("visual");
        let style = visual.style.as_ref().expect("expanded style");
        assert_eq!(
            style.bar,
            Some(OverlayBar::Fill {
                color: Rgba::YELLOW
            })
        );
        assert_eq!(
            style.size,
            Some(OverlaySize::Percent {
                milli: LOWER_THIRD_SIZE_MILLI
            })
        );
        let dumped = format!("{visual:?}");
        assert!(
            !dumped.contains(LOWER_THIRD),
            "Visual must not store the IDENT: {dumped}"
        );
        assert!(
            draft
                .explain
                .iter()
                .any(|line| line.message.contains("using lower-third")),
            "{:?}",
            draft.explain
        );
    }

    #[test]
    fn registry_callout_using_lower_third_diags() {
        let registry = LoweringRegistry::stdlib();
        let inv = callout_using(LOWER_THIRD);
        let mut draft = draft();
        registry.lower(&inv, &mut draft).unwrap();
        assert!(
            draft
                .diagnostics
                .iter()
                .any(|diag| diag.code == UNKNOWN_PRESET && diag.message.contains("title only")),
            "{:?}",
            draft.diagnostics
        );
        let visual = overlay_visual(&draft);
        assert!(visual.style.as_ref().is_none_or(OverlayStyle::is_empty));
    }

    #[test]
    fn merge_keeps_explicit_and_fills_omitted() {
        let explicit = OverlayStyle {
            color: Rgba::from_hex_rrggbb("#00FF00"),
            ..OverlayStyle::default()
        };
        let merged = merge_explicit_over_preset(explicit, lower_third_style());
        assert_eq!(merged.color, Rgba::from_hex_rrggbb("#00FF00"));
        assert_eq!(
            merged.size,
            Some(OverlaySize::Percent {
                milli: LOWER_THIRD_SIZE_MILLI
            })
        );
        assert_eq!(
            merged.bar,
            Some(OverlayBar::Fill {
                color: Rgba::YELLOW
            })
        );
    }

    #[test]
    fn overlay_body_default_has_no_preset() {
        assert_eq!(OverlayBody::default().applied_preset, None);
    }
}

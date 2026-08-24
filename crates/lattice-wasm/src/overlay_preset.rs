//! v0 overlay presets (`using IDENT`). Title only.
//!
//! The VEL parser keeps `using` generic. This module expands IDENT through
//! [`OverlayPresetRegistry`] and DSL `overlay-preset`. Core sees the filled
//! [`OverlayStyle`] only — never a preset name.

use lattice_core::{Diagnostic, OverlayStyle};

use crate::overlay_body::{UNKNOWN_BODY_WORD, parse_overlay_style_fields};
use crate::overlay_registry::{
    LOWER_THIRD, OverlayPresetRegistry, OverlayPresetSource, merge_explicit_over_preset,
};
use crate::view::{BodyItem, ExplainLine, InvocationView, SceneDraft, ValueView};

/// Unknown or inapplicable `using IDENT`.
pub const UNKNOWN_PRESET: &str = "LAT-OVL-013";

/// Same-layer redefinition of an overlay-preset IDENT.
pub const REDEFINED_PRESET: &str = "LAT-OVL-014";

/// Invalid `overlay-preset` (missing IDENT, geometry in v0 body).
pub const INVALID_PRESET: &str = "LAT-OVL-015";

const PRESET_STYLE_WORDS: &[&str] = &["color", "size", "weight", "family", "bar", "align"];
const PRESET_GEOMETRY_WORDS: &[&str] = &["position", "scale", "anchor"];

/// First invocation-level `using` value. Body `using` stays an unknown word.
#[must_use]
pub fn invocation_using(inv: &InvocationView) -> Option<&ValueView> {
    inv.modifiers
        .iter()
        .find(|(name, _)| name == "using")
        .map(|(_, value)| value)
}

/// Consume invocation-level `using IDENT` for overlays.
///
/// `title` looks up the draft registry. Any unknown IDENT, a non-ident value,
/// or `using` on callout/caption is [`UNKNOWN_PRESET`]. Returns the applied
/// IDENT when a preset was merged.
pub fn apply_using_preset(
    inv: &InvocationView,
    draft: &mut SceneDraft,
    applies_to_title: bool,
    style: &mut OverlayStyle,
) -> Option<String> {
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
    let Some(preset) = draft.overlay_presets.title_preset_style(name) else {
        let known = draft.overlay_presets.known_names();
        let listed = if known.is_empty() {
            LOWER_THIRD.to_string()
        } else {
            known
                .iter()
                .map(|ident| format!("`{ident}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        draft.diagnostics.push(Diagnostic::error(
            UNKNOWN_PRESET,
            format!("unknown overlay preset `{name}`; v0 title presets: {listed}"),
            Some(inv.span),
        ));
        return None;
    };
    let explicit = std::mem::take(style);
    *style = merge_explicit_over_preset(explicit, preset);
    Some(name.to_string())
}

/// Register a document-scope `overlay-preset IDENT { style fields }`.
///
/// Body may use existing overlay style words only. Geometry is a v0 diag.
/// Same-layer redefinition is [`REDEFINED_PRESET`] and does not overwrite.
pub fn register_dsl_preset(
    inv: &InvocationView,
    presets: &mut OverlayPresetRegistry,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ExplainLine> {
    let Some(name) = inv.args.first().and_then(ValueView::as_name) else {
        diagnostics.push(Diagnostic::error(
            INVALID_PRESET,
            "overlay-preset needs an identifier",
            Some(inv.span),
        ));
        return None;
    };
    if inv.args.len() > 1 {
        diagnostics.push(Diagnostic::error(
            INVALID_PRESET,
            format!("overlay-preset `{name}` takes a single identifier"),
            Some(inv.span),
        ));
        return None;
    }
    diagnose_preset_body(inv, name, diagnostics);
    let mut scratch = SceneDraft {
        overlay_presets: OverlayPresetRegistry::default(),
        ..SceneDraft::default()
    };
    let style = parse_overlay_style_fields(inv, &mut scratch);
    diagnostics.append(&mut scratch.diagnostics);
    if let Err(existing) = presets.register(name, style, OverlayPresetSource::Dsl) {
        diagnostics.push(Diagnostic::error(
            REDEFINED_PRESET,
            format!("overlay-preset `{existing}` is already defined"),
            Some(inv.span),
        ));
        return None;
    }
    Some(ExplainLine {
        origin: lattice_core::Origin::Invocation {
            command: "overlay-preset".into(),
        },
        message: format!("overlay-preset `{name}` registered"),
    })
}

fn diagnose_preset_body(inv: &InvocationView, name: &str, diagnostics: &mut Vec<Diagnostic>) {
    for item in &inv.body {
        match item {
            BodyItem::Invocation(inner) => {
                if PRESET_STYLE_WORDS.contains(&inner.command.as_str()) {
                    continue;
                }
                if PRESET_GEOMETRY_WORDS.contains(&inner.command.as_str()) {
                    diagnostics.push(Diagnostic::error(
                        INVALID_PRESET,
                        format!(
                            "overlay-preset `{name}` does not take `{}` in v0",
                            inner.command
                        ),
                        Some(inner.span),
                    ));
                    continue;
                }
                diagnostics.push(Diagnostic::error(
                    UNKNOWN_BODY_WORD,
                    format!("unknown overlay body word `{}`", inner.command),
                    Some(inner.span),
                ));
            }
            BodyItem::Modifier { name: word, .. } => {
                diagnostics.push(Diagnostic::error(
                    UNKNOWN_BODY_WORD,
                    format!("unknown overlay body word `{word}`"),
                    Some(inv.span),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use lattice_core::{OverlayBar, OverlaySize, PlacementKind, Rgba, Severity, Span, Visual};

    use super::*;
    use crate::overlay_body::{
        OverlayBody, OverlayConvention, UNKNOWN_BODY_WORD, parse_overlay_body,
        parse_overlay_body_for,
    };
    use crate::overlay_registry::{
        LOWER_THIRD_FAMILY, LOWER_THIRD_SIZE_MILLI, lower_third_style, title_preset_style,
    };
    use crate::registry::LoweringRegistry;
    use crate::view::{BodyItem, InvocationView, SceneDraft, ValueView};

    fn span() -> Span {
        Span::new(0, 1, 1, 1)
    }

    fn draft() -> SceneDraft {
        SceneDraft {
            name: "intro".into(),
            ..SceneDraft::default()
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

    fn preset_inv(name: &str, body: Vec<BodyItem>) -> InvocationView {
        InvocationView {
            command: "overlay-preset".into(),
            args: vec![ValueView::Name(name.into())],
            modifiers: Vec::new(),
            body,
            span: span(),
        }
    }

    #[test]
    fn registry_maps_lower_third_on_title_only() {
        assert!(title_preset_style(LOWER_THIRD).is_some());
        assert!(title_preset_style("upper-third").is_none());
        assert!(title_preset_style("callout").is_none());
    }

    #[test]
    fn builtin_is_an_entry_in_the_same_registry() {
        let registry = OverlayPresetRegistry::builtin();
        assert_eq!(registry.lookup(LOWER_THIRD), Some(&lower_third_style()));
        assert_eq!(registry.known_names(), vec![LOWER_THIRD.to_string()]);
    }

    #[test]
    fn lower_third_fills_omitted_style_without_preset_name() {
        let inv = title_using(LOWER_THIRD, Vec::new());
        let mut draft = draft();
        let parsed = parse_overlay_body(&inv, &mut draft);
        assert!(draft.diagnostics.is_empty(), "{:?}", draft.diagnostics);
        assert_eq!(parsed.applied_preset.as_deref(), Some(LOWER_THIRD));
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
        draft.overlay_presets = registry.overlay_presets();
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

    #[test]
    fn wasm_registered_ident_applies_like_builtin() {
        let mut draft = draft();
        draft
            .overlay_presets
            .register(
                "guest-plate",
                OverlayStyle {
                    size: Some(OverlaySize::Percent { milli: 500 }),
                    family: Some("GuestSans".into()),
                    bar: Some(OverlayBar::Fill {
                        color: Rgba::from_hex_rrggbb("#00FF00").unwrap(),
                    }),
                    ..OverlayStyle::default()
                },
                OverlayPresetSource::Wasm,
            )
            .unwrap();
        let inv = title_using("guest-plate", Vec::new());
        let parsed = parse_overlay_body(&inv, &mut draft);
        assert!(draft.diagnostics.is_empty(), "{:?}", draft.diagnostics);
        assert_eq!(parsed.applied_preset.as_deref(), Some("guest-plate"));
        assert_eq!(parsed.style.size, Some(OverlaySize::Percent { milli: 500 }));
        assert_eq!(parsed.style.family.as_deref(), Some("GuestSans"));
        let dumped = format!("{:?}", parsed.style);
        assert!(
            !dumped.contains("guest-plate"),
            "Core style must not store the IDENT: {dumped}"
        );
    }

    #[test]
    fn lookup_prefers_dsl_over_wasm_over_builtin() {
        let mut registry = OverlayPresetRegistry::builtin();
        let wasm = OverlayStyle {
            family: Some("WasmSans".into()),
            ..OverlayStyle::default()
        };
        let dsl = OverlayStyle {
            family: Some("DslSans".into()),
            ..OverlayStyle::default()
        };
        registry
            .register(LOWER_THIRD, wasm.clone(), OverlayPresetSource::Wasm)
            .unwrap();
        assert_eq!(
            registry.lookup(LOWER_THIRD).unwrap().family.as_deref(),
            Some("WasmSans")
        );
        registry
            .register(LOWER_THIRD, dsl, OverlayPresetSource::Dsl)
            .unwrap();
        assert_eq!(
            registry.lookup(LOWER_THIRD).unwrap().family.as_deref(),
            Some("DslSans")
        );
    }

    #[test]
    fn same_layer_redefinition_is_diag_not_overwrite() {
        let mut presets = OverlayPresetRegistry::default();
        let mut diagnostics = Vec::new();
        let first = preset_inv(
            "name-plate",
            vec![body_command("family", vec![quoted("FirstSans")])],
        );
        let second = preset_inv(
            "name-plate",
            vec![body_command("family", vec![quoted("SecondSans")])],
        );
        assert!(register_dsl_preset(&first, &mut presets, &mut diagnostics).is_some());
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert!(register_dsl_preset(&second, &mut presets, &mut diagnostics).is_none());
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.code == REDEFINED_PRESET && diag.message.contains("`name-plate`")),
            "{diagnostics:?}"
        );
        assert_eq!(
            presets
                .lookup("name-plate")
                .and_then(|style| style.family.clone()),
            Some("FirstSans".into())
        );
    }

    #[test]
    fn dsl_preset_body_rejects_geometry() {
        let mut presets = OverlayPresetRegistry::default();
        let mut diagnostics = Vec::new();
        let inv = preset_inv(
            "name-plate",
            vec![
                body_command("family", vec![quoted("LatticeSans")]),
                body_command(
                    "position",
                    vec![ValueView::Tuple(vec![percent(10), percent(20)])],
                ),
            ],
        );
        assert!(register_dsl_preset(&inv, &mut presets, &mut diagnostics).is_some());
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.code == INVALID_PRESET && diag.message.contains("`position`")),
            "{diagnostics:?}"
        );
        assert!(presets.lookup("name-plate").is_some());
    }

    #[test]
    fn wasm_guest_registers_lower_third_into_the_same_registry() {
        let registry = LoweringRegistry::stdlib();
        assert!(
            registry.uses_wasm(),
            "CHI-92 expects the hosted stdlib to provide overlay-presets"
        );
        let presets = registry.overlay_presets();
        assert!(presets.contains_in(LOWER_THIRD, OverlayPresetSource::Builtin));
        assert!(presets.contains_in(LOWER_THIRD, OverlayPresetSource::Wasm));
        assert_eq!(presets.lookup(LOWER_THIRD), Some(&lower_third_style()));
    }

    #[test]
    fn dsl_preset_missing_ident_diags() {
        let mut presets = OverlayPresetRegistry::default();
        let mut diagnostics = Vec::new();
        let inv = InvocationView {
            command: "overlay-preset".into(),
            args: Vec::new(),
            modifiers: Vec::new(),
            body: Vec::new(),
            span: span(),
        };
        assert!(register_dsl_preset(&inv, &mut presets, &mut diagnostics).is_none());
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.code == INVALID_PRESET && diag.message.contains("identifier")),
            "{diagnostics:?}"
        );
        assert!(presets.is_empty());
    }
}

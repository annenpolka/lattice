//! Overlay-body lowering for `title` / `callout`.
//!
//! Shared by in-process builtins and the Wasm host so invalid `position` /
//! `scale` and unknown body words produce the same diagnostics.

use lattice_core::{Diagnostic, NormalizedPosition, NormalizedScale};

use crate::view::{BodyItem, InvocationView, SceneDraft, ValueView};

/// Overlay body invocations that already have a lowering, plus `at`/`for`.
/// `align` / `anchor` are not implemented yet.
const OVERLAY_BODY_ALLOWLIST: &[&str] = &["opacity", "position", "scale", "at", "for"];
/// Generic parser modifiers that are already consumed as timing. Others
/// (`over` / `using` / `by` / `from` / `to`) must not silent-drop.
const OVERLAY_MODIFIER_ALLOWLIST: &[&str] = &["at", "for"];

pub const INVALID_POSITION: &str = "LAT-OVL-001";
pub const INVALID_SCALE: &str = "LAT-OVL-002";
pub const UNKNOWN_BODY_WORD: &str = "LAT-OVL-003";

/// Read overlay `position` / `scale` and diagnose unknown remaining body words.
pub fn overlay_geometry(
    inv: &InvocationView,
    draft: &mut SceneDraft,
) -> (Option<NormalizedPosition>, Option<NormalizedScale>) {
    let position = body_position(inv, draft);
    let scale = body_scale(inv, draft);
    diagnose_unknown_overlay_body(inv, draft);
    (position, scale)
}

pub fn body_position(inv: &InvocationView, draft: &mut SceneDraft) -> Option<NormalizedPosition> {
    let mut found = None;
    for item in &inv.body {
        let BodyItem::Invocation(inner) = item else {
            continue;
        };
        if inner.command != "position" {
            continue;
        }
        match inner
            .args
            .first()
            .and_then(ValueView::as_normalized_position)
        {
            Some(position) if found.is_none() => found = Some(position),
            Some(_) => {}
            None => draft.diagnostics.push(Diagnostic::error(
                INVALID_POSITION,
                "invalid overlay `position` (out-of-range, unit-less, or non-tuple); expected `(x%, y%)` in 0%..=100%",
                Some(inner.span),
            )),
        }
    }
    found
}

pub fn body_scale(inv: &InvocationView, draft: &mut SceneDraft) -> Option<NormalizedScale> {
    let mut found = None;
    for item in &inv.body {
        let BodyItem::Invocation(inner) = item else {
            continue;
        };
        if inner.command != "scale" {
            continue;
        }
        match inner.args.first().and_then(ValueView::as_normalized_scale) {
            Some(scale) if found.is_none() => found = Some(scale),
            Some(_) => {}
            None => draft.diagnostics.push(Diagnostic::error(
                INVALID_SCALE,
                "invalid overlay `scale` (out-of-range or unit-less); expected a percent in 25%..=200%",
                Some(inner.span),
            )),
        }
    }
    found
}

pub fn diagnose_unknown_overlay_body(inv: &InvocationView, draft: &mut SceneDraft) {
    for item in &inv.body {
        match item {
            BodyItem::Invocation(inner) => {
                if OVERLAY_BODY_ALLOWLIST.contains(&inner.command.as_str()) {
                    continue;
                }
                draft.diagnostics.push(Diagnostic::error(
                    UNKNOWN_BODY_WORD,
                    format!("unknown overlay body word `{}`", inner.command),
                    Some(inner.span),
                ));
            }
            BodyItem::Modifier { name, .. } => {
                if OVERLAY_MODIFIER_ALLOWLIST.contains(&name.as_str()) {
                    continue;
                }
                draft.diagnostics.push(Diagnostic::error(
                    UNKNOWN_BODY_WORD,
                    format!("unknown overlay body word `{name}`"),
                    Some(inv.span),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use lattice_core::{Severity, Span};

    use super::*;

    fn span() -> Span {
        Span::new(0, 1, 1, 1)
    }

    fn percent(digits: i64, frac_scale: u32) -> ValueView {
        ValueView::Quantity {
            negative: false,
            digits,
            scale: frac_scale,
            unit: Some("%".into()),
        }
    }

    fn unitless(digits: i64) -> ValueView {
        ValueView::Quantity {
            negative: false,
            digits,
            scale: 0,
            unit: None,
        }
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

    fn overlay_with_body(items: Vec<BodyItem>) -> InvocationView {
        InvocationView {
            command: "title".into(),
            args: vec![ValueView::String("Hello".into())],
            modifiers: Vec::new(),
            body: items,
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

    #[test]
    fn valid_tuple_position_and_percent_scale_do_not_diag() {
        let inv = overlay_with_body(vec![
            body_command(
                "position",
                vec![ValueView::Tuple(vec![percent(25, 0), percent(10, 0)])],
            ),
            body_command("scale", vec![percent(50, 0)]),
            body_command("opacity", vec![ValueView::Name("keep".into())]),
        ]);
        let mut draft = draft();
        let (position, scale) = overlay_geometry(&inv, &mut draft);
        assert_eq!(position, NormalizedPosition::new(2_500, 1_000));
        assert_eq!(scale, NormalizedScale::new(500));
        assert!(draft.diagnostics.is_empty(), "{:?}", draft.diagnostics);
    }

    #[test]
    fn out_of_range_and_unitless_position_diag_and_stay_none() {
        for args in [
            vec![percent(150, 0)],
            vec![unitless(10)],
            vec![ValueView::Tuple(vec![percent(150, 0), percent(0, 0)])],
        ] {
            let inv = overlay_with_body(vec![body_command("position", args)]);
            let mut draft = draft();
            let (position, _) = overlay_geometry(&inv, &mut draft);
            assert_eq!(position, None);
            assert!(
                draft.diagnostics.iter().any(|diag| {
                    diag.code == INVALID_POSITION
                        && diag.severity == Severity::Error
                        && diag.message.contains("out-of-range")
                        && diag.message.contains("unit-less")
                        && diag.message.contains("non-tuple")
                }),
                "{:?}",
                draft.diagnostics
            );
        }
    }

    #[test]
    fn invalid_scale_diags_and_stays_none() {
        for args in [
            vec![unitless(10)],
            vec![percent(10, 0)],
            vec![percent(250, 0)],
        ] {
            let inv = overlay_with_body(vec![body_command("scale", args)]);
            let mut draft = draft();
            let (_, scale) = overlay_geometry(&inv, &mut draft);
            assert_eq!(scale, None);
            assert!(
                draft
                    .diagnostics
                    .iter()
                    .any(|diag| diag.code == INVALID_SCALE && diag.message.contains("scale")),
                "{:?}",
                draft.diagnostics
            );
        }
    }

    #[test]
    fn unknown_body_word_does_not_vanish() {
        let inv = overlay_with_body(vec![
            body_command("align", vec![ValueView::Name("center".into())]),
            body_command("anchor", vec![ValueView::Name("top".into())]),
        ]);
        let mut draft = draft();
        overlay_geometry(&inv, &mut draft);
        let words: Vec<_> = draft
            .diagnostics
            .iter()
            .filter(|diag| diag.code == UNKNOWN_BODY_WORD)
            .map(|diag| diag.message.as_str())
            .collect();
        assert!(
            words.iter().any(|message| message.contains("`align`")),
            "{words:?}"
        );
        assert!(
            words.iter().any(|message| message.contains("`anchor`")),
            "{words:?}"
        );
    }

    fn body_modifier(name: &str) -> BodyItem {
        BodyItem::Modifier {
            name: name.into(),
            value: ValueView::Name("clip".into()),
        }
    }

    #[test]
    fn unknown_body_modifiers_do_not_vanish() {
        let inv = overlay_with_body(vec![
            body_modifier("at"),
            body_modifier("for"),
            body_modifier("over"),
            body_modifier("using"),
            body_modifier("by"),
            body_modifier("from"),
            body_modifier("to"),
        ]);
        let mut draft = draft();
        overlay_geometry(&inv, &mut draft);
        let words: Vec<_> = draft
            .diagnostics
            .iter()
            .filter(|diag| diag.code == UNKNOWN_BODY_WORD)
            .map(|diag| diag.message.clone())
            .collect();
        for allowed in ["`at`", "`for`"] {
            assert!(
                words.iter().all(|message| !message.contains(allowed)),
                "{allowed} should stay allowed: {words:?}"
            );
        }
        for unknown in ["`over`", "`using`", "`by`", "`from`", "`to`"] {
            assert!(
                words.iter().any(|message| message.contains(unknown)),
                "{unknown} vanished: {words:?}"
            );
        }
    }
}

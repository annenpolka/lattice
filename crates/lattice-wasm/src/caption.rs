//! Caption timing merge. `at` / `for` come from invocation modifiers and
//! overlay-body modifiers. After merge each appears exactly once.
//!
//! Body-less cues keep `at`/`for` inline (`caption "…" at 1s for 2s`). This
//! crate does not pretty-print VEL; CHI-86 owns `SemanticEdit` parity.

use lattice_core::{Diagnostic, Time};

use crate::view::{BodyItem, InvocationView, SceneDraft, ValueView};

pub const MISSING_TIMING: &str = "LAT-OVL-011";
pub const DUPLICATE_TIMING: &str = "LAT-OVL-012";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptionTiming {
    pub at: Time,
    pub hold: Time,
}

/// Merge modifier + body `at`/`for`. Missing → 011, duplicate → 012.
/// Returns `None` when the cue cannot be timed; no placement is emitted.
pub fn merge_caption_timing(inv: &InvocationView, draft: &mut SceneDraft) -> Option<CaptionTiming> {
    let at = merge_one_time(inv, draft, "at")?;
    let hold = merge_one_time(inv, draft, "for")?;
    Some(CaptionTiming { at, hold })
}

fn merge_one_time(inv: &InvocationView, draft: &mut SceneDraft, name: &str) -> Option<Time> {
    let hits = caption_times(inv, name);
    match hits.as_slice() {
        [] => {
            draft.diagnostics.push(Diagnostic::error(
                MISSING_TIMING,
                format!("caption is missing `{name}`"),
                Some(inv.span),
            ));
            None
        }
        [Some(time)] => Some(*time),
        [None] => {
            draft.diagnostics.push(Diagnostic::error(
                MISSING_TIMING,
                format!("caption `{name}` must be a time"),
                Some(inv.span),
            ));
            None
        }
        _ => {
            draft.diagnostics.push(Diagnostic::error(
                DUPLICATE_TIMING,
                format!("caption has duplicate `{name}`"),
                Some(inv.span),
            ));
            None
        }
    }
}

fn caption_times(inv: &InvocationView, name: &str) -> Vec<Option<Time>> {
    let mut times = Vec::new();
    for (key, value) in &inv.modifiers {
        if key == name {
            times.push(value.as_time());
        }
    }
    for item in &inv.body {
        match item {
            BodyItem::Modifier {
                name: item_name,
                value,
            } if item_name == name => times.push(value.as_time()),
            BodyItem::Invocation(inner) if inner.command == name => {
                times.push(inner.args.first().and_then(ValueView::as_time));
            }
            _ => {}
        }
    }
    times
}

#[cfg(test)]
mod tests {
    use lattice_core::{Severity, Span};

    use super::*;
    use crate::view::SceneDraft;

    fn span() -> Span {
        Span::new(0, 1, 1, 1)
    }

    fn draft() -> SceneDraft {
        SceneDraft {
            name: "intro".into(),
            ..SceneDraft::default()
        }
    }

    fn caption(modifiers: Vec<(&str, Time)>, body: Vec<BodyItem>) -> InvocationView {
        InvocationView {
            command: "caption".into(),
            args: vec![ValueView::String("cue".into())],
            modifiers: modifiers
                .into_iter()
                .map(|(name, time)| (name.into(), ValueView::Time(time)))
                .collect(),
            body,
            span: span(),
        }
    }

    fn body_time(name: &str, time: Time) -> BodyItem {
        BodyItem::Modifier {
            name: name.into(),
            value: ValueView::Time(time),
        }
    }

    #[test]
    fn inline_at_and_for_merge() {
        let inv = caption(
            vec![("at", Time::seconds(1)), ("for", Time::seconds(2))],
            Vec::new(),
        );
        let mut draft = draft();
        let timing = merge_caption_timing(&inv, &mut draft).expect("timing");
        assert_eq!(timing.at, Time::seconds(1));
        assert_eq!(timing.hold, Time::seconds(2));
        assert!(draft.diagnostics.is_empty(), "{:?}", draft.diagnostics);
    }

    #[test]
    fn body_at_and_for_merge() {
        let inv = caption(
            Vec::new(),
            vec![
                body_time("at", Time::seconds(3)),
                body_time("for", Time::seconds(4)),
            ],
        );
        let mut draft = draft();
        let timing = merge_caption_timing(&inv, &mut draft).expect("timing");
        assert_eq!(timing.at, Time::seconds(3));
        assert_eq!(timing.hold, Time::seconds(4));
        assert!(draft.diagnostics.is_empty(), "{:?}", draft.diagnostics);
    }

    #[test]
    fn split_modifier_and_body_merge() {
        let inv = caption(
            vec![("at", Time::seconds(1))],
            vec![body_time("for", Time::seconds(2))],
        );
        let mut draft = draft();
        let timing = merge_caption_timing(&inv, &mut draft).expect("timing");
        assert_eq!(timing.at, Time::seconds(1));
        assert_eq!(timing.hold, Time::seconds(2));
        assert!(draft.diagnostics.is_empty(), "{:?}", draft.diagnostics);
    }

    #[test]
    fn missing_at_or_for_is_011() {
        for modifiers in [
            vec![("at", Time::seconds(1))],
            vec![("for", Time::seconds(2))],
            Vec::new(),
        ] {
            let inv = caption(modifiers, Vec::new());
            let mut draft = draft();
            assert!(merge_caption_timing(&inv, &mut draft).is_none());
            assert!(
                draft
                    .diagnostics
                    .iter()
                    .any(|diag| diag.code == MISSING_TIMING && diag.severity == Severity::Error),
                "{:?}",
                draft.diagnostics
            );
        }
    }

    #[test]
    fn duplicate_at_or_for_is_012() {
        let inv = caption(
            vec![("at", Time::seconds(1)), ("for", Time::seconds(2))],
            vec![body_time("at", Time::seconds(3))],
        );
        let mut draft = draft();
        assert!(merge_caption_timing(&inv, &mut draft).is_none());
        assert!(
            draft
                .diagnostics
                .iter()
                .any(|diag| diag.code == DUPLICATE_TIMING && diag.message.contains("`at`")),
            "{:?}",
            draft.diagnostics
        );
    }
}

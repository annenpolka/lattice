//! Overlay-body lowering for `title` / `callout`.
//!
//! Shared by in-process builtins and the Wasm host so invalid `position` /
//! `scale` / style and unknown body words produce the same diagnostics.

use std::fmt::Write as _;

use lattice_core::{
    Diagnostic, NormalizedPosition, NormalizedScale, OverlayBar, OverlaySize, OverlayStyle, Rgba,
};

use crate::view::{BodyItem, InvocationView, SceneDraft, ValueView};

/// Overlay body invocations that already have a lowering, plus `at`/`for`.
/// `align` / `anchor` are not implemented yet (CHI-90 / CHI-91).
const OVERLAY_BODY_ALLOWLIST: &[&str] = &[
    "opacity", "position", "scale", "at", "for", "color", "size", "weight", "family", "bar",
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

/// Title vs callout convention used only for size explain (height/16 vs /20).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayConvention {
    Title,
    Callout,
}

impl OverlayConvention {
    fn size_base_explain(self) -> &'static str {
        match self {
            Self::Title => "title height/16",
            Self::Callout => "callout height/20",
        }
    }
}

/// Parsed overlay body after validation. Invalid values stay `None` and diag.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OverlayBody {
    pub position: Option<NormalizedPosition>,
    pub scale: Option<NormalizedScale>,
    pub style: OverlayStyle,
}

/// Shared by builtins and the Wasm host. Invalid style values diag; they do not silent-drop.
pub fn parse_overlay_body(inv: &InvocationView, draft: &mut SceneDraft) -> OverlayBody {
    let position = body_position(inv, draft);
    let scale = body_scale(inv, draft);
    let style = body_style(inv, draft);
    diagnose_unknown_overlay_body(inv, draft);
    OverlayBody {
        position,
        scale,
        style,
    }
}

/// Compile-time explain suffix for explicit overlay geometry and style.
pub fn overlay_explain_notes(
    position: Option<NormalizedPosition>,
    scale: Option<NormalizedScale>,
    style: &OverlayStyle,
    convention: OverlayConvention,
) -> String {
    let mut notes = String::new();
    if let Some(position) = position {
        let _ = write!(notes, " position {position}");
    }
    if let Some(scale) = scale {
        let _ = write!(notes, " scale {scale}");
    }
    if let Some(color) = style.color {
        let _ = write!(notes, " color {}", color.to_hex_rrggbb());
    }
    if let Some(size) = style.size {
        notes.push_str(&size_explain(size, convention));
    }
    if let Some(weight) = style.weight {
        let _ = write!(notes, " weight {weight}");
    }
    if let Some(family) = &style.family {
        let _ = write!(notes, " family {family:?}");
    }
    match style.bar {
        Some(OverlayBar::Off) => notes.push_str(" bar off"),
        Some(OverlayBar::Fill { color }) => {
            let _ = write!(notes, " bar {}", color.to_hex_rrggbb());
        }
        None => {}
    }
    notes
}

fn size_explain(size: OverlaySize, convention: OverlayConvention) -> String {
    match size {
        OverlaySize::Percent { milli } => {
            let percent = format_milli_percent(milli);
            format!(
                " size {percent} (base {}, resolved at evaluate)",
                convention.size_base_explain()
            )
        }
        OverlaySize::Px { px } => {
            format!(
                " size {px}px (base {}, resolved {px}px)",
                convention.size_base_explain()
            )
        }
    }
}

fn format_milli_percent(milli: u16) -> String {
    let whole = milli / 10;
    let frac = milli % 10;
    if frac == 0 {
        format!("{whole}%")
    } else {
        format!("{whole}.{frac}%")
    }
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

fn body_style(inv: &InvocationView, draft: &mut SceneDraft) -> OverlayStyle {
    OverlayStyle {
        color: body_color(inv, draft),
        size: body_size(inv, draft),
        weight: body_weight(inv, draft),
        family: body_family(inv, draft),
        bar: body_bar(inv, draft),
    }
}

fn body_color(inv: &InvocationView, draft: &mut SceneDraft) -> Option<Rgba> {
    let mut found = None;
    for inner in body_invocations(inv, "color") {
        match inner.args.first().and_then(parse_hex_color) {
            Some(color) if found.is_none() => found = Some(color),
            Some(_) => {}
            None => draft.diagnostics.push(Diagnostic::error(
                INVALID_COLOR,
                "invalid overlay `color`; expected a quoted `#RRGGBB` (no named colors)",
                Some(inner.span),
            )),
        }
    }
    found
}

fn body_size(inv: &InvocationView, draft: &mut SceneDraft) -> Option<OverlaySize> {
    let mut found = None;
    for inner in body_invocations(inv, "size") {
        match inner.args.first().and_then(parse_overlay_size) {
            Some(size) if found.is_none() => found = Some(size),
            Some(_) => {}
            None => draft.diagnostics.push(Diagnostic::error(
                INVALID_SIZE,
                "invalid overlay `size`; expected a convention percent (`50%`) or a pixel lock (`24px`)",
                Some(inner.span),
            )),
        }
    }
    found
}

fn body_weight(inv: &InvocationView, draft: &mut SceneDraft) -> Option<u16> {
    let mut found = None;
    for inner in body_invocations(inv, "weight") {
        match inner.args.first().and_then(parse_overlay_weight) {
            Some(weight) if found.is_none() => found = Some(weight),
            Some(_) => {}
            None => draft.diagnostics.push(Diagnostic::error(
                INVALID_WEIGHT,
                "invalid overlay `weight`; expected `normal`, `bold`, or an integer 1..=1000",
                Some(inner.span),
            )),
        }
    }
    found
}

fn body_family(inv: &InvocationView, draft: &mut SceneDraft) -> Option<String> {
    let mut found = None;
    for inner in body_invocations(inv, "family") {
        match inner.args.first().and_then(parse_overlay_family) {
            Some(family) if found.is_none() => found = Some(family),
            Some(_) => {}
            None => draft.diagnostics.push(Diagnostic::error(
                INVALID_FAMILY,
                "invalid overlay `family`; expected a quoted string",
                Some(inner.span),
            )),
        }
    }
    found
}

fn body_bar(inv: &InvocationView, draft: &mut SceneDraft) -> Option<OverlayBar> {
    let mut found = None;
    for inner in body_invocations(inv, "bar") {
        match inner.args.first().and_then(parse_overlay_bar) {
            Some(bar) if found.is_none() => found = Some(bar),
            Some(_) => {}
            None => draft.diagnostics.push(Diagnostic::error(
                INVALID_BAR,
                "invalid overlay `bar`; expected `off` or a quoted `#RRGGBB`",
                Some(inner.span),
            )),
        }
    }
    found
}

fn body_invocations<'a>(
    inv: &'a InvocationView,
    command: &'a str,
) -> impl Iterator<Item = &'a InvocationView> {
    inv.body.iter().filter_map(move |item| match item {
        BodyItem::Invocation(inner) if inner.command == command => Some(inner),
        _ => None,
    })
}

fn parse_hex_color(value: &ValueView) -> Option<Rgba> {
    value.as_string().and_then(Rgba::from_hex_rrggbb)
}

fn parse_overlay_size(value: &ValueView) -> Option<OverlaySize> {
    if let Some(milli) = percent_milli(value) {
        return Some(OverlaySize::Percent { milli });
    }
    px_size(value).map(|px| OverlaySize::Px { px })
}

fn percent_milli(value: &ValueView) -> Option<u16> {
    let ValueView::Quantity {
        negative,
        digits,
        scale,
        unit: Some(unit),
    } = value
    else {
        return None;
    };
    if *negative || unit != "%" || *digits < 0 {
        return None;
    }
    let divisor = 10_i128.checked_pow(*scale)?;
    let numerator = i128::from(*digits).checked_mul(10)?;
    let rounded = numerator.checked_add(divisor / 2)?.checked_div(divisor)?;
    u16::try_from(rounded).ok().filter(|milli| *milli > 0)
}

fn px_size(value: &ValueView) -> Option<u32> {
    let ValueView::Quantity {
        negative,
        digits,
        scale,
        unit: Some(unit),
    } = value
    else {
        return None;
    };
    if *negative || unit != "px" || *scale != 0 || *digits <= 0 {
        return None;
    }
    u32::try_from(*digits).ok()
}

fn parse_overlay_weight(value: &ValueView) -> Option<u16> {
    match value {
        ValueView::Name(name) if name == "normal" => Some(400),
        ValueView::Name(name) if name == "bold" => Some(700),
        other => other
            .as_int()
            .and_then(|weight| u16::try_from(weight).ok())
            .filter(|weight| (1..=1000).contains(weight)),
    }
}

fn parse_overlay_family(value: &ValueView) -> Option<String> {
    value
        .as_string()
        .filter(|family| !family.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_overlay_bar(value: &ValueView) -> Option<OverlayBar> {
    match value {
        ValueView::Name(name) if name == "off" => Some(OverlayBar::Off),
        ValueView::String(text) => {
            Rgba::from_hex_rrggbb(text).map(|color| OverlayBar::Fill { color })
        }
        _ => None,
    }
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
        let parsed = parse_overlay_body(&inv, &mut draft);
        assert_eq!(parsed.position, NormalizedPosition::new(2_500, 1_000));
        assert_eq!(parsed.scale, NormalizedScale::new(500));
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
            let parsed = parse_overlay_body(&inv, &mut draft);
            assert_eq!(parsed.position, None);
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
            let parsed = parse_overlay_body(&inv, &mut draft);
            assert_eq!(parsed.scale, None);
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
        parse_overlay_body(&inv, &mut draft);
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
        parse_overlay_body(&inv, &mut draft);
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

    fn quoted(text: &str) -> ValueView {
        ValueView::String(text.into())
    }

    fn name(text: &str) -> ValueView {
        ValueView::Name(text.into())
    }

    fn px(digits: i64) -> ValueView {
        ValueView::Quantity {
            negative: false,
            digits,
            scale: 0,
            unit: Some("px".into()),
        }
    }

    #[test]
    fn valid_style_words_do_not_diag() {
        let inv = overlay_with_body(vec![
            body_command("color", vec![quoted("#00FF00")]),
            body_command("size", vec![percent(50, 0)]),
            body_command("weight", vec![name("bold")]),
            body_command("family", vec![quoted("LatticeSans")]),
            body_command("bar", vec![name("off")]),
        ]);
        let mut draft = draft();
        let parsed = parse_overlay_body(&inv, &mut draft);
        assert!(draft.diagnostics.is_empty(), "{:?}", draft.diagnostics);
        assert_eq!(parsed.style.color, Rgba::from_hex_rrggbb("#00FF00"));
        assert_eq!(parsed.style.size, Some(OverlaySize::Percent { milli: 500 }));
        assert_eq!(parsed.style.weight, Some(700));
        assert_eq!(parsed.style.family.as_deref(), Some("LatticeSans"));
        assert_eq!(parsed.style.bar, Some(OverlayBar::Off));
        let notes = overlay_explain_notes(
            parsed.position,
            parsed.scale,
            &parsed.style,
            OverlayConvention::Title,
        );
        assert!(notes.contains("color #00FF00"), "{notes}");
        assert!(
            notes.contains("size 50%") && notes.contains("base") && notes.contains("resolved"),
            "{notes}"
        );
        assert!(notes.contains("title height/16"), "{notes}");
        assert!(notes.contains("weight 700"), "{notes}");
        assert!(notes.contains("family \"LatticeSans\""), "{notes}");
        assert!(notes.contains("bar off"), "{notes}");
    }

    #[test]
    fn size_px_locks_and_weight_int_maps() {
        let inv = overlay_with_body(vec![
            body_command("size", vec![px(24)]),
            body_command("weight", vec![unitless(700)]),
            body_command("bar", vec![quoted("#FF00FF")]),
        ]);
        let mut draft = draft();
        let parsed = parse_overlay_body(&inv, &mut draft);
        assert!(draft.diagnostics.is_empty(), "{:?}", draft.diagnostics);
        assert_eq!(parsed.style.size, Some(OverlaySize::Px { px: 24 }));
        assert_eq!(parsed.style.weight, Some(700));
        assert_eq!(
            parsed.style.bar,
            Some(OverlayBar::Fill {
                color: Rgba::from_hex_rrggbb("#FF00FF").unwrap()
            })
        );
        let notes = overlay_explain_notes(None, None, &parsed.style, OverlayConvention::Callout);
        assert!(
            notes.contains("size 24px") && notes.contains("resolved 24px"),
            "{notes}"
        );
        assert!(notes.contains("callout height/20"), "{notes}");
    }

    #[test]
    fn invalid_style_values_diag_and_stay_none() {
        let cases: &[(&str, &str, ValueView)] = &[
            ("color", INVALID_COLOR, name("green")),
            ("color", INVALID_COLOR, quoted("green")),
            ("color", INVALID_COLOR, quoted("#GG0000")),
            ("color", INVALID_COLOR, quoted("#FFF")),
            ("color", INVALID_COLOR, unitless(255)),
            ("size", INVALID_SIZE, unitless(24)),
            ("size", INVALID_SIZE, name("large")),
            ("weight", INVALID_WEIGHT, name("light")),
            ("weight", INVALID_WEIGHT, unitless(0)),
            ("weight", INVALID_WEIGHT, unitless(1001)),
            ("family", INVALID_FAMILY, name("LatticeSans")),
            ("family", INVALID_FAMILY, quoted("")),
            ("bar", INVALID_BAR, name("on")),
            ("bar", INVALID_BAR, quoted("yellow")),
        ];
        for (command, code, arg) in cases {
            let inv = overlay_with_body(vec![body_command(command, vec![arg.clone()])]);
            let mut draft = draft();
            let parsed = parse_overlay_body(&inv, &mut draft);
            assert!(
                draft
                    .diagnostics
                    .iter()
                    .any(|diag| diag.code == *code && diag.message.contains(command)),
                "{command} {arg:?}: {:?}",
                draft.diagnostics
            );
            assert!(
                parsed.style.is_empty(),
                "{command} leaked: {:?}",
                parsed.style
            );
        }
    }

    #[test]
    fn align_stays_unknown() {
        let inv = overlay_with_body(vec![body_command("align", vec![name("center")])]);
        let mut draft = draft();
        parse_overlay_body(&inv, &mut draft);
        assert!(
            draft
                .diagnostics
                .iter()
                .any(|diag| diag.code == UNKNOWN_BODY_WORD && diag.message.contains("`align`")),
            "{:?}",
            draft.diagnostics
        );
    }
}

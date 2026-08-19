use lattice_core::{EditProposal, Locus, SemanticEdit, Span};
use lattice_vel::{Document, Expr, Invocation, Item};

use crate::EngineError;

pub fn propose_edit(
    source: &str,
    locus: &Locus,
    edit: SemanticEdit,
) -> Result<EditProposal, EngineError> {
    if edit.is_empty() {
        return Err(EngineError::Edit("edit has no fields".into()));
    }
    let document = lattice_vel::parse(source)?;
    let new_source = apply_semantic_edit(source, &document, locus, &edit)?;
    Ok(EditProposal {
        locus_id: locus.id.clone(),
        description: edit.describe(),
        edit,
        vel_diff: line_diff(source, &new_source),
        new_source,
    })
}

pub fn apply_proposal(_source: &str, proposal: &EditProposal) -> String {
    proposal.new_source.clone()
}

fn apply_semantic_edit(
    source: &str,
    document: &Document,
    locus: &Locus,
    edit: &SemanticEdit,
) -> Result<String, EngineError> {
    let SemanticEdit::Title {
        text,
        at,
        duration,
        opacity,
    } = edit;
    let inv = find_title(document, locus)
        .ok_or_else(|| EngineError::Edit("title locus did not match a title invocation".into()))?;
    let mut splices = Vec::new();
    if let Some(text) = text {
        let span = string_arg_span(inv)
            .ok_or_else(|| EngineError::Edit("title has no string argument".into()))?;
        splices.push((span, quote_vel_string(text)));
    }
    if let Some(at) = at {
        let span = modifier_value_span(inv, "at")
            .ok_or_else(|| EngineError::Edit("title has no `at` modifier".into()))?;
        splices.push((span, at.to_string()));
    }
    if let Some(duration) = duration {
        let span = modifier_value_span(inv, "for")
            .ok_or_else(|| EngineError::Edit("title has no `for` modifier".into()))?;
        splices.push((span, duration.to_string()));
    }
    if let Some(opacity) = opacity {
        if let Some(span) = opacity_arg_span(inv) {
            splices.push((span, opacity.to_string()));
        } else if let Some(body) = &inv.body {
            let insert_at = Span::new(
                body.span.end.saturating_sub(1),
                body.span.end.saturating_sub(1),
                body.span.line,
                body.span.column,
            );
            splices.push((insert_at, format!("    opacity {opacity}\n  ")));
        } else {
            return Err(EngineError::Edit(
                "title has no body to attach opacity".into(),
            ));
        }
    }
    Ok(apply_splices(source, splices))
}

fn find_title<'a>(document: &'a Document, locus: &Locus) -> Option<&'a Invocation> {
    let want = locus.source_span;
    for item in walk_items(&document.items) {
        let Item::Invocation(inv) = item else {
            continue;
        };
        if inv.name != "title" {
            continue;
        }
        if want.is_some_and(|span| inv.span.start == span.start && inv.span.end == span.end) {
            return Some(inv);
        }
        if want.is_none() && locus.node_id.contains("title") {
            return Some(inv);
        }
    }
    walk_items(&document.items).find_map(|item| match item {
        Item::Invocation(inv) if inv.name == "title" => Some(inv),
        _ => None,
    })
}

fn walk_items(items: &[Item]) -> impl Iterator<Item = &Item> {
    fn walk<'a>(items: &'a [Item], out: &mut Vec<&'a Item>) {
        for item in items {
            out.push(item);
            match item {
                Item::Project {
                    body: Some(body), ..
                }
                | Item::Sequence { body, .. }
                | Item::Scene { body, .. }
                | Item::Narration { body, .. } => walk(&body.items, out),
                Item::Invocation(inv) => {
                    if let Some(body) = &inv.body {
                        walk(&body.items, out);
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(items, &mut out);
    out.into_iter()
}

fn string_arg_span(inv: &Invocation) -> Option<Span> {
    match inv.args.first() {
        Some(Expr::String { span, .. }) => Some(*span),
        _ => None,
    }
}

fn modifier_value_span(inv: &Invocation, name: &str) -> Option<Span> {
    if let Some(modifier) = inv.modifiers.iter().find(|modifier| modifier.name == name) {
        return Some(modifier.value.span());
    }
    let body = inv.body.as_ref()?;
    for item in &body.items {
        if let Item::Modifiers { modifiers, .. } = item
            && let Some(modifier) = modifiers.iter().find(|modifier| modifier.name == name)
        {
            return Some(modifier.value.span());
        }
    }
    None
}

fn opacity_arg_span(inv: &Invocation) -> Option<Span> {
    let body = inv.body.as_ref()?;
    for item in &body.items {
        if let Item::Invocation(inner) = item
            && inner.name == "opacity"
        {
            return inner.args.first().map(Expr::span);
        }
    }
    None
}

fn quote_vel_string(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn apply_splices(source: &str, mut splices: Vec<(Span, String)>) -> String {
    splices.sort_by_key(|(span, _)| span.start);
    splices.reverse();
    let mut out = source.to_string();
    for (span, replacement) in splices {
        let start = span.start as usize;
        let end = (span.end as usize).min(out.len()).max(start);
        if start > out.len() {
            continue;
        }
        out.replace_range(start..end, &replacement);
    }
    out
}

fn line_diff(old: &str, new: &str) -> String {
    use std::fmt::Write as _;
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let mut out = String::from("--- a/source.vel\n+++ b/source.vel\n");
    let max = old_lines.len().max(new_lines.len());
    for i in 0..max {
        match (old_lines.get(i), new_lines.get(i)) {
            (Some(a), Some(b)) if a == b => {}
            (Some(a), Some(b)) => {
                let _ = write!(out, "@@ line {} @@\n-{a}\n+{b}\n", i + 1);
            }
            (Some(a), None) => {
                let _ = write!(out, "@@ line {} @@\n-{a}\n", i + 1);
            }
            (None, Some(b)) => {
                let _ = write!(out, "@@ line {} @@\n+{b}\n", i + 1);
            }
            _ => {}
        }
    }
    if out.lines().count() <= 2 {
        out.push_str("@@ no line changes @@\n");
    }
    out
}

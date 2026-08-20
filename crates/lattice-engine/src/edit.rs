use lattice_core::{
    EditProposal, Locus, LocusKind, SemanticEdit, Span, Time, TimeSpan, source_revision,
};
use lattice_vel::{Document, Expr, Invocation, Item};

use crate::EngineError;
use crate::time_eval::expr_time;

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
        base_revision: source_revision(source),
    })
}

pub fn apply_proposal(source: &str, proposal: &EditProposal) -> Result<String, EngineError> {
    let found = source_revision(source);
    if proposal.base_revision != found {
        return Err(EngineError::StaleProposal {
            expected: proposal.base_revision.clone(),
            found,
        });
    }
    Ok(proposal.new_source.clone())
}

fn apply_semantic_edit(
    source: &str,
    document: &Document,
    locus: &Locus,
    edit: &SemanticEdit,
) -> Result<String, EngineError> {
    match edit {
        SemanticEdit::Title {
            text,
            at,
            duration,
            opacity,
        } => apply_title(
            source,
            document,
            locus,
            text.as_ref(),
            *at,
            *duration,
            *opacity,
        ),
        SemanticEdit::Trim {
            in_point,
            out_point,
        } => apply_trim(source, document, locus, *in_point, *out_point),
        SemanticEdit::Split { at } => apply_split(source, document, locus, *at),
        SemanticEdit::Delete => apply_delete(source, document, locus),
        SemanticEdit::SetGain { db } => apply_gain(source, document, locus, *db),
        SemanticEdit::SetFade { fade_in } => {
            let fade = fade_in.ok_or_else(|| EngineError::Edit("fade has no duration".into()))?;
            apply_fade(source, document, locus, fade)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_title(
    source: &str,
    document: &Document,
    locus: &Locus,
    text: Option<&String>,
    at: Option<Time>,
    duration: Option<Time>,
    opacity: Option<u8>,
) -> Result<String, EngineError> {
    if let Some(inv) = find_title(document, locus) {
        return splice_title(source, inv, text, at, duration, opacity);
    }
    if matches!(
        locus.kind,
        LocusKind::Scene | LocusKind::Source | LocusKind::Sequence
    ) {
        let text = text
            .map(String::as_str)
            .ok_or_else(|| EngineError::Edit("title insert needs text".into()))?;
        return insert_title(source, document, locus, text, at, duration, opacity);
    }
    Err(EngineError::Edit(
        "title locus did not match a title invocation".into(),
    ))
}

fn splice_title(
    source: &str,
    inv: &Invocation,
    text: Option<&String>,
    at: Option<Time>,
    duration: Option<Time>,
    opacity: Option<u8>,
) -> Result<String, EngineError> {
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

fn insert_title(
    source: &str,
    document: &Document,
    locus: &Locus,
    text: &str,
    at: Option<Time>,
    duration: Option<Time>,
    opacity: Option<u8>,
) -> Result<String, EngineError> {
    let scene = find_scene(document, locus)
        .ok_or_else(|| EngineError::Edit("no scene to attach title".into()))?;
    let Item::Scene { body, .. } = scene else {
        return Err(EngineError::Edit("title insert expected a scene".into()));
    };
    let at = at.unwrap_or(Time::ZERO);
    let duration = duration.unwrap_or(Time::seconds(3));
    let opacity_line = opacity.map_or(String::new(), |value| format!("    opacity {value}\n"));
    let snippet = format!(
        "  title {} {{\n    at {at} for {duration}\n{opacity_line}  }}\n",
        quote_vel_string(text)
    );
    let insert_at = Span::new(
        body.span.end.saturating_sub(1),
        body.span.end.saturating_sub(1),
        body.span.line,
        body.span.column,
    );
    Ok(apply_splices(source, vec![(insert_at, snippet)]))
}

fn apply_trim(
    source: &str,
    document: &Document,
    locus: &Locus,
    in_point: Option<Time>,
    out_point: Option<Time>,
) -> Result<String, EngineError> {
    let binding = find_binding(document, locus)
        .ok_or_else(|| EngineError::Edit("trim target is not a source range".into()))?;
    let Item::Binding { expr, .. } = binding else {
        return Err(EngineError::Edit("trim expected a source binding".into()));
    };
    let Expr::Index { index, .. } = expr else {
        return Err(EngineError::Edit("trim expected media[start..end]".into()));
    };
    let Expr::Range { start, end, .. } = index.as_ref() else {
        return Err(EngineError::Edit("trim expected a time range".into()));
    };
    let start_t = expr_time(start).map_err(|err| EngineError::Edit(err.to_string()))?;
    let end_t = expr_time(end).map_err(|err| EngineError::Edit(err.to_string()))?;
    let duration = end_t
        .checked_sub(start_t)
        .map_err(|err| EngineError::Edit(err.to_string()))?;
    let original = TimeSpan::new(start_t, duration);
    let new_in = in_point.unwrap_or(start_t);
    let new_out = out_point.unwrap_or(end_t);
    if new_in < original.start || new_out > original.end() || new_in >= new_out {
        return Err(EngineError::Edit(
            "trim must stay inside the original source range".into(),
        ));
    }
    let mut splices = Vec::new();
    if in_point.is_some() {
        splices.push((start.span(), new_in.to_string()));
    }
    if out_point.is_some() {
        splices.push((end.span(), new_out.to_string()));
    }
    Ok(apply_splices(source, splices))
}

fn apply_split(
    source: &str,
    document: &Document,
    locus: &Locus,
    at: Time,
) -> Result<String, EngineError> {
    let scene = find_scene(document, locus)
        .ok_or_else(|| EngineError::Edit("split target is not a scene".into()))?;
    let Item::Scene {
        name, body, span, ..
    } = scene
    else {
        return Err(EngineError::Edit("split expected a scene".into()));
    };
    let binding = body.items.iter().find_map(|item| match item {
        Item::Binding { expr, name, span } => Some((expr, name.as_str(), *span)),
        _ => None,
    });
    let Some((expr, binding_name, binding_span)) = binding else {
        return Err(EngineError::Edit(
            "split scene has no source binding".into(),
        ));
    };
    let Expr::Index { target, index, .. } = expr else {
        return Err(EngineError::Edit("split expected media[start..end]".into()));
    };
    let Expr::Range { start, end, .. } = index.as_ref() else {
        return Err(EngineError::Edit("split expected a time range".into()));
    };
    let start_t = expr_time(start).map_err(|err| EngineError::Edit(err.to_string()))?;
    let end_t = expr_time(end).map_err(|err| EngineError::Edit(err.to_string()))?;
    let duration = end_t
        .checked_sub(start_t)
        .map_err(|err| EngineError::Edit(err.to_string()))?;
    let original = TimeSpan::new(start_t, duration);
    let Some((left, right)) = original.split_at(at) else {
        return Err(EngineError::Edit(format!(
            "split time {at} is not inside {start_t}..{end_t}"
        )));
    };
    let new_name = unique_scene_name(document, name);
    let media_name = match target.as_ref() {
        Expr::Ident { name, .. } => name.clone(),
        Expr::Path { parts, .. } => parts.join("."),
        _ => {
            return Err(EngineError::Edit("split binding has no media name".into()));
        }
    };
    let rest = rest_of_scene_body(source, body, binding_span);
    let new_scene = format!(
        "\n\nscene {new_name} {{\n  {media_name}[{}..{}] as {binding_name}\n{rest}}}\n",
        right.start,
        right.end()
    );
    let mut splices = vec![
        (end.span(), left.end().to_string()),
        (
            Span::new(span.end, span.end, span.line, span.column),
            new_scene,
        ),
    ];
    if let Some(inv) = find_sequence_ref(document, name) {
        splices.push((
            Span::new(inv.span.end, inv.span.end, inv.span.line, inv.span.column),
            format!("\n  {new_name}"),
        ));
    }
    Ok(apply_splices(source, splices))
}

fn rest_of_scene_body(source: &str, body: &lattice_vel::Block, binding_span: Span) -> String {
    let start = binding_span.end as usize;
    let end = body.span.end.saturating_sub(1) as usize;
    if start >= source.len() || end <= start {
        return String::new();
    }
    let slice = &source[start.min(source.len())..end.min(source.len())];
    let trimmed = slice.trim_start_matches(['\r', '\n']);
    if trimmed.trim().is_empty() {
        String::new()
    } else if trimmed.ends_with('\n') {
        trimmed.to_string()
    } else {
        format!("{trimmed}\n")
    }
}

fn unique_scene_name(document: &Document, base: &str) -> String {
    let names = scene_names(document);
    for i in 2..10_000 {
        let candidate = format!("{base}_{i}");
        if !names.iter().any(|name| name == &candidate) {
            return candidate;
        }
    }
    format!("{base}_split")
}

fn scene_names(document: &Document) -> Vec<String> {
    walk_items(&document.items)
        .filter_map(|item| match item {
            Item::Scene { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

fn apply_delete(source: &str, document: &Document, locus: &Locus) -> Result<String, EngineError> {
    let scene = find_scene(document, locus)
        .ok_or_else(|| EngineError::Edit("delete target is not a scene".into()))?;
    let Item::Scene { name, span, .. } = scene else {
        return Err(EngineError::Edit("delete expected a scene".into()));
    };
    let mut splices = Vec::new();
    if let Some(inv) = find_sequence_ref(document, name) {
        splices.push((widen_line(source, inv.span), String::new()));
    }
    splices.push((widen_line(source, *span), String::new()));
    Ok(apply_splices(source, splices))
}

fn widen_line(source: &str, span: Span) -> Span {
    let bytes = source.as_bytes();
    let mut start = span.start as usize;
    let mut end = (span.end as usize).min(bytes.len());
    while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
        start -= 1;
    }
    if end < bytes.len() && bytes[end] == b'\r' {
        end += 1;
    }
    if end < bytes.len() && bytes[end] == b'\n' {
        end += 1;
    } else if start > 0 && bytes[start - 1] == b'\n' {
        start -= 1;
        if start > 0 && bytes[start - 1] == b'\r' {
            start -= 1;
        }
    }
    Span::new(
        u32::try_from(start).unwrap_or(span.start),
        u32::try_from(end).unwrap_or(span.end),
        span.line,
        span.column,
    )
}

fn apply_gain(
    source: &str,
    document: &Document,
    locus: &Locus,
    db: i32,
) -> Result<String, EngineError> {
    let scene = find_scene(document, locus)
        .ok_or_else(|| EngineError::Edit("gain target is not a scene".into()))?;
    let Item::Scene { body, .. } = scene else {
        return Err(EngineError::Edit("gain expected a scene".into()));
    };
    let source_name = primary_source_name(body, locus)
        .ok_or_else(|| EngineError::Edit("gain needs a source".into()))?;
    if let Some(inv) = find_named_invocation(body, "gain")
        && let Some(span) =
            modifier_value_span(inv, "by").or_else(|| inv.args.get(1).map(Expr::span))
    {
        return Ok(apply_splices(source, vec![(span, db.to_string())]));
    }
    let insert_at = Span::new(
        body.span.end.saturating_sub(1),
        body.span.end.saturating_sub(1),
        body.span.line,
        body.span.column,
    );
    Ok(apply_splices(
        source,
        vec![(insert_at, format!("  gain {source_name} by {db}\n"))],
    ))
}

fn apply_fade(
    source: &str,
    document: &Document,
    locus: &Locus,
    fade: Time,
) -> Result<String, EngineError> {
    let scene = find_scene(document, locus)
        .ok_or_else(|| EngineError::Edit("fade target is not a scene".into()))?;
    let Item::Scene { body, .. } = scene else {
        return Err(EngineError::Edit("fade expected a scene".into()));
    };
    let source_name = primary_source_name(body, locus)
        .ok_or_else(|| EngineError::Edit("fade needs a source".into()))?;
    if let Some(inv) = find_named_invocation(body, "fade")
        && let Some(span) = modifier_value_span(inv, "for")
    {
        return Ok(apply_splices(source, vec![(span, fade.to_string())]));
    }
    let insert_at = Span::new(
        body.span.end.saturating_sub(1),
        body.span.end.saturating_sub(1),
        body.span.line,
        body.span.column,
    );
    Ok(apply_splices(
        source,
        vec![(
            insert_at,
            format!("  fade {source_name} {{\n    at 0s for {fade}\n  }}\n"),
        )],
    ))
}

fn primary_source_name<'a>(body: &'a lattice_vel::Block, locus: &Locus) -> Option<&'a str> {
    if locus.kind == LocusKind::Source {
        for item in &body.items {
            if let Item::Binding { name, span, .. } = item
                && locus
                    .source_span
                    .is_some_and(|want| want.start == span.start && want.end == span.end)
            {
                return Some(name.as_str());
            }
        }
        if let Some(label) = Some(locus.label.as_str()) {
            for item in &body.items {
                if let Item::Binding { name, .. } = item
                    && name == label
                {
                    return Some(name.as_str());
                }
            }
        }
    }
    body.items.iter().find_map(|item| match item {
        Item::Binding { name, .. } => Some(name.as_str()),
        _ => None,
    })
}

fn find_named_invocation<'a>(body: &'a lattice_vel::Block, name: &str) -> Option<&'a Invocation> {
    body.items.iter().find_map(|item| match item {
        Item::Invocation(inv) if inv.name == name => Some(inv),
        _ => None,
    })
}

fn find_title<'a>(document: &'a Document, locus: &Locus) -> Option<&'a Invocation> {
    let want = locus.source_span?;
    for item in walk_items(&document.items) {
        let Item::Invocation(inv) = item else {
            continue;
        };
        if inv.name != "title" {
            continue;
        }
        if inv.span.start == want.start && inv.span.end == want.end {
            return Some(inv);
        }
    }
    None
}

fn find_scene<'a>(document: &'a Document, locus: &Locus) -> Option<&'a Item> {
    let scene_id = locus.scene_id.as_deref();
    let node = locus.node_id.as_str();
    walk_items(&document.items).find(|item| {
        let Item::Scene { name, span, .. } = item else {
            return false;
        };
        let id = format!("scene:{name}");
        scene_id == Some(id.as_str())
            || node == id
            || node == *name
            || locus.label == *name
            || locus
                .source_span
                .is_some_and(|want| want.start >= span.start && want.end <= span.end)
    })
}

fn find_binding<'a>(document: &'a Document, locus: &Locus) -> Option<&'a Item> {
    if let Some(want) = locus.source_span {
        for item in walk_items(&document.items) {
            if let Item::Binding { span, .. } = item
                && span.start == want.start
                && span.end == want.end
            {
                return Some(item);
            }
        }
    }
    let scene = find_scene(document, locus)?;
    let Item::Scene { body, .. } = scene else {
        return None;
    };
    body.items
        .iter()
        .find(|item| matches!(item, Item::Binding { .. }))
}

fn find_sequence_ref<'a>(document: &'a Document, scene_name: &str) -> Option<&'a Invocation> {
    for item in walk_items(&document.items) {
        let Item::Sequence { body, .. } = item else {
            continue;
        };
        for inner in &body.items {
            if let Item::Invocation(inv) = inner
                && inv.name == scene_name
                && inv.args.is_empty()
            {
                return Some(inv);
            }
        }
    }
    None
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

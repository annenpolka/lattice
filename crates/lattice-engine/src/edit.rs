use lattice_core::{
    EditProposal, Locus, LocusKind, NormalizedPosition, NormalizedScale, SemanticEdit, Span, Time,
    TimeSpan, source_revision,
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
        SemanticEdit::ReorderScene { before } => {
            apply_reorder(source, document, locus, before.as_deref())
        }
        SemanticEdit::Callout { text, at, duration } => {
            apply_callout(source, document, locus, text.as_ref(), *at, *duration)
        }
        SemanticEdit::SetPosition { position } => {
            apply_position(source, document, locus, *position)
        }
        SemanticEdit::ResizeOverlay { position, scale } => {
            apply_resize_overlay(source, document, locus, *position, *scale)
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
        return splice_title(source, inv, "title", text, at, duration, opacity);
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
    word: &str,
    text: Option<&String>,
    at: Option<Time>,
    duration: Option<Time>,
    opacity: Option<u8>,
) -> Result<String, EngineError> {
    let mut splices = Vec::new();
    if let Some(text) = text {
        let span = string_arg_span(inv)
            .ok_or_else(|| EngineError::Edit(format!("{word} has no string argument")))?;
        splices.push((span, quote_vel_string(text)));
    }
    if let Some(at) = at {
        let span = modifier_value_span(inv, "at")
            .ok_or_else(|| EngineError::Edit(format!("{word} has no `at` modifier")))?;
        splices.push((span, at.to_string()));
    }
    if let Some(duration) = duration {
        if duration < Time::ZERO {
            return Err(EngineError::Edit("duration must not be negative".into()));
        }
        let span = modifier_value_span(inv, "for")
            .ok_or_else(|| EngineError::Edit(format!("{word} has no `for` modifier")))?;
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
            return Err(EngineError::Edit(format!(
                "{word} has no body to attach opacity"
            )));
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
        let span = include_leading_minus(source, span);
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

fn apply_callout(
    source: &str,
    document: &Document,
    locus: &Locus,
    text: Option<&String>,
    at: Option<Time>,
    duration: Option<Time>,
) -> Result<String, EngineError> {
    if text.is_none() && at.is_none() && duration.is_none() {
        return Err(EngineError::Edit("callout edit has no fields".into()));
    }
    if duration.is_some_and(|time| time < Time::ZERO) {
        return Err(EngineError::Edit("duration must not be negative".into()));
    }
    let inv = find_invocation(document, locus, "callout").ok_or_else(|| {
        EngineError::Edit("callout locus did not match a callout invocation".into())
    })?;
    splice_title(source, inv, "callout", text, at, duration, None)
}

fn apply_position(
    source: &str,
    document: &Document,
    locus: &Locus,
    position: NormalizedPosition,
) -> Result<String, EngineError> {
    let word = match locus.kind {
        LocusKind::Title => "title",
        LocusKind::Callout => "callout",
        _ => {
            return Err(EngineError::Edit(
                "canvas position needs a title or callout locus".into(),
            ));
        }
    };
    let inv = find_invocation(document, locus, word).ok_or_else(|| {
        EngineError::Edit(format!("{word} locus did not match its source invocation"))
    })?;
    let replacement = format_position(position);
    if let Some(span) = position_arg_span(inv) {
        return Ok(apply_splices(source, vec![(span, replacement)]));
    }
    Ok(attach_overlay_body_lines(
        source,
        inv,
        &format!("    position {replacement}\n"),
    ))
}

fn apply_resize_overlay(
    source: &str,
    document: &Document,
    locus: &Locus,
    position: NormalizedPosition,
    scale: NormalizedScale,
) -> Result<String, EngineError> {
    let (_word, inv) = visual_invocation(document, locus)?;
    let mut splices = Vec::new();
    let mut insert = String::new();
    let position = format_position(position);
    if let Some(span) = position_arg_span(inv) {
        splices.push((span, position));
    } else {
        insert.push_str("    position ");
        insert.push_str(&position);
        insert.push('\n');
    }
    let scale = format_scale(scale);
    if let Some(span) = scale_arg_span(inv) {
        splices.push((span, scale));
    } else {
        insert.push_str("    scale ");
        insert.push_str(&scale);
        insert.push('\n');
    }
    if !insert.is_empty() {
        return Ok(attach_overlay_body_lines_with(
            source, inv, splices, &insert,
        ));
    }
    Ok(apply_splices(source, splices))
}

fn attach_overlay_body_lines(source: &str, inv: &Invocation, lines: &str) -> String {
    attach_overlay_body_lines_with(source, inv, Vec::new(), lines)
}

fn attach_overlay_body_lines_with(
    source: &str,
    inv: &Invocation,
    mut splices: Vec<(Span, String)>,
    lines: &str,
) -> String {
    if let Some(body) = &inv.body {
        let insert_at = Span::new(
            body.span.end.saturating_sub(1),
            body.span.end.saturating_sub(1),
            body.span.line,
            body.span.column,
        );
        splices.push((insert_at, format!("{lines}  ")));
    } else {
        let insert_at = Span::new(inv.span.end, inv.span.end, inv.span.line, inv.span.column);
        splices.push((insert_at, format!(" {{\n{lines}  }}")));
    }
    apply_splices(source, splices)
}

fn visual_invocation<'a>(
    document: &'a Document,
    locus: &Locus,
) -> Result<(&'static str, &'a Invocation), EngineError> {
    let word = match locus.kind {
        LocusKind::Title => "title",
        LocusKind::Callout => "callout",
        _ => {
            return Err(EngineError::Edit(
                "canvas resize needs a title or callout locus".into(),
            ));
        }
    };
    let inv = find_invocation(document, locus, word).ok_or_else(|| {
        EngineError::Edit(format!("{word} locus did not match its source invocation"))
    })?;
    Ok((word, inv))
}

fn apply_reorder(
    source: &str,
    document: &Document,
    locus: &Locus,
    before: Option<&str>,
) -> Result<String, EngineError> {
    let scene_name = scene_name_of(locus)
        .ok_or_else(|| EngineError::Edit("reorder needs a scene identity".into()))?;
    let Item::Sequence { body, .. } = find_sequence_for(document, &scene_name)
        .ok_or_else(|| EngineError::Edit("reorder target is not in a sequence".into()))?
    else {
        return Err(EngineError::Edit("reorder expected a sequence".into()));
    };
    let refs: Vec<(&str, Span)> = body
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Invocation(inv) if inv.args.is_empty() => Some((inv.name.as_str(), inv.span)),
            _ => None,
        })
        .collect();
    if refs.is_empty() {
        return Err(EngineError::Edit(
            "sequence has no scenes to reorder".into(),
        ));
    }
    let from = refs
        .iter()
        .position(|(name, _)| *name == scene_name)
        .ok_or_else(|| EngineError::Edit(format!("scene `{scene_name}` is not in the sequence")))?;
    let mut names: Vec<&str> = refs.iter().map(|(name, _)| *name).collect();
    let moved = names.remove(from);
    let insert_at = match before {
        Some(target) => names
            .iter()
            .position(|name| *name == target)
            .ok_or_else(|| EngineError::Edit(format!("reorder before unknown scene `{target}`")))?,
        None => names.len(),
    };
    names.insert(insert_at, moved);
    let original: Vec<&str> = refs.iter().map(|(name, _)| *name).collect();
    if names == original {
        return Err(EngineError::Edit(
            "reorder does not change scene order".into(),
        ));
    }
    let splices = refs
        .iter()
        .zip(names)
        .map(|((_, span), name)| (*span, name.to_string()))
        .collect();
    Ok(apply_splices(source, splices))
}

fn scene_name_of(locus: &Locus) -> Option<String> {
    if locus.kind == LocusKind::Scene {
        return Some(locus.label.clone());
    }
    if let Some(id) = &locus.scene_id {
        if let Some(name) = id.strip_prefix("scene:") {
            return Some(name.to_string());
        }
        return Some(id.clone());
    }
    if let Some(name) = locus.node_id.strip_prefix("scene:") {
        return Some(name.to_string());
    }
    None
}

fn find_sequence_for<'a>(document: &'a Document, scene_name: &str) -> Option<&'a Item> {
    walk_items(&document.items).find(|item| {
        let Item::Sequence { body, .. } = item else {
            return false;
        };
        body.items.iter().any(|inner| {
            matches!(inner, Item::Invocation(inv) if inv.args.is_empty() && inv.name == scene_name)
        })
    })
}

fn find_invocation<'a>(
    document: &'a Document,
    locus: &Locus,
    name: &str,
) -> Option<&'a Invocation> {
    let want = locus.source_span?;
    for item in walk_items(&document.items) {
        let Item::Invocation(inv) = item else {
            continue;
        };
        if inv.name != name {
            continue;
        }
        if inv.span.start == want.start && inv.span.end == want.end {
            return Some(inv);
        }
    }
    None
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

/// Include a unary minus immediately before `span` so `gain … by -3` + `SetGain { -3 }`
/// cannot splice into `by --3`. The VEL quantity span should already cover the sign;
/// this is the Engine-side belt for any leftover exclusive-sign span.
fn include_leading_minus(source: &str, span: Span) -> Span {
    let start = usize::try_from(span.start).unwrap_or(0);
    if start == 0 {
        return span;
    }
    let Some(prefix) = source.get(..start) else {
        return span;
    };
    let trimmed = prefix.trim_end_matches([' ', '\t']);
    if trimmed.as_bytes().last() == Some(&b'-')
        && trimmed
            .len()
            .checked_sub(1)
            .is_none_or(|i| !trimmed.as_bytes()[i].is_ascii_digit())
    {
        let minus_at = u32::try_from(trimmed.len() - 1).unwrap_or(span.start);
        return Span::new(
            minus_at,
            span.end,
            span.line,
            span.column
                .saturating_sub(span.start.saturating_sub(minus_at)),
        );
    }
    span
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

fn position_arg_span(inv: &Invocation) -> Option<Span> {
    let body = inv.body.as_ref()?;
    body.items.iter().find_map(|item| match item {
        Item::Invocation(inner) if inner.name == "position" => inner.args.first().map(Expr::span),
        _ => None,
    })
}

fn scale_arg_span(inv: &Invocation) -> Option<Span> {
    let body = inv.body.as_ref()?;
    body.items.iter().find_map(|item| match item {
        Item::Invocation(inner) if inner.name == "scale" => inner.args.first().map(Expr::span),
        _ => None,
    })
}

fn format_position(position: NormalizedPosition) -> String {
    format!(
        "({}, {})",
        format_percent(position.x),
        format_percent(position.y)
    )
}

fn format_percent(basis_points: u16) -> String {
    let whole = basis_points / 100;
    let fraction = basis_points % 100;
    if fraction == 0 {
        format!("{whole}%")
    } else if fraction.is_multiple_of(10) {
        format!("{whole}.{}%", fraction / 10)
    } else {
        format!("{whole}.{fraction:02}%")
    }
}

fn format_scale(scale: NormalizedScale) -> String {
    let whole = scale.milli / 10;
    let fraction = scale.milli % 10;
    if fraction == 0 {
        format!("{whole}%")
    } else {
        format!("{whole}.{fraction}%")
    }
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

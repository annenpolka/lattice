use lattice_core::{
    Audio, Media, MediaLocator, Origin, Placement, PlacementKind, Provenance, Source, Time,
    TimeMap, TimeSpan, Visual,
};

use crate::caption::merge_caption_timing;
use crate::overlay_body::{
    OverlayConvention, overlay_explain_notes, parse_overlay_body, parse_overlay_body_for,
};
use crate::view::{BodyItem, InvocationView, LoweringError, SceneDraft, ValueView};

pub fn lower_freeze(inv: &InvocationView, draft: &mut SceneDraft) -> Result<(), LoweringError> {
    let target = inv
        .args
        .first()
        .and_then(super::view::ValueView::as_name)
        .ok_or_else(|| LoweringError::Message("`freeze` needs a source name".into()))?;
    let at = inv
        .modifier("at")
        .and_then(super::view::ValueView::as_time)
        .ok_or_else(|| LoweringError::Message("`freeze` needs `at`".into()))?;
    let hold = inv
        .modifier("for")
        .and_then(super::view::ValueView::as_time)
        .ok_or_else(|| LoweringError::Message("`freeze` needs `for`".into()))?;
    let source = draft.source_mut(target)?;
    let updated = source
        .time_map
        .with_freeze(at, hold)
        .map_err(|err| LoweringError::Message(err.to_string()))?;
    source.time_map = updated;
    draft.explain(
        Origin::Invocation {
            command: "freeze".into(),
        },
        format!("freeze `{target}` at {at} for {hold} -> TimeMap hold (rate 0)"),
    );
    Ok(())
}

pub fn lower_title(inv: &InvocationView, draft: &mut SceneDraft) -> Result<(), LoweringError> {
    let text = inv
        .args
        .first()
        .and_then(super::view::ValueView::as_string)
        .ok_or_else(|| LoweringError::Message("`title` needs a string".into()))?
        .to_string();
    let at = inv
        .modifier("at")
        .and_then(super::view::ValueView::as_time)
        .unwrap_or(Time::ZERO);
    let hold = inv
        .modifier("for")
        .and_then(super::view::ValueView::as_time)
        .unwrap_or(Time::seconds(3));
    let opacity = body_opacity(inv);
    let parsed = parse_overlay_body(inv, draft);
    let id = draft.next_placement_id("title");
    let mut visual = Visual::text_overlay(text.clone());
    visual.opacity = opacity;
    visual.position = parsed.position;
    visual.scale = parsed.scale;
    visual.anchor = parsed.anchor;
    visual.style = parsed.style.clone().into_option();
    draft.placements.push(Placement {
        id,
        kind: PlacementKind::Title,
        source_id: None,
        span: TimeSpan::new(at, hold),
        visual: Some(visual),
        audio: None,
        provenance: Provenance::invocation("title", Some(inv.span)),
    });
    let opacity_note = opacity.map_or(String::new(), |value| format!(" opacity {value}"));
    let preset_note = parsed
        .applied_preset
        .map(|name| format!(" using {name}"))
        .unwrap_or_default();
    let style_notes = overlay_explain_notes(
        parsed.position,
        parsed.scale,
        parsed.anchor,
        &parsed.style,
        OverlayConvention::Title,
    );
    draft.explain(
        Origin::Invocation {
            command: "title".into(),
        },
        format!("title {text:?} at {at} for {hold}{opacity_note}{preset_note}{style_notes}"),
    );
    Ok(())
}

pub fn lower_caption(inv: &InvocationView, draft: &mut SceneDraft) -> Result<(), LoweringError> {
    let text = inv
        .args
        .first()
        .and_then(ValueView::as_string)
        .ok_or_else(|| LoweringError::Message("`caption` needs a string".into()))?
        .to_string();
    let Some(timing) = merge_caption_timing(inv, draft) else {
        return Ok(());
    };
    let at = timing.at;
    let hold = timing.hold;
    let opacity = body_opacity(inv);
    let parsed = parse_overlay_body_for(inv, draft, OverlayConvention::Caption);
    let id = draft.next_placement_id("caption");
    let mut visual = Visual::text_overlay(text.clone());
    visual.opacity = opacity;
    visual.position = parsed.position;
    visual.scale = parsed.scale;
    visual.anchor = parsed.anchor;
    visual.style = parsed.style.clone().into_option();
    draft.placements.push(Placement {
        id,
        kind: PlacementKind::Title,
        source_id: None,
        span: TimeSpan::new(at, hold),
        visual: Some(visual),
        audio: None,
        provenance: Provenance::invocation("caption", Some(inv.span)),
    });
    let opacity_note = opacity.map_or(String::new(), |value| format!(" opacity {value}"));
    let style_notes = overlay_explain_notes(
        parsed.position,
        parsed.scale,
        parsed.anchor,
        &parsed.style,
        OverlayConvention::Caption,
    );
    draft.explain(
        Origin::Invocation {
            command: "caption".into(),
        },
        format!("caption {text:?} at {at} for {hold}{opacity_note}{style_notes}"),
    );
    Ok(())
}

pub fn lower_callout(inv: &InvocationView, draft: &mut SceneDraft) -> Result<(), LoweringError> {
    let text = inv
        .args
        .first()
        .and_then(ValueView::as_string)
        .ok_or_else(|| LoweringError::Message("`callout` needs a string".into()))?
        .to_string();
    let at = inv
        .modifier("at")
        .and_then(ValueView::as_time)
        .unwrap_or(Time::ZERO);
    let hold = inv
        .modifier("for")
        .and_then(ValueView::as_time)
        .unwrap_or(Time::seconds(2));
    let id = draft.next_placement_id("callout");
    let parsed = parse_overlay_body_for(inv, draft, OverlayConvention::Callout);
    let mut visual = Visual::text_overlay(text.clone());
    visual.position = parsed.position;
    visual.scale = parsed.scale;
    visual.anchor = parsed.anchor;
    visual.style = parsed.style.clone().into_option();
    draft.placements.push(Placement {
        id,
        kind: PlacementKind::Callout,
        source_id: None,
        span: TimeSpan::new(at, hold),
        visual: Some(visual),
        audio: None,
        provenance: Provenance::invocation("callout", Some(inv.span)),
    });
    let style_notes = overlay_explain_notes(
        parsed.position,
        parsed.scale,
        parsed.anchor,
        &parsed.style,
        OverlayConvention::Callout,
    );
    draft.explain(
        Origin::Invocation {
            command: "callout".into(),
        },
        format!("callout {text:?} at {at} for {hold}{style_notes}"),
    );
    Ok(())
}

pub fn lower_fade(inv: &InvocationView, draft: &mut SceneDraft) -> Result<(), LoweringError> {
    let target = inv
        .args
        .first()
        .and_then(ValueView::as_name)
        .ok_or_else(|| LoweringError::Message("`fade` needs a source name".into()))?;
    let hold = inv
        .modifier("for")
        .and_then(ValueView::as_time)
        .ok_or_else(|| LoweringError::Message("`fade` needs `for`".into()))?;
    draft.source_mut(target)?;
    draft.source_fade_in.push((target.to_string(), hold));
    draft.explain(
        Origin::Invocation {
            command: "fade".into(),
        },
        format!("fade `{target}` in over {hold} (opacity envelope on video placement)"),
    );
    Ok(())
}

pub fn lower_gain(inv: &InvocationView, draft: &mut SceneDraft) -> Result<(), LoweringError> {
    let target = inv
        .args
        .first()
        .and_then(ValueView::as_name)
        .ok_or_else(|| LoweringError::Message("`gain` needs a source name".into()))?;
    let db = inv
        .modifier("by")
        .and_then(ValueView::as_int)
        .or_else(|| inv.args.get(1).and_then(ValueView::as_int))
        .ok_or_else(|| LoweringError::Message("`gain` needs `by` (dB)".into()))?;
    let db =
        i32::try_from(db).map_err(|_| LoweringError::Message("gain dB out of range".into()))?;
    draft.source_mut(target)?;
    draft.source_gain_db.push((target.to_string(), db));
    draft.explain(
        Origin::Invocation {
            command: "gain".into(),
        },
        format!("gain `{target}` {db} dB"),
    );
    Ok(())
}

pub fn lower_speech(inv: &InvocationView, draft: &mut SceneDraft) -> Result<(), LoweringError> {
    let text = inv
        .args
        .first()
        .and_then(ValueView::as_string)
        .ok_or_else(|| LoweringError::Message("`speech` needs a string".into()))?
        .to_string();
    let at = inv
        .modifier("at")
        .and_then(ValueView::as_time)
        .unwrap_or(Time::ZERO);
    let hold = inv
        .modifier("for")
        .and_then(ValueView::as_time)
        .unwrap_or(Time::seconds(2));
    let slug = speech_slug(&text);
    let media_name = format!("speech-{slug}");
    draft.media.push(Media {
        id: format!("media:{media_name}"),
        name: media_name.clone(),
        locator: MediaLocator::Generated {
            generator: "speech".into(),
            key: text.clone(),
        },
    });
    let source_id = format!("source:{media_name}");
    draft.sources.push(Source {
        id: source_id.clone(),
        name: media_name.clone(),
        media_name: media_name.clone(),
        source_range: TimeSpan::new(Time::ZERO, hold),
        time_map: TimeMap::identity(Time::ZERO, hold),
        provenance: Provenance::invocation("speech", Some(inv.span)),
        generated: true,
    });
    let id = draft.next_placement_id("speech");
    draft.placements.push(Placement {
        id,
        kind: PlacementKind::Audio,
        source_id: Some(source_id),
        span: TimeSpan::new(at, hold),
        visual: None,
        audio: Some(Audio { gain_db: None }),
        provenance: Provenance::invocation("speech", Some(inv.span)),
    });
    draft.explain(
        Origin::Invocation {
            command: "speech".into(),
        },
        format!(
            "speech {text:?} at {at} for {hold} -> generated media `{media_name}` (Resolve materializes; Compile does not)"
        ),
    );
    Ok(())
}

fn body_opacity(inv: &InvocationView) -> Option<u8> {
    inv.body.iter().find_map(|item| match item {
        BodyItem::Invocation(inner) if inner.command == "opacity" => inner
            .args
            .first()
            .and_then(ValueView::as_int)
            .and_then(|value| u8::try_from(value).ok()),
        _ => None,
    })
}

fn speech_slug(text: &str) -> String {
    let mut slug = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
        if slug.len() >= 32 {
            break;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() { "line".into() } else { slug }
}

pub fn apply_commentary(draft: &mut SceneDraft) {
    if draft.sources.is_empty() {
        return;
    }
    let duck = draft.over.is_some();
    let scene_len = super::view::scene_duration(draft);
    let gain = if duck { Some(-15) } else { None };
    let sources = draft.sources.clone();
    for source in sources {
        let video_id = draft.next_placement_id("video");
        if source.generated {
            continue;
        }
        let fade_in = draft
            .source_fade_in
            .iter()
            .rev()
            .find(|(name, _)| name == &source.name)
            .map(|(_, time)| *time);
        let gain_db = draft
            .source_gain_db
            .iter()
            .rev()
            .find(|(name, _)| name == &source.name)
            .map(|(_, db)| *db)
            .or(gain);
        let mut visual = Visual::fit("canvas-fill");
        visual.fade_in = fade_in;
        draft.placements.push(Placement {
            id: video_id,
            kind: PlacementKind::Video,
            source_id: Some(source.id.clone()),
            span: TimeSpan::new(Time::ZERO, scene_len.max(source.time_map.duration)),
            visual: Some(visual),
            audio: None,
            provenance: Provenance::convention("commentary"),
        });
        draft.placements.push(Placement {
            id: draft.next_placement_id("audio"),
            kind: PlacementKind::Audio,
            source_id: Some(source.id),
            span: TimeSpan::new(Time::ZERO, scene_len.max(source.time_map.duration)),
            visual: None,
            audio: Some(Audio { gain_db }),
            provenance: Provenance::convention("commentary"),
        });
        let duck_text = if duck {
            "game audio -> -15dB (under narration)"
        } else {
            "game audio -> passthrough"
        };
        draft.explain(
            Origin::Convention {
                name: "commentary".into(),
            },
            format!("source `{}` video -> canvas-fill; {duck_text}", source.name),
        );
    }
}

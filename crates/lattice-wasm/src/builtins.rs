use lattice_core::{Audio, Origin, Placement, PlacementKind, Provenance, Time, TimeSpan, Visual};

use crate::view::{InvocationView, LoweringError, SceneDraft};

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
    let id = draft.next_placement_id("title");
    draft.placements.push(Placement {
        id,
        kind: PlacementKind::Title,
        source_id: None,
        span: TimeSpan::new(at, hold),
        visual: Some(Visual {
            fit: None,
            text: Some(text.clone()),
        }),
        audio: None,
        provenance: Provenance::invocation("title", Some(inv.span)),
    });
    draft.explain(
        Origin::Invocation {
            command: "title".into(),
        },
        format!("title {text:?} at {at} for {hold}"),
    );
    Ok(())
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
        draft.placements.push(Placement {
            id: video_id,
            kind: PlacementKind::Video,
            source_id: Some(source.id.clone()),
            span: TimeSpan::new(Time::ZERO, scene_len.max(source.time_map.duration)),
            visual: Some(Visual {
                fit: Some("canvas-fill".into()),
                text: None,
            }),
            audio: None,
            provenance: Provenance::convention("commentary"),
        });
        draft.placements.push(Placement {
            id: draft.next_placement_id("audio"),
            kind: PlacementKind::Audio,
            source_id: Some(source.id),
            span: TimeSpan::new(Time::ZERO, scene_len.max(source.time_map.duration)),
            visual: None,
            audio: Some(Audio { gain_db: gain }),
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

//! Pointer begin / update / commit / cancel against a Studio session.

use lattice_engine::{Engine, EngineError, LocusId, LocusKind, SemanticEdit, Time, TimeSpan};

use crate::gesture::{
    ClipKind, CursorKind, Edge, GestureOutcome, HitClip, SNAP_THRESHOLD_PX, TimelineGesture,
    TimelineHit, clip_kind_from_track, clips_at_x, crossed_drag_threshold, cursor_for_hit,
    hit_test, nearest_frame, overlay_duration_valid, preview_overlay_move, preview_overlay_resize,
    preview_trim, reorder_index, snap_time,
};
use crate::session::StudioSession;
use crate::verb::{Projection, UnresolvedPointing, refuse_edit};
use crate::viewport::clamp_interaction_time;

pub fn begin(
    session: &mut StudioSession,
    x: f64,
    snap_off: bool,
    track: Option<&str>,
) -> Result<(), EngineError> {
    session.last_gesture_error = None;
    session.snap_time = None;
    session.touch_projection(Projection::Timeline);
    let clips = hit_clips_on_track(session, track)?;
    let hit = hit_test(&clips, x, session.viewport);
    session.gesture = match hit {
        TimelineHit::Rail => TimelineGesture::Point { start_x: x },
        TimelineHit::Trim {
            clip_id,
            edge,
            kind,
        } => match kind {
            ClipKind::Video => begin_trim(session, &clips, &clip_id, edge, x)?,
            ClipKind::Title | ClipKind::Callout => {
                begin_resize_overlay(session, &clips, &clip_id, kind, edge, x)?
            }
            ClipKind::Other => TimelineGesture::Scrub {
                start_playhead: session.playhead,
                start_x: x,
            },
        },
        TimelineHit::ClipBody { clip_id, kind } => match kind {
            ClipKind::Video => begin_reorder(session, &clips, &clip_id, x)?,
            ClipKind::Title | ClipKind::Callout => {
                begin_move_overlay(session, &clips, &clip_id, kind, x)?
            }
            ClipKind::Other => TimelineGesture::Scrub {
                start_playhead: session.playhead,
                start_x: x,
            },
        },
    };
    if matches!(session.gesture, TimelineGesture::Scrub { .. }) {
        update_scrub(session, x, snap_off);
    }
    let _ = snap_off;
    Ok(())
}

#[allow(clippy::too_many_lines)]
pub fn update(session: &mut StudioSession, x: f64, snap_off: bool) -> Result<(), EngineError> {
    session.snap_time = None;
    match session.gesture.clone() {
        TimelineGesture::None => Ok(()),
        TimelineGesture::Point { start_x } => {
            if crossed_drag_threshold(start_x, x) {
                session.gesture = TimelineGesture::Scrub {
                    start_playhead: session.playhead,
                    start_x,
                };
                update_scrub(session, x, snap_off);
            }
            Ok(())
        }
        TimelineGesture::Scrub { .. } => {
            update_scrub(session, x, snap_off);
            Ok(())
        }
        TimelineGesture::Trim {
            edge,
            original_in,
            original_out,
            timeline_start,
            start_x,
            ..
        } => {
            let delta = snapped_delta(
                session,
                start_x,
                x,
                snap_off,
                Some(original_in),
                Some(original_out),
            );
            let (preview_in, preview_out) = preview_trim(original_in, original_out, edge, delta);
            if let TimelineGesture::Trim {
                preview_in: pin,
                preview_out: pout,
                ..
            } = &mut session.gesture
            {
                *pin = preview_in;
                *pout = preview_out;
            }
            let playhead = match edge {
                Edge::Left => timeline_start
                    .checked_add(preview_in.checked_sub(original_in).unwrap_or(Time::ZERO))
                    .unwrap_or(timeline_start),
                Edge::Right => timeline_start
                    .checked_add(preview_out.checked_sub(original_in).unwrap_or(Time::ZERO))
                    .unwrap_or(timeline_start),
            };
            session.playhead = clamp_interaction_time(playhead, session.timeline_duration());
            Ok(())
        }
        TimelineGesture::Reorder {
            clip_id,
            original_index,
            start_x,
            ..
        } => {
            let clips = video_scene_clips(session)?;
            let at = snapped_time(session, session.viewport.time_at_x(x), snap_off);
            let spans: Vec<(Time, Time)> =
                clips.iter().map(|clip| (clip.start, clip.end())).collect();
            let proposed = reorder_index(&spans, original_index, at);
            let moved = crossed_drag_threshold(start_x, x) || proposed != original_index;
            if let TimelineGesture::Reorder {
                proposed_index,
                moved: flag,
                ..
            } = &mut session.gesture
            {
                *proposed_index = proposed;
                *flag = moved;
            }
            let _ = clip_id;
            Ok(())
        }
        TimelineGesture::MoveOverlay {
            original,
            grab_offset,
            start_x,
            scene_offset,
            ..
        } => {
            let raw = session.viewport.time_at_x(x);
            let mut start = raw.checked_sub(grab_offset).unwrap_or(Time::ZERO);
            start = snapped_time(session, start, snap_off);
            let preview = preview_overlay_move(original, start);
            let moved = crossed_drag_threshold(start_x, x);
            if let TimelineGesture::MoveOverlay {
                preview: slot,
                moved: flag,
                ..
            } = &mut session.gesture
            {
                *slot = preview;
                *flag = moved;
            }
            let _ = scene_offset;
            Ok(())
        }
        TimelineGesture::ResizeOverlay {
            edge,
            original,
            start_x,
            scene_offset,
            ..
        } => {
            let delta = snapped_delta(
                session,
                start_x,
                x,
                snap_off,
                Some(original.start),
                Some(original.end()),
            );
            let preview = preview_overlay_resize(original, edge, delta);
            if let TimelineGesture::ResizeOverlay { preview: slot, .. } = &mut session.gesture {
                *slot = preview;
            }
            let playhead = match edge {
                Edge::Left => preview
                    .start
                    .checked_add(scene_offset)
                    .unwrap_or(preview.start),
                Edge::Right => preview
                    .end()
                    .checked_add(scene_offset)
                    .unwrap_or(preview.end()),
            };
            session.playhead = clamp_interaction_time(playhead, session.timeline_duration());
            Ok(())
        }
    }
}

#[allow(clippy::too_many_lines)]
pub fn commit(
    session: &mut StudioSession,
    x: f64,
    snap_off: bool,
) -> Result<GestureOutcome, EngineError> {
    let _ = update(session, x, snap_off);
    let gesture = std::mem::replace(&mut session.gesture, TimelineGesture::None);
    session.snap_time = None;
    match gesture {
        TimelineGesture::None => Ok(GestureOutcome::Idle),
        TimelineGesture::Point { .. } => {
            let time = session.viewport.time_at_x(x);
            session.point_from_timeline_time(time)?;
            Ok(GestureOutcome::Clicked)
        }
        TimelineGesture::Scrub { .. } => {
            // Playhead is not here. Scrub must not re-point.
            Ok(GestureOutcome::Scrubbed)
        }
        TimelineGesture::Trim {
            clip_id,
            original_in,
            original_out,
            preview_in,
            preview_out,
            scene_id,
            ..
        } => {
            session.point_source_for_clip(&clip_id)?;
            let _ = scene_id;
            if preview_in == original_in && preview_out == original_out {
                return Ok(GestureOutcome::Clicked);
            }
            apply_committed(
                session,
                SemanticEdit::Trim {
                    in_point: (preview_in != original_in).then_some(preview_in),
                    out_point: (preview_out != original_out).then_some(preview_out),
                },
            )
            .map(|()| GestureOutcome::Applied)
        }
        TimelineGesture::Reorder {
            clip_id,
            scene_id,
            original_index,
            proposed_index,
            moved,
            ..
        } => {
            let _ = scene_id;
            if !moved || proposed_index == original_index {
                session.point_source_for_clip(&clip_id)?;
                return Ok(GestureOutcome::Clicked);
            }
            let here = session.current_locus()?;
            if here
                .as_ref()
                .is_none_or(|locus| locus.kind != LocusKind::Scene)
            {
                let loci = session.loci().unwrap_or_default();
                let spoken = refuse_edit(
                    here.as_ref(),
                    &SemanticEdit::ReorderScene { before: None },
                    &loci,
                );
                session.last_spoken = Some(spoken.clone());
                session.last_gesture_error = Some(spoken);
                return Ok(GestureOutcome::Failed);
            }
            let names = scene_names(session);
            if names.is_empty() {
                return Ok(GestureOutcome::Clicked);
            }
            let mut order = names.clone();
            if original_index >= order.len() {
                return Ok(GestureOutcome::Clicked);
            }
            let moved_name = order.remove(original_index);
            let insert_at = proposed_index.min(order.len());
            order.insert(insert_at, moved_name);
            if order == names {
                return Ok(GestureOutcome::Clicked);
            }
            let before = order.get(insert_at + 1).cloned();
            apply_committed(session, SemanticEdit::ReorderScene { before })
                .map(|()| GestureOutcome::Applied)
        }
        TimelineGesture::MoveOverlay {
            clip_id,
            kind,
            original,
            preview,
            moved,
            scene_offset,
            ..
        } => {
            if !moved || preview == original {
                point_overlay_or_overlap(session, &clip_id, x)?;
                return Ok(GestureOutcome::Clicked);
            }
            point_clip(session, &clip_id);
            if !overlay_duration_valid(preview) {
                session.last_gesture_error = Some("duration must not be negative".into());
                return Ok(GestureOutcome::Failed);
            }
            let at = local_at(preview.start, scene_offset);
            commit_overlay(session, kind, Some(at), None)
        }
        TimelineGesture::ResizeOverlay {
            clip_id,
            kind,
            original,
            preview,
            scene_offset,
            ..
        } => {
            if preview == original {
                point_overlay_or_overlap(session, &clip_id, x)?;
                return Ok(GestureOutcome::Clicked);
            }
            point_clip(session, &clip_id);
            if !overlay_duration_valid(preview) {
                session.last_gesture_error = Some("duration must not be negative".into());
                return Ok(GestureOutcome::Failed);
            }
            let at = local_at(preview.start, scene_offset);
            let at_changed = preview.start != original.start;
            commit_overlay(
                session,
                kind,
                at_changed.then_some(at),
                Some(preview.duration),
            )
        }
    }
}

pub fn cancel(session: &mut StudioSession) -> GestureOutcome {
    if session.gesture.is_none() {
        return GestureOutcome::Idle;
    }
    session.gesture = TimelineGesture::None;
    session.snap_time = None;
    session.last_gesture_error = None;
    GestureOutcome::Cancelled
}

pub fn apply_committed(session: &mut StudioSession, edit: SemanticEdit) -> Result<(), EngineError> {
    session.gesture = TimelineGesture::None;
    session.snap_time = None;
    match session.apply_edit(edit) {
        Ok(()) => {
            session.last_gesture_error = None;
            Ok(())
        }
        Err(err) => {
            session.last_gesture_error = Some(err.to_string());
            Err(err)
        }
    }
}

pub fn cursor_at(session: &StudioSession, x: f64) -> CursorKind {
    cursor_at_on_track(session, x, None)
}

pub fn cursor_at_on_track(session: &StudioSession, x: f64, track: Option<&str>) -> CursorKind {
    if session.gesture.is_active() {
        return match &session.gesture {
            TimelineGesture::Trim { .. } | TimelineGesture::ResizeOverlay { .. } => {
                CursorKind::Trim
            }
            TimelineGesture::Reorder { .. } | TimelineGesture::MoveOverlay { .. } => {
                CursorKind::Grabbing
            }
            TimelineGesture::Scrub { .. } => CursorKind::Scrub,
            TimelineGesture::Point { .. } | TimelineGesture::None => CursorKind::Select,
        };
    }
    let Ok(clips) = hit_clips_on_track(session, track) else {
        return CursorKind::Select;
    };
    let hit = hit_test(&clips, x, session.viewport);
    cursor_for_hit(&hit, false)
}

fn begin_trim(
    session: &StudioSession,
    clips: &[HitClip],
    clip_id: &str,
    edge: Edge,
    x: f64,
) -> Result<TimelineGesture, EngineError> {
    let clip = clips
        .iter()
        .find(|clip| clip.id == clip_id)
        .ok_or_else(|| EngineError::Edit("trim clip missing".into()))?;
    let range = source_range(session, clip_id)
        .ok_or_else(|| EngineError::Edit("trim clip has no source range".into()))?;
    Ok(TimelineGesture::Trim {
        clip_id: clip_id.to_string(),
        edge,
        original_in: range.start,
        original_out: range.end(),
        preview_in: range.start,
        preview_out: range.end(),
        timeline_start: clip.start,
        start_x: x,
        scene_id: clip.scene_id.clone(),
    })
}

fn begin_reorder(
    session: &StudioSession,
    clips: &[HitClip],
    clip_id: &str,
    x: f64,
) -> Result<TimelineGesture, EngineError> {
    let video: Vec<_> = clips
        .iter()
        .filter(|clip| clip.kind == ClipKind::Video)
        .collect();
    let original_index = video
        .iter()
        .position(|clip| clip.id == clip_id)
        .ok_or_else(|| EngineError::Edit("reorder clip missing".into()))?;
    let scene_id = video[original_index].scene_id.clone();
    let _ = session;
    Ok(TimelineGesture::Reorder {
        clip_id: clip_id.to_string(),
        scene_id,
        original_index,
        proposed_index: original_index,
        start_x: x,
        moved: false,
    })
}

fn begin_move_overlay(
    session: &StudioSession,
    clips: &[HitClip],
    clip_id: &str,
    kind: ClipKind,
    x: f64,
) -> Result<TimelineGesture, EngineError> {
    let clip = clips
        .iter()
        .find(|clip| clip.id == clip_id)
        .ok_or_else(|| EngineError::Edit("overlay clip missing".into()))?;
    let original = TimeSpan::new(clip.start, clip.duration);
    let at = session.viewport.time_at_x(x);
    let grab_offset = at.checked_sub(clip.start).unwrap_or(Time::ZERO);
    let scene_offset = overlay_scene_offset(session, clip_id).unwrap_or(Time::ZERO);
    Ok(TimelineGesture::MoveOverlay {
        clip_id: clip_id.to_string(),
        kind,
        original,
        preview: original,
        grab_offset,
        start_x: x,
        moved: false,
        scene_offset,
    })
}

fn begin_resize_overlay(
    session: &StudioSession,
    clips: &[HitClip],
    clip_id: &str,
    kind: ClipKind,
    edge: Edge,
    x: f64,
) -> Result<TimelineGesture, EngineError> {
    let clip = clips
        .iter()
        .find(|clip| clip.id == clip_id)
        .ok_or_else(|| EngineError::Edit("overlay clip missing".into()))?;
    let original = TimeSpan::new(clip.start, clip.duration);
    let scene_offset = overlay_scene_offset(session, clip_id).unwrap_or(Time::ZERO);
    Ok(TimelineGesture::ResizeOverlay {
        clip_id: clip_id.to_string(),
        kind,
        edge,
        original,
        preview: original,
        start_x: x,
        scene_offset,
    })
}

fn update_scrub(session: &mut StudioSession, x: f64, snap_off: bool) {
    let raw = session.viewport.time_at_x(x);
    let time = snapped_time(session, raw, snap_off);
    session.playhead = clamp_interaction_time(time, session.timeline_duration());
}

fn snapped_time(session: &mut StudioSession, raw: Time, snap_off: bool) -> Time {
    if snap_off {
        return raw;
    }
    let candidates = snap_candidates(session, raw);
    match snap_time(raw, &candidates, session.viewport, SNAP_THRESHOLD_PX) {
        Some((snapped, target)) => {
            session.snap_time = Some(target);
            snapped
        }
        None => raw,
    }
}

fn snapped_delta(
    session: &mut StudioSession,
    start_x: f64,
    x: f64,
    snap_off: bool,
    extra_a: Option<Time>,
    extra_b: Option<Time>,
) -> Time {
    let raw = session.viewport.delta_time(x - start_x);
    if snap_off {
        return raw;
    }
    let start_time = session.viewport.time_at_x(start_x);
    let proposed = start_time.checked_add(raw).unwrap_or(start_time);
    let mut candidates = snap_candidates(session, proposed);
    if let Some(t) = extra_a {
        candidates.push(t);
    }
    if let Some(t) = extra_b {
        candidates.push(t);
    }
    match snap_time(proposed, &candidates, session.viewport, SNAP_THRESHOLD_PX) {
        Some((snapped, target)) => {
            session.snap_time = Some(target);
            snapped.checked_sub(start_time).unwrap_or(raw)
        }
        None => raw,
    }
}

fn snap_candidates(session: &StudioSession, raw: Time) -> Vec<Time> {
    let mut out = Vec::new();
    out.push(session.playhead);
    out.push(Time::ZERO);
    out.push(session.timeline_duration());
    if let Ok(clips) = hit_clips(session) {
        for clip in clips {
            out.push(clip.start);
            out.push(clip.end());
        }
    }
    if let Some((num, den)) = session.frame_rate
        && let Some(frame) = nearest_frame(raw, num, den)
    {
        out.push(frame);
    }
    out
}

fn hit_clips_on_track(
    session: &StudioSession,
    track: Option<&str>,
) -> Result<Vec<HitClip>, EngineError> {
    let mut clips = hit_clips(session)?;
    if let Some(track) = track {
        match track {
            "Video" | "video" => clips.retain(|clip| clip.kind == ClipKind::Video),
            "Text" | "text" => {
                clips.retain(|clip| matches!(clip.kind, ClipKind::Title | ClipKind::Callout));
            }
            "Audio" | "audio" => clips.clear(),
            _ => {}
        }
    }
    Ok(clips)
}

fn hit_clips(session: &StudioSession) -> Result<Vec<HitClip>, EngineError> {
    let timeline = Engine::timeline(&session.compilation.project)?;
    let current = session.current.clone();
    let mut clips = Vec::new();
    for clip in &timeline.clips {
        let kind_name = format!("{:?}", clip.kind).to_ascii_lowercase();
        let track_kind = clip_kind_from_track(
            &kind_name,
            match kind_name.as_str() {
                "title" | "callout" => "text",
                "audio" => "audio",
                _ => "video",
            },
        );
        if matches!(track_kind, ClipKind::Other) {
            continue;
        }
        let scene_id = scene_id_for_clip(session, &clip.id).unwrap_or_default();
        let selected = current.as_ref().is_some_and(|id| {
            if id.as_str() == clip.id || id.as_str() == scene_id.as_str() {
                return !matches!(track_kind, ClipKind::Title | ClipKind::Callout)
                    || id.as_str() == clip.id;
            }
            session
                .engine
                .inspect(&session.compilation, id)
                .ok()
                .is_some_and(|proj| {
                    if proj.locus.id.as_str() == clip.id {
                        return true;
                    }
                    if matches!(track_kind, ClipKind::Title | ClipKind::Callout) {
                        return false;
                    }
                    matches!(
                        proj.locus.kind,
                        lattice_engine::LocusKind::Scene | lattice_engine::LocusKind::Source
                    ) && proj.locus.scene_id.as_deref() == Some(scene_id.as_str())
                })
        });
        clips.push(HitClip {
            id: clip.id.clone(),
            kind: track_kind,
            start: clip.span.start,
            duration: clip.span.duration,
            selected,
            scene_id,
        });
    }
    Ok(clips)
}

fn scene_id_for_clip(session: &StudioSession, clip_id: &str) -> Option<String> {
    session.compilation.project.scenes.iter().find_map(|scene| {
        scene
            .placements
            .iter()
            .any(|placement| placement.id == clip_id)
            .then(|| scene.id.clone())
    })
}

fn source_range(session: &StudioSession, clip_id: &str) -> Option<TimeSpan> {
    for scene in &session.compilation.project.scenes {
        let Some(placement) = scene
            .placements
            .iter()
            .find(|placement| placement.id == clip_id)
        else {
            continue;
        };
        let source_id = placement.source_id.as_ref()?;
        let source = scene
            .sources
            .iter()
            .find(|source| source.id == *source_id)?;
        return Some(source.source_range);
    }
    None
}

fn video_scene_clips(session: &StudioSession) -> Result<Vec<HitClip>, EngineError> {
    Ok(hit_clips(session)?
        .into_iter()
        .filter(|clip| clip.kind == ClipKind::Video)
        .collect())
}

fn overlay_scene_offset(session: &StudioSession, clip_id: &str) -> Option<Time> {
    let timeline = Engine::timeline(&session.compilation.project).ok()?;
    let clip = timeline.clips.iter().find(|clip| clip.id == clip_id)?;
    let scene = session.compilation.project.scenes.iter().find(|scene| {
        scene
            .placements
            .iter()
            .any(|placement| placement.id == clip_id)
    })?;
    let placement = scene
        .placements
        .iter()
        .find(|placement| placement.id == clip_id)?;
    clip.span.start.checked_sub(placement.span.start).ok()
}

fn scene_names(session: &StudioSession) -> Vec<String> {
    session
        .compilation
        .project
        .sequences
        .first()
        .map(|sequence| {
            sequence
                .scene_ids
                .iter()
                .filter_map(|id| id.strip_prefix("scene:").map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn point_clip(session: &mut StudioSession, clip_id: &str) {
    session.unresolved = None;
    session.current = Some(LocusId::new(clip_id));
}

fn point_overlay_or_overlap(
    session: &mut StudioSession,
    clip_id: &str,
    x: f64,
) -> Result<(), EngineError> {
    let clips = hit_clips_on_track(session, Some("Text"))?;
    let at = clips_at_x(&clips, x, session.viewport);
    if at.len() <= 1 {
        point_clip(session, clip_id);
        return Ok(());
    }
    let loci = session.loci()?;
    let candidates: Vec<_> = at
        .iter()
        .filter_map(|clip| {
            loci.iter()
                .find(|locus| locus.id.as_str() == clip.id || locus.node_id == clip.id)
                .cloned()
        })
        .collect();
    if candidates.len() <= 1 {
        point_clip(session, clip_id);
        return Ok(());
    }
    session.current = None;
    session.unresolved = Some(UnresolvedPointing {
        projection: Projection::Timeline,
        time: Some(session.viewport.time_at_x(x)),
        candidates,
    });
    session.last_spoken = Some(session.utterance().spoken_text());
    Ok(())
}

fn local_at(timeline_start: Time, scene_offset: Time) -> Time {
    timeline_start
        .checked_sub(scene_offset)
        .unwrap_or(Time::ZERO)
}

fn commit_overlay(
    session: &mut StudioSession,
    kind: ClipKind,
    at: Option<Time>,
    duration: Option<Time>,
) -> Result<GestureOutcome, EngineError> {
    if let Some(duration) = duration
        && duration < Time::ZERO
    {
        session.last_gesture_error = Some("duration must not be negative".into());
        return Ok(GestureOutcome::Failed);
    }
    let edit = match kind {
        ClipKind::Callout => SemanticEdit::Callout { at, duration },
        _ => SemanticEdit::Title {
            text: None,
            at,
            duration,
            opacity: None,
        },
    };
    apply_committed(session, edit).map(|()| GestureOutcome::Applied)
}

/// Ephemeral clip rectangles for layout while a gesture is in flight.
pub fn ephemeral_clip_span(session: &StudioSession, clip_id: &str) -> Option<(Time, Time)> {
    match &session.gesture {
        TimelineGesture::Trim {
            clip_id: id,
            original_in,
            preview_in,
            preview_out,
            timeline_start,
            edge,
            ..
        } if id == clip_id => {
            let duration = preview_out.checked_sub(*preview_in).unwrap_or(Time::ZERO);
            let start = match edge {
                Edge::Left => {
                    let delta = preview_in.checked_sub(*original_in).unwrap_or(Time::ZERO);
                    timeline_start.checked_add(delta).unwrap_or(*timeline_start)
                }
                Edge::Right => *timeline_start,
            };
            Some((start, duration))
        }
        TimelineGesture::MoveOverlay {
            clip_id: id,
            preview,
            moved,
            ..
        } if id == clip_id && *moved => Some((preview.start, preview.duration)),
        TimelineGesture::ResizeOverlay {
            clip_id: id,
            preview,
            ..
        } if id == clip_id => Some((preview.start, preview.duration)),
        _ => None,
    }
}

pub fn insertion_marker(session: &StudioSession) -> Option<Time> {
    let TimelineGesture::Reorder {
        proposed_index,
        moved,
        ..
    } = &session.gesture
    else {
        return None;
    };
    if !moved {
        return None;
    }
    let Ok(clips) = video_scene_clips(session) else {
        return None;
    };
    if clips.is_empty() {
        return None;
    }
    if *proposed_index >= clips.len() {
        return clips.last().map(HitClip::end);
    }
    Some(clips[*proposed_index].start)
}

pub fn overlay_playhead_visible(
    session: &StudioSession,
    clip_id: &str,
    compiled: TimeSpan,
) -> bool {
    match &session.gesture {
        TimelineGesture::MoveOverlay {
            clip_id: id,
            preview,
            moved,
            ..
        } if id == clip_id && *moved => preview.contains(session.playhead),
        TimelineGesture::ResizeOverlay {
            clip_id: id,
            preview,
            ..
        } if id == clip_id => preview.contains(session.playhead),
        _ => compiled.contains(session.playhead),
    }
}

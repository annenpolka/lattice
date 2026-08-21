//! Observable Studio semantic state for agent smoke.
//!
//! This is a log/test hook over existing session fields. It is not a second
//! selection model and not a permanent on-canvas debug HUD.

use serde_json::{Value, json};

use crate::gesture::TimelineGesture;
use crate::session::StudioSession;

/// Snapshot of the shared locus, playhead, and in-flight interaction.
#[must_use]
pub fn snapshot(session: &StudioSession) -> Value {
    let locus = match session.current_locus() {
        Ok(Some(locus)) => json!({
            "id": locus.id.as_str(),
            "kind": locus.kind,
            "label": locus.label,
        }),
        Ok(None) => Value::Null,
        Err(err) => json!({ "error": err.to_string() }),
    };
    json!({
        "locus": locus,
        "playhead": session.playhead().to_string(),
        "playing": session.is_playing(),
        "interaction": interaction_mode(session),
        "gesture": gesture_value(session.gesture()),
        "drag": drag_value(session),
        "last_gesture_error": session.last_gesture_error(),
    })
}

fn interaction_mode(session: &StudioSession) -> &'static str {
    if session.is_playing() {
        return "play";
    }
    if session.canvas_overlay_resize_active() {
        return "canvas-resize";
    }
    if session.canvas_overlay_drag_active() {
        return "canvas-drag";
    }
    match session.gesture() {
        TimelineGesture::None => "idle",
        TimelineGesture::Scrub { .. } => "scrub",
        TimelineGesture::Trim { .. } => "trim",
        TimelineGesture::Reorder { .. } => "reorder",
        TimelineGesture::MoveOverlay { .. } => "move-overlay",
        TimelineGesture::ResizeOverlay { .. } => "resize-overlay",
    }
}

fn gesture_value(gesture: &TimelineGesture) -> Value {
    match gesture {
        TimelineGesture::None => json!({ "kind": "none" }),
        TimelineGesture::Scrub { start_playhead, .. } => {
            json!({ "kind": "scrub", "start_playhead": start_playhead.to_string() })
        }
        TimelineGesture::Trim {
            clip_id,
            edge,
            preview_in,
            preview_out,
            ..
        } => json!({
            "kind": "trim",
            "clip_id": clip_id,
            "edge": format!("{edge:?}").to_ascii_lowercase(),
            "preview_in": preview_in.to_string(),
            "preview_out": preview_out.to_string(),
        }),
        TimelineGesture::Reorder {
            clip_id,
            scene_id,
            original_index,
            proposed_index,
            moved,
            ..
        } => json!({
            "kind": "reorder",
            "source": clip_id,
            "target": scene_id,
            "original_index": original_index,
            "proposed_index": proposed_index,
            "moved": moved,
            "valid": proposed_index != original_index || !*moved,
        }),
        TimelineGesture::MoveOverlay {
            clip_id,
            preview,
            moved,
            ..
        } => json!({
            "kind": "move-overlay",
            "source": clip_id,
            "preview_start": preview.start.to_string(),
            "preview_end": preview.end().to_string(),
            "moved": moved,
            "valid": preview.duration >= crate::gesture::min_duration(),
        }),
        TimelineGesture::ResizeOverlay {
            clip_id,
            edge,
            preview,
            ..
        } => json!({
            "kind": "resize-overlay",
            "source": clip_id,
            "edge": format!("{edge:?}").to_ascii_lowercase(),
            "preview_start": preview.start.to_string(),
            "preview_end": preview.end().to_string(),
            "valid": preview.duration >= crate::gesture::min_duration(),
        }),
    }
}

fn drag_value(session: &StudioSession) -> Value {
    if let Some(drag) = session.canvas_drag() {
        let position = drag.preview_position();
        return json!({
            "kind": "canvas-move",
            "source": drag.locus_id().as_str(),
            "target": { "x": position.x, "y": position.y },
            "valid": session.last_gesture_error().is_none(),
        });
    }
    if let Some(resize) = session.canvas_resize() {
        let preview = resize.preview();
        return json!({
            "kind": "canvas-resize",
            "source": resize.locus_id().as_str(),
            "target": {
                "x": preview.position.x,
                "y": preview.position.y,
                "scale": preview.scale.milli,
            },
            "valid": session.last_gesture_error().is_none(),
        });
    }
    match session.gesture() {
        TimelineGesture::None | TimelineGesture::Scrub { .. } => Value::Null,
        TimelineGesture::Trim {
            clip_id,
            preview_in,
            preview_out,
            ..
        } => json!({
            "kind": "trim",
            "source": clip_id,
            "target": {
                "in": preview_in.to_string(),
                "out": preview_out.to_string(),
            },
            "valid": (*preview_out - *preview_in) >= crate::gesture::min_duration()
                && session.last_gesture_error().is_none(),
        }),
        TimelineGesture::Reorder {
            clip_id,
            proposed_index,
            original_index,
            moved,
            ..
        } => json!({
            "kind": "reorder",
            "source": clip_id,
            "target": proposed_index,
            "valid": (*proposed_index != *original_index || !*moved)
                && session.last_gesture_error().is_none(),
        }),
        TimelineGesture::MoveOverlay {
            clip_id, preview, ..
        }
        | TimelineGesture::ResizeOverlay {
            clip_id, preview, ..
        } => json!({
            "kind": "overlay",
            "source": clip_id,
            "target": {
                "start": preview.start.to_string(),
                "end": preview.end().to_string(),
            },
            "valid": preview.duration >= crate::gesture::min_duration()
                && session.last_gesture_error().is_none(),
        }),
    }
}

/// Persist the latest snapshot when `LATTICE_STUDIO_STATE` is set.
pub fn write_state_file(state: &Value) {
    let Ok(path) = std::env::var("LATTICE_STUDIO_STATE") else {
        return;
    };
    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(path, text);
    }
}

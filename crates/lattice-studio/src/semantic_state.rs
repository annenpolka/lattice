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
    let utterance = session.utterance();
    json!({
        "locus": locus,
        "playhead": session.playhead().to_string(),
        "duration": session.duration().to_string(),
        "playing": session.is_playing(),
        "interaction": interaction_mode(session),
        "gesture": gesture_value(session.gesture()),
        "drag": drag_value(session),
        "last_gesture_error": session.last_gesture_error(),
        "pointing": utterance.pointing,
        "projection": session.touched_projection().as_str(),
        "legal": utterance.legal.iter().map(|edit| {
            json!({
                "verb": edit.verb,
                "target": edit.target.as_str(),
                "scope": edit.scope,
                "effect": edit.effect,
            })
        }).collect::<Vec<_>>(),
        "routed": utterance.routed,
        "spoken": utterance.spoken.iter().map(|clause| {
            json!({
                "verb": clause.verb,
                "status": clause.status,
                "reason": clause.reason,
                "target": clause.target,
                "scope": clause.scope,
                "effect": clause.effect,
                "text": clause.text,
            })
        }).collect::<Vec<_>>(),
        "unresolved": session.unresolved_pointing().map(|point| {
            json!({
                "projection": point.projection.as_str(),
                "candidates": point.candidates.iter().map(|locus| locus.id.as_str()).collect::<Vec<_>>(),
            })
        }),
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
        TimelineGesture::Point { .. } => "point",
        TimelineGesture::Scrub { .. } => "scrub",
        TimelineGesture::Trim { .. } => "trim",
        TimelineGesture::Reorder { .. } => "reorder",
        TimelineGesture::MoveOverlay { .. } => "move-overlay",
        TimelineGesture::ResizeOverlay { .. } => "resize-overlay",
        TimelineGesture::Gain { .. } => "set-gain",
        TimelineGesture::Fade { .. } => "set-fade",
        TimelineGesture::Split { .. } => "split",
        TimelineGesture::Delete { .. } => "delete",
        TimelineGesture::PointSource { .. } => "point-source",
        TimelineGesture::PointScene { .. } => "point-scene",
    }
}

#[allow(clippy::too_many_lines)]
fn gesture_value(gesture: &TimelineGesture) -> Value {
    match gesture {
        TimelineGesture::None => json!({ "kind": "none" }),
        TimelineGesture::Point { start_x } => json!({ "kind": "point", "start_x": start_x }),
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
        TimelineGesture::Gain {
            clip_id,
            preview_db,
            moved,
            ..
        } => json!({
            "kind": "set-gain",
            "clip_id": clip_id,
            "preview_db": preview_db,
            "moved": moved,
        }),
        TimelineGesture::Fade {
            clip_id,
            preview,
            moved,
            ..
        } => json!({
            "kind": "set-fade",
            "clip_id": clip_id,
            "preview": preview.to_string(),
            "moved": moved,
        }),
        TimelineGesture::Split { scene_id, at } => json!({
            "kind": "split",
            "scene_id": scene_id,
            "at": at.to_string(),
        }),
        TimelineGesture::Delete { scene_id } => json!({
            "kind": "delete",
            "scene_id": scene_id,
        }),
        TimelineGesture::PointSource { clip_id } => json!({
            "kind": "point-source",
            "clip_id": clip_id,
        }),
        TimelineGesture::PointScene { scene_id } => json!({
            "kind": "point-scene",
            "scene_id": scene_id,
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
        TimelineGesture::None | TimelineGesture::Point { .. } | TimelineGesture::Scrub { .. } => {
            Value::Null
        }
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
        TimelineGesture::Gain {
            clip_id,
            preview_db,
            ..
        } => json!({
            "kind": "set-gain",
            "source": clip_id,
            "target": { "db": preview_db },
            "valid": session.last_gesture_error().is_none(),
        }),
        TimelineGesture::Fade {
            clip_id, preview, ..
        } => json!({
            "kind": "set-fade",
            "source": clip_id,
            "target": { "fade_in": preview.to_string() },
            "valid": session.last_gesture_error().is_none(),
        }),
        TimelineGesture::Split { scene_id, at } => json!({
            "kind": "split",
            "source": scene_id,
            "target": { "at": at.to_string() },
            "valid": session.last_gesture_error().is_none(),
        }),
        TimelineGesture::Delete { scene_id } => json!({
            "kind": "delete",
            "source": scene_id,
            "valid": session.last_gesture_error().is_none(),
        }),
        TimelineGesture::PointSource { .. } | TimelineGesture::PointScene { .. } => Value::Null,
    }
}

/// Persist the latest snapshot when `LATTICE_STUDIO_STATE` is set.
///
/// A missing env var is a no-op. A set path that cannot be created or written
/// is an error — callers must surface it. Silent `let _ =` I/O is not allowed.
pub fn write_state_file(state: &Value) -> Result<(), String> {
    write_env_json("LATTICE_STUDIO_STATE", state)
}

/// Persist window-local widget bounds when `LATTICE_STUDIO_GEOM` is set.
pub fn write_geom_file(geom: &Value) -> Result<(), String> {
    write_env_json("LATTICE_STUDIO_GEOM", geom)
}

fn write_env_json(var: &str, value: &Value) -> Result<(), String> {
    let Ok(path) = std::env::var(var) else {
        return Ok(());
    };
    if path.trim().is_empty() {
        return Ok(());
    }
    write_json_file(&path, value)
}

/// Write `value` to `path`, creating parents. Failures are returned, not swallowed.
pub fn write_json_file(path: &str, value: &Value) -> Result<(), String> {
    let parent = std::path::Path::new(path).parent();
    if let Some(parent) = parent
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create JSON parent {}: {err}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|err| format!("serialize JSON file {path}: {err}"))?;
    std::fs::write(path, text).map_err(|err| format!("write JSON file {path}: {err}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::write_json_file;
    use serde_json::json;

    #[test]
    fn write_json_file_creates_and_reads_back() {
        let dir = std::env::temp_dir().join(format!(
            "lattice-semantic-state-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("nested").join("state.json");
        write_json_file(path.to_str().unwrap(), &json!({"reason": "open"})).expect("write");
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"reason\": \"open\""));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn write_json_file_reports_unwritable_path() {
        let path = if cfg!(windows) {
            "\\\\?\\CON\\state.json"
        } else {
            "/proc/lattice-studio-no-such/state.json"
        };
        let err = write_json_file(path, &json!({"ok": false})).expect_err("must fail");
        assert!(
            err.contains("write JSON file") || err.contains("create JSON parent"),
            "{err}"
        );
    }
}

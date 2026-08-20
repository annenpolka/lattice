use lattice_engine::LocusKind;
use lattice_studio::{CanvasPoint, CanvasRect, CanvasSize, GestureOutcome, StudioSession};

fn session_with_title() -> StudioSession {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("lattice-studio-canvas-{nonce}"));
    std::fs::create_dir_all(&root).unwrap();
    lattice_media::generate_av_fixture(root.join("capture.mp4"), 2).unwrap();
    let path = root.join("main.vel");
    std::fs::write(
        &path,
        r#"project "canvas"
convention commentary
media game "capture.mp4"
sequence main {
  intro
}
scene intro {
  game[0s..2s] as clip
  title "Move me" {
    at 0s for 2s
  }
}
"#,
    )
    .unwrap();
    StudioSession::open(path).unwrap()
}

#[test]
fn canvas_drag_is_ephemeral_then_commits_one_undoable_source_patch() {
    let mut session = session_with_title();
    let original = session.source().to_string();
    let title = session
        .loci()
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == LocusKind::Title)
        .unwrap();
    session.point_at(title.id.clone());
    let before = session.layout().unwrap().canvas;
    let overlay = before.overlays.iter().find(|item| item.selected).unwrap();
    let rect = CanvasRect::new(
        f64::from(overlay.x),
        f64::from(overlay.y),
        f64::from(overlay.width),
        f64::from(overlay.height),
    );
    let size = CanvasSize::new(
        f64::from(before.preview_width),
        f64::from(before.preview_height),
    );
    let start = CanvasPoint::new(rect.x + 100.0, rect.y + 10.0);
    session
        .begin_canvas_overlay_drag(title.id.as_str(), rect, size, start)
        .unwrap();
    let end = CanvasPoint::new(start.x + 120.0, 50.0);
    session.update_canvas_overlay_drag(end).unwrap();
    assert_eq!(session.source(), original, "update must not rewrite VEL");
    assert_eq!(session.undo_len(), 0);
    let ephemeral = session.layout().unwrap();
    let moved = ephemeral
        .canvas
        .overlays
        .iter()
        .find(|item| item.locus_id == title.id.as_str())
        .unwrap();
    assert!(moved.x > overlay.x, "x drag must move selection chrome");
    assert!(moved.y < overlay.y, "y drag must move selection chrome");

    assert_eq!(
        session.commit_canvas_overlay_drag(end).unwrap(),
        GestureOutcome::Applied
    );
    assert!(session.source().contains("position ("));
    assert_eq!(session.undo_len(), 1);
    let rebound = session.current_locus().unwrap().unwrap();
    assert_eq!(rebound.id, title.id);
    assert!(rebound.visual.and_then(|visual| visual.position).is_some());

    session.undo().unwrap();
    assert_eq!(session.source(), original);
    assert_eq!(session.current_locus().unwrap().unwrap().id, title.id);
}

#[test]
fn cancel_discards_canvas_preview_without_source_or_history() {
    let mut session = session_with_title();
    let title = session
        .loci()
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == LocusKind::Title)
        .unwrap();
    session.point_at(title.id.clone());
    let canvas = session.layout().unwrap().canvas;
    let overlay = canvas.overlays.first().unwrap();
    let rect = CanvasRect::new(
        f64::from(overlay.x),
        f64::from(overlay.y),
        f64::from(overlay.width),
        f64::from(overlay.height),
    );
    let size = CanvasSize::new(
        f64::from(canvas.preview_width),
        f64::from(canvas.preview_height),
    );
    let start = CanvasPoint::new(rect.x + 10.0, rect.y + 10.0);
    let original = session.source().to_string();
    session
        .begin_canvas_overlay_drag(title.id.as_str(), rect, size, start)
        .unwrap();
    session
        .update_canvas_overlay_drag(CanvasPoint::new(start.x + 40.0, start.y - 40.0))
        .unwrap();
    assert_eq!(
        session.cancel_canvas_overlay_drag(),
        GestureOutcome::Cancelled
    );
    assert_eq!(session.source(), original);
    assert_eq!(session.undo_len(), 0);
}

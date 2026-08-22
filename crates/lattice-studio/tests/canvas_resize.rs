use lattice_engine::LocusKind;
use lattice_studio::{
    CanvasPoint, CanvasRect, CanvasSize, GestureOutcome, ResizeCorner, StudioSession,
};

fn session_with_overlays() -> StudioSession {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("lattice-studio-resize-{nonce}"));
    std::fs::create_dir_all(&root).unwrap();
    lattice_media::generate_av_fixture(root.join("capture.mp4"), 2).unwrap();
    let path = root.join("main.vel");
    std::fs::write(
        &path,
        r#"project "canvas-resize"
convention commentary
media game "capture.mp4"
sequence main {
  intro
}
scene intro {
  game[0s..2s] as clip
  title "Resize me" {
    at 0s for 2s
  }
  callout "Also me" {
    at 0s for 2s
  }
}
"#,
    )
    .unwrap();
    StudioSession::open(path).unwrap()
}

fn geometry(session: &StudioSession, locus_id: &str) -> (CanvasRect, CanvasSize) {
    let canvas = session.layout().unwrap().canvas;
    let overlay = canvas
        .overlays
        .iter()
        .find(|overlay| overlay.locus_id == locus_id)
        .unwrap();
    (
        CanvasRect::new(
            f64::from(overlay.x),
            f64::from(overlay.y),
            f64::from(overlay.width),
            f64::from(overlay.height),
        ),
        CanvasSize::new(
            f64::from(canvas.preview_width),
            f64::from(canvas.preview_height),
        ),
    )
}

#[test]
fn corner_resize_previews_then_commits_one_undoable_source_patch() {
    let mut session = session_with_overlays();
    let original = session.source().to_string();
    let title = session
        .loci()
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == LocusKind::Title)
        .unwrap();
    session.point_at(title.id.clone());
    let (rect, canvas) = geometry(&session, title.id.as_str());
    let start = CanvasPoint::new(rect.x + rect.width, rect.y + rect.height);
    session
        .begin_canvas_overlay_resize(
            title.id.as_str(),
            ResizeCorner::BottomRight,
            rect,
            canvas,
            start,
        )
        .unwrap();
    let end = CanvasPoint::new(rect.x + rect.width * 0.75, rect.y + rect.height * 0.75);
    let preview = session.update_canvas_overlay_resize(end).unwrap();
    assert_eq!(session.source(), original);
    assert_eq!(session.undo_len(), 0);
    assert_eq!(preview.scale.milli, 750);
    assert!((preview.rect.x - rect.x).abs() < 0.001);
    assert!((preview.rect.y - rect.y).abs() < 0.001);
    let ephemeral = geometry(&session, title.id.as_str()).0;
    assert!(ephemeral.width < rect.width);
    assert!(ephemeral.height < rect.height);

    assert_eq!(
        session.commit_canvas_overlay_resize(end).unwrap(),
        GestureOutcome::Applied
    );
    assert_eq!(session.undo_len(), 1);
    assert!(session.source().contains("position ("));
    assert!(session.source().contains("scale 75%"));
    let rebound = session.current_locus().unwrap().unwrap();
    assert_eq!(rebound.id, title.id);
    assert_eq!(
        rebound.visual.and_then(|visual| visual.scale),
        lattice_engine::NormalizedScale::new(750)
    );

    session.undo().unwrap();
    assert_eq!(session.source(), original);
    assert_eq!(session.current_locus().unwrap().unwrap().id, title.id);
}

#[test]
fn resize_cancel_and_callout_share_the_same_locus_model() {
    let mut session = session_with_overlays();
    let original = session.source().to_string();
    let callout = session
        .loci()
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == LocusKind::Callout)
        .unwrap();
    session.point_at(callout.id.clone());
    let (rect, canvas) = geometry(&session, callout.id.as_str());
    let start = CanvasPoint::new(rect.x, rect.y);
    session
        .begin_canvas_overlay_resize(
            callout.id.as_str(),
            ResizeCorner::TopLeft,
            rect,
            canvas,
            start,
        )
        .unwrap();
    session
        .update_canvas_overlay_resize(CanvasPoint::new(start.x + 30.0, start.y + 5.0))
        .unwrap();
    assert_eq!(
        session.cancel_canvas_overlay_resize(),
        GestureOutcome::Cancelled
    );
    assert_eq!(session.source(), original);
    assert_eq!(session.undo_len(), 0);
    assert_eq!(session.current_locus().unwrap().unwrap().id, callout.id);
}

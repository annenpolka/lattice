//! Gesture lifecycle against the shipped session API.

use lattice_engine::Time;
use lattice_studio::{GestureOutcome, StudioSession, TimelineGesture};

fn open_imported() -> StudioSession {
    let dir = std::env::temp_dir().join(format!(
        "lattice-studio-gesture-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let media = dir.join("gameplay.mp4");
    lattice_media::generate_av_fixture(&media, 8).unwrap();
    StudioSession::open_video(&media).expect("open video")
}

#[test]
fn begin_update_commit_applies_one_rewrite_and_one_undo() {
    let mut session = open_imported();
    let original = session.source().to_string();
    let undo0 = session.undo_len();
    let width = session.viewport().width_pixels();
    let duration = session.layout().unwrap().timeline.duration;
    assert!(duration > Time::ZERO);

    let left = session.x_at_time(Time::ZERO);
    session
        .begin_timeline_pointer(left + 2.0, false)
        .expect("begin");
    assert!(
        matches!(
            session.gesture(),
            TimelineGesture::Trim { .. }
                | TimelineGesture::Scrub { .. }
                | TimelineGesture::Reorder { .. }
        ),
        "begin must enter a gesture: {:?}",
        session.gesture()
    );

    let later = left + width * 0.25;
    session.update_timeline_pointer(later, false).expect("u1");
    session
        .update_timeline_pointer(later + 12.0, false)
        .expect("u2");
    session
        .update_timeline_pointer(later + 24.0, false)
        .expect("u3");
    assert_eq!(session.source(), original, "updates must not rewrite VEL");
    assert_eq!(session.undo_len(), undo0, "updates must not push Undo");

    let outcome = session
        .commit_timeline_pointer(later + 24.0)
        .expect("commit");
    assert!(
        matches!(
            outcome,
            GestureOutcome::Applied | GestureOutcome::Scrubbed | GestureOutcome::Clicked
        ),
        "{outcome:?}"
    );
    if outcome == GestureOutcome::Applied {
        assert_ne!(session.source(), original);
        assert_eq!(session.undo_len(), undo0 + 1);
    } else {
        assert_eq!(session.source(), original);
        assert_eq!(session.undo_len(), undo0);
    }
    assert!(session.gesture().is_none());
}

#[test]
fn begin_update_cancel_mutates_nothing() {
    let mut session = open_imported();
    let original = session.source().to_string();
    let undo0 = session.undo_len();
    let width = session.viewport().width_pixels();

    session.begin_timeline_pointer(4.0, false).expect("begin");
    session
        .update_timeline_pointer(width * 0.3, false)
        .expect("update");
    session
        .update_timeline_pointer(width * 0.4, false)
        .expect("update2");
    assert_eq!(session.source(), original);
    assert_eq!(session.undo_len(), undo0);

    let outcome = session.cancel_timeline_pointer();
    assert_eq!(outcome, GestureOutcome::Cancelled);
    assert_eq!(session.source(), original);
    assert_eq!(session.undo_len(), undo0);
    assert!(session.gesture().is_none());
    let layout = session.layout().unwrap();
    assert!(
        layout.timeline.snap_indicator.is_none(),
        "cancel clears snap"
    );
}

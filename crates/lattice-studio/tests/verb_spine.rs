//! INTEGRATED verb-license spine and the two pointing locks.

use lattice_engine::{LocusKind, Time};
use lattice_studio::{Projection, StudioSession};

fn unique_dir(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lattice-verb-{tag}-{nanos}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn overlap_session() -> StudioSession {
    let dir = unique_dir("overlap");
    lattice_media::generate_av_fixture(dir.join("capture.mp4"), 8).unwrap();
    let vel = dir.join("main.vel");
    std::fs::write(
        &vel,
        r#"project "demo"
convention commentary
media game "capture.mp4"
sequence main {
  demo
}
scene demo {
  game[0s..6s] as fight
  title "Hello" {
    at 2s for 3s
  }
}
"#,
    )
    .unwrap();
    StudioSession::open(&vel).expect("open")
}

fn video_clip_id(session: &StudioSession) -> String {
    session
        .layout()
        .unwrap()
        .timeline
        .tracks
        .iter()
        .find(|track| track.name == "Video")
        .expect("video")
        .clips
        .first()
        .expect("clip")
        .id
        .clone()
}

#[test]
fn video_clip_click_points_source_not_scene() {
    let mut session = overlap_session();
    let clip_id = video_clip_id(&session);
    let before = session
        .current_locus()
        .unwrap()
        .map(|locus| locus.id.clone());
    let pointed = session
        .point_video_clip(&clip_id)
        .unwrap()
        .expect("source clip");
    assert_eq!(
        pointed.kind,
        LocusKind::Source,
        "video click keeps clip identity"
    );
    assert_ne!(pointed.kind, LocusKind::Scene);
    assert!(
        pointed.id.as_str().contains("fight") || pointed.label == "fight",
        "{}",
        pointed.id.as_str()
    );
    let layout = session.layout().unwrap();
    assert!(
        layout.timeline.candidates.is_empty(),
        "identity-bearing click is not overlap"
    );
    assert_eq!(
        session.current_locus().unwrap().unwrap().id,
        pointed.id,
        "one shared LocusId"
    );
    let _ = before;
}

#[test]
fn overlap_failed_point_uses_timeline_hit_path() {
    let mut session = overlap_session();
    let at = Time::from_decimal_seconds(2, 4, 1).unwrap();
    let x = session.x_at_time(at);
    session.begin_timeline_pointer_on(x, true, "Audio").unwrap();
    assert!(
        matches!(
            session.gesture(),
            lattice_studio::TimelineGesture::Point { .. }
        ),
        "empty-rail click begins a coordinate point: {:?}",
        session.gesture()
    );
    session.commit_timeline_pointer(x).unwrap();
    let unresolved = session
        .unresolved_pointing()
        .expect("production hit path must open unresolved pointing");
    assert_eq!(unresolved.projection, Projection::Timeline);
    let kinds: Vec<_> = unresolved
        .candidates
        .iter()
        .map(|locus| locus.kind)
        .collect();
    assert!(kinds.contains(&LocusKind::Title), "{kinds:?}");
    assert!(kinds.contains(&LocusKind::Source), "{kinds:?}");
    assert!(kinds.contains(&LocusKind::Scene), "{kinds:?}");
    assert!(session.current_locus().unwrap().is_none());
}

#[test]
fn overlap_candidates_appear_on_timeline_not_modal() {
    let mut session = overlap_session();
    let here_before = session.current_locus().unwrap().map(|locus| locus.id);
    let pointed = session
        .point_from_timeline_time(Time::from_decimal_seconds(2, 4, 1).unwrap())
        .unwrap();
    assert!(
        pointed.is_none(),
        "a time that names title + source + scene must not collapse"
    );
    let unresolved = session
        .unresolved_pointing()
        .expect("unresolved pointing is first-class");
    assert_eq!(unresolved.projection, Projection::Timeline);
    let kinds: Vec<_> = unresolved
        .candidates
        .iter()
        .map(|locus| locus.kind)
        .collect();
    assert!(kinds.contains(&LocusKind::Title), "{kinds:?}");
    assert!(kinds.contains(&LocusKind::Source), "{kinds:?}");
    assert!(kinds.contains(&LocusKind::Scene), "{kinds:?}");
    let layout = session.layout().unwrap();
    assert!(
        layout.timeline.candidates.len() >= 3,
        "candidates live on the Timeline projection: {:?}",
        layout.timeline.candidates
    );
    assert_eq!(layout.inspector.heading, "unresolved pointing");
    assert!(
        layout
            .inspector
            .utterance
            .spoken
            .iter()
            .any(|line| line.contains("Timeline") && line.contains("loci")),
        "{:?}",
        layout.inspector.utterance.spoken
    );
    assert!(session.current_locus().unwrap().is_none());
    let title = unresolved
        .candidates
        .iter()
        .find(|locus| locus.kind == LocusKind::Title)
        .unwrap()
        .id
        .clone();
    let picked = session
        .pick_point_candidate(title.clone())
        .unwrap()
        .unwrap();
    assert_eq!(picked.id, title);
    assert!(session.unresolved_pointing().is_none());
    assert_eq!(
        session.current_locus().unwrap().unwrap().id,
        title,
        "pick commits one shared LocusId"
    );
    let _ = here_before;
}

#[test]
fn scrub_and_playhead_do_not_change_locus() {
    let mut session = overlap_session();
    let title = session.point_at_title().unwrap().expect("title");
    let here = title.id.clone();
    session.seek(Time::seconds(1));
    assert_eq!(session.current_locus().unwrap().unwrap().id, here);
    session.scrub(Time::seconds(4));
    assert_eq!(session.current_locus().unwrap().unwrap().id, here);
    session.begin_timeline_scrub(40.0, true);
    session.update_timeline_pointer(80.0, true).unwrap();
    session.commit_timeline_pointer(80.0).unwrap();
    assert_eq!(
        session.current_locus().unwrap().unwrap().id,
        here,
        "scrub commit must not call point_from_timeline_time"
    );
    session.click_timeline(Time::seconds(5)).unwrap();
    assert_eq!(
        session.current_locus().unwrap().unwrap().id,
        here,
        "playhead move is not here"
    );
    assert_eq!(session.playhead(), Time::seconds(5));
}

#[test]
fn inspector_hides_title_fields_when_here_is_scene() {
    let mut session = overlap_session();
    let scene = session
        .loci()
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == LocusKind::Scene)
        .unwrap();
    session.point_at(scene.id);
    let layout = session.layout().unwrap();
    assert!(!layout.inspector.title_fields);
    assert!(!layout.inspector.heading.contains("title"));
    assert!(layout.inspector.heading.contains("scene"));
    assert!(
        layout
            .tree
            .iter()
            .flat_map(|node| node.children.iter())
            .flat_map(|node| node.children.iter())
            .all(|node| node.kind != "freeze"),
        "freeze is not a selectable tree row"
    );
}

#[test]
fn toolbar_does_not_silently_retarget_when_routing_differs() {
    let mut session = overlap_session();
    let source = session
        .loci()
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == LocusKind::Source)
        .unwrap();
    session.point_at(source.id.clone());
    let here = source.id.clone();
    let original = session.source().to_string();
    let err = session
        .split_at_playhead()
        .expect_err("no silent scene retarget");
    let spoken = session
        .last_spoken()
        .unwrap_or(&err.to_string())
        .to_string();
    assert!(
        spoken.contains("needs-scene") || spoken.contains("scene:demo"),
        "{spoken}"
    );
    assert_eq!(session.current_locus().unwrap().unwrap().id, here);
    assert_eq!(session.source(), original);

    let scene = session
        .loci()
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == LocusKind::Scene)
        .unwrap();
    session.point_at(scene.id);
    let err = session.set_gain(-3).expect_err("no silent source retarget");
    let spoken = session
        .last_spoken()
        .unwrap_or(&err.to_string())
        .to_string();
    assert!(
        spoken.contains("needs-source-binding") || spoken.contains("source"),
        "{spoken}"
    );
    assert_eq!(
        session.current_locus().unwrap().unwrap().kind,
        LocusKind::Scene,
        "gain must not retarget here to a source"
    );

    session.point_at_title().unwrap();
    session.touch_projection(Projection::Toolbar);
    let utterance = session.utterance();
    assert!(
        utterance.speaks_gap(),
        "legal set vs toolbar commit must be spoken: {}",
        utterance.spoken_text()
    );
    assert!(
        utterance
            .spoken
            .iter()
            .any(|clause| clause.verb == "set-position" && clause.status == "routed"),
        "{}",
        utterance.spoken_text()
    );
}

#[test]
fn timeline_title_utterance_speaks_canvas_route() {
    let mut session = overlap_session();
    session.point_at_title().unwrap();
    session.touch_projection(Projection::Timeline);
    let utterance = session.utterance();
    assert!(
        utterance
            .legal
            .iter()
            .any(|edit| edit.verb == "set-position")
    );
    assert!(!utterance.routed.iter().any(|verb| verb == "set-position"));
    assert!(
        utterance
            .spoken
            .iter()
            .any(|clause| clause.verb == "set-position" && clause.status == "routed")
    );
    assert!(utterance.spoken_text().contains("committed on Canvas"));
}

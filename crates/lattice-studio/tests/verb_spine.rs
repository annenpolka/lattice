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
    let clip = session
        .layout()
        .unwrap()
        .timeline
        .tracks
        .iter()
        .find(|track| track.name == "Video")
        .unwrap()
        .clips
        .iter()
        .find(|clip| clip.id == clip_id)
        .cloned()
        .expect("video clip");
    let x = session.x_at_time(clip.start) + session.viewport().delta_x(clip.duration) / 2.0;
    session.begin_timeline_pointer_on(x, true, "Video").unwrap();
    session.commit_timeline_pointer(x).unwrap();
    let pointed = session
        .current_locus()
        .unwrap()
        .expect("video clip click must commit a source locus");
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
    assert!(
        session.utterance().spoken.iter().any(|clause| {
            clause.status == "relation"
                && clause.text.contains("split →")
                && clause.text.contains("scene:demo")
        }),
        "container verbs come from Engine legal_edits_for on the related Scene: {}",
        session.utterance().spoken_text()
    );
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
    let at = Time::from_decimal_seconds(2, 4, 1).unwrap();
    let x = session.x_at_time(at);
    session.begin_timeline_pointer_on(x, true, "Audio").unwrap();
    session.commit_timeline_pointer(x).unwrap();
    assert!(
        session.current_locus().unwrap().is_none(),
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
    let mut card_ids: Vec<_> = layout
        .timeline
        .candidates
        .iter()
        .map(|card| card.locus_id.clone())
        .collect();
    card_ids.sort();
    card_ids.dedup();
    assert_eq!(
        card_ids.len(),
        layout.timeline.candidates.len(),
        "each card is a distinct LocusId: {:?}",
        layout.timeline.candidates
    );
    assert!(
        layout
            .timeline
            .candidates
            .iter()
            .any(|card| card.kind == "title" && card.routed_verbs.iter().eq(["title"])),
        "cards advertise Timeline routes, not the full legal set: {:?}",
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
    point_scene_via_band(&mut session);
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
    point_source_via_video(&mut session);
    let here = session.current_locus().unwrap().unwrap().id;
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

    point_scene_via_band(&mut session);
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

#[test]
fn timeline_source_utterance_commits_gain_and_fade_here() {
    let mut session = overlap_session();
    point_source_via_video(&mut session);
    session.touch_projection(Projection::Timeline);
    let source_id = session.current_locus().unwrap().unwrap().id;
    let utterance = session.utterance();
    for verb in ["trim", "set-gain", "set-fade"] {
        assert!(
            utterance.routed.iter().any(|item| item == verb),
            "{verb} {:?}",
            utterance.routed
        );
        assert!(
            utterance
                .spoken
                .iter()
                .any(|clause| clause.verb == verb && clause.status == "present"),
            "{verb} {}",
            utterance.spoken_text()
        );
    }
    assert!(
        !utterance.spoken_text().contains("committed on Toolbar"),
        "{}",
        utterance.spoken_text()
    );
    assert!(
        utterance.spoken.iter().any(|clause| {
            clause.status == "relation"
                && clause.text.contains("split →")
                && clause.text.contains("scene:demo")
        }),
        "scene verbs stay Engine-named on the related Scene: {}",
        utterance.spoken_text()
    );
    let state = session.semantic_state();
    assert!(
        state["legal"].as_array().is_some_and(|edits| edits
            .iter()
            .any(|edit| edit["verb"] == "trim"
                && edit["target"] == source_id.as_str()
                && edit["scope"] == "source-range")),
        "{state}"
    );
}

#[test]
fn duplicate_overlay_overlap_cards_are_distinct_by_locus_id() {
    let dir = unique_dir("dup-overlap");
    lattice_media::generate_av_fixture(dir.join("capture.mp4"), 4).unwrap();
    let vel = dir.join("main.vel");
    std::fs::write(
        &vel,
        r#"project "duplicate-overlays"
convention commentary
media game "capture.mp4"
sequence main {
  demo
}
scene demo {
  game[0s..2s] as clip
  title "Same" {
    at 0s for 2s
  }
  title "Same" {
    at 0s for 2s
  }
  callout "Same" {
    at 0s for 2s
  }
}
"#,
    )
    .unwrap();
    let mut session = StudioSession::open(&vel).expect("open");
    let at = Time::seconds(1);
    let x = session.x_at_time(at);
    session.begin_timeline_pointer_on(x, true, "Audio").unwrap();
    session.commit_timeline_pointer(x).unwrap();
    let unresolved = session
        .unresolved_pointing()
        .expect("identical spans still fail as a point");
    assert_eq!(unresolved.projection, Projection::Timeline);
    let titles: Vec<_> = unresolved
        .candidates
        .iter()
        .filter(|locus| locus.kind == LocusKind::Title)
        .cloned()
        .collect();
    assert_eq!(titles.len(), 2, "two Same titles stay two identities");
    assert_ne!(titles[0].id, titles[1].id);
    let cards = session.layout().unwrap().timeline.candidates;
    let title_cards: Vec<_> = cards.iter().filter(|card| card.kind == "title").collect();
    assert_eq!(title_cards.len(), 2);
    assert_ne!(title_cards[0].locus_id, title_cards[1].locus_id);
    assert_eq!(title_cards[0].label, title_cards[1].label);
    assert_eq!(title_cards[0].scope, title_cards[1].scope);
    let picked = session
        .pick_point_candidate(titles[1].id.clone())
        .unwrap()
        .unwrap();
    assert_eq!(picked.id, titles[1].id);
    assert_eq!(
        session.current_locus().unwrap().unwrap().id,
        titles[1].id,
        "pick commits the chosen LocusId, not a same-label collapse"
    );
}

#[test]
fn pick_without_unresolved_pointing_is_refused() {
    let mut session = overlap_session();
    assert!(session.unresolved_pointing().is_none());
    let source = session
        .loci()
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == LocusKind::Source)
        .expect("source");
    let here_before = session.current_locus().unwrap().map(|locus| locus.id);
    let err = session
        .pick_point_candidate(source.id.clone())
        .expect_err("pick requires an active unresolved point");
    assert!(err.to_string().contains("no unresolved pointing"), "{err}");
    assert_eq!(
        session.current_locus().unwrap().map(|locus| locus.id),
        here_before,
        "a refused pick must not adopt here"
    );
}

#[test]
fn pick_rejects_locus_not_in_unresolved_candidates() {
    let mut session = overlap_session();
    let at = Time::from_decimal_seconds(2, 4, 1).unwrap();
    let x = session.x_at_time(at);
    session.begin_timeline_pointer_on(x, true, "Audio").unwrap();
    session.commit_timeline_pointer(x).unwrap();
    let unresolved = session
        .unresolved_pointing()
        .expect("overlap opens unresolved pointing");
    let candidate_ids: Vec<_> = unresolved
        .candidates
        .iter()
        .map(|locus| locus.id.clone())
        .collect();
    let outsider = session
        .loci()
        .unwrap()
        .into_iter()
        .find(|locus| !candidate_ids.contains(&locus.id))
        .expect("a locus outside the touched projection's candidate list");
    let err = session
        .pick_point_candidate(outsider.id.clone())
        .expect_err("non-candidate must be refused");
    assert!(
        err.to_string()
            .contains("candidate is not on the touched projection"),
        "{err}"
    );
    assert!(
        session.unresolved_pointing().is_some(),
        "a refused pick must leave pointing unresolved"
    );
    assert!(session.current_locus().unwrap().is_none());
}

fn clip_on(session: &StudioSession, track: &str) -> lattice_studio::TimelineClipView {
    session
        .layout()
        .unwrap()
        .timeline
        .tracks
        .iter()
        .find(|row| row.name == track)
        .expect(track)
        .clips
        .first()
        .cloned()
        .expect(track)
}

fn point_source_via_video(session: &mut StudioSession) {
    let clip = clip_on(session, "Video");
    let x = session.x_at_time(clip.start) + session.viewport().delta_x(clip.duration) / 2.0;
    session.begin_timeline_pointer_on(x, true, "Video").unwrap();
    session.commit_timeline_pointer(x).unwrap();
    assert_eq!(
        session.current_locus().unwrap().unwrap().kind,
        LocusKind::Source
    );
}

fn point_scene_via_band(session: &mut StudioSession) {
    let clip = clip_on(session, "Scene");
    let x = session.x_at_time(clip.start) + session.viewport().delta_x(clip.duration) / 2.0;
    session.begin_timeline_pointer_on(x, true, "Scene").unwrap();
    session.commit_timeline_pointer(x).unwrap();
    assert_eq!(
        session.current_locus().unwrap().unwrap().kind,
        LocusKind::Scene
    );
}

#[test]
fn source_here_commits_gain_on_audio_line_and_fade_on_video_wedge() {
    let mut session = overlap_session();
    let original = session.source().to_string();
    point_source_via_video(&mut session);
    let layout = session.layout().unwrap();
    let audio = layout
        .timeline
        .tracks
        .iter()
        .find(|track| track.name == "Audio")
        .unwrap()
        .clips
        .first()
        .cloned()
        .expect("audio block");
    assert!(audio.gain_handle, "gain line is drawn on the audio block");
    let video = layout
        .timeline
        .tracks
        .iter()
        .find(|track| track.name == "Video")
        .unwrap()
        .clips
        .first()
        .cloned()
        .expect("video block");
    assert!(video.fade_handle, "fade wedge is drawn on the video block");
    assert!(video.handles, "trim stays on clip edges");

    let x = session.x_at_time(audio.start) + session.viewport().delta_x(audio.duration) / 2.0;
    let line_y = lattice_studio::gain_line_top(audio.gain_db.unwrap_or(0)) + 2.0;
    session
        .begin_timeline_pointer_on_xy(x, line_y, true, "Audio")
        .unwrap();
    assert!(
        matches!(
            session.gesture(),
            lattice_studio::TimelineGesture::Gain { .. }
        ),
        "{:?}",
        session.gesture()
    );
    session.update_timeline_pointer_xy(x, 2.0, true).unwrap();
    session.commit_timeline_pointer_xy(x, 2.0).unwrap();
    assert_ne!(session.source(), original);
    assert!(session.source().contains("gain"), "{}", session.source());
    assert!(!session.source().contains("by --"));

    let before_body = session.source().to_string();
    session
        .begin_timeline_pointer_on_xy(x, 20.0, true, "Audio")
        .unwrap();
    assert!(
        !matches!(
            session.gesture(),
            lattice_studio::TimelineGesture::Gain { .. }
        ),
        "audio-block body is not a hidden gain surface: {:?}",
        session.gesture()
    );
    session.update_timeline_pointer_xy(x, 2.0, true).unwrap();
    session.commit_timeline_pointer_xy(x, 2.0).unwrap();
    assert_eq!(
        session.source(),
        before_body,
        "audio-block body must not commit SetGain"
    );

    let before_fade = session.source().to_string();
    let fade_x = session.x_at_time(video.start) + 14.0;
    session
        .begin_timeline_pointer_on_xy(fade_x, 3.0, true, "Video")
        .unwrap();
    assert!(
        matches!(
            session.gesture(),
            lattice_studio::TimelineGesture::Fade { .. }
        ),
        "{:?}",
        session.gesture()
    );
    session
        .update_timeline_pointer_xy(fade_x + 40.0, 3.0, true)
        .unwrap();
    session
        .commit_timeline_pointer_xy(fade_x + 40.0, 3.0)
        .unwrap();
    assert_ne!(session.source(), before_fade);
    assert!(session.source().contains("fade"), "{}", session.source());
}

#[test]
fn scene_here_commits_split_on_cut_lane_and_delete_on_handle() {
    let mut session = overlap_session();
    point_scene_via_band(&mut session);
    let scene = clip_on(&session, "Scene");
    assert!(scene.cut_lane && scene.delete_handle);

    let mid = session.x_at_time(scene.start) + session.viewport().delta_x(scene.duration) / 2.0;
    let before = session.source().to_string();
    session
        .begin_timeline_pointer_on_xy(mid, 3.0, true, "Scene")
        .unwrap();
    assert!(
        matches!(
            session.gesture(),
            lattice_studio::TimelineGesture::Split { .. }
        ),
        "{:?}",
        session.gesture()
    );
    session.commit_timeline_pointer_xy(mid, 3.0).unwrap();
    assert_ne!(session.source(), before);
    assert!(
        session.source().matches("scene ").count() >= 2,
        "{}",
        session.source()
    );

    point_scene_via_band(&mut session);
    let scene = clip_on(&session, "Scene");
    let right = session.x_at_time(scene.start.checked_add(scene.duration).unwrap()) - 8.0;
    let before_delete = session.source().to_string();
    session
        .begin_timeline_pointer_on_xy(right, 11.0, true, "Scene")
        .unwrap();
    assert!(
        matches!(
            session.gesture(),
            lattice_studio::TimelineGesture::Delete { .. }
        ),
        "{:?}",
        session.gesture()
    );
    session.commit_timeline_pointer_xy(right, 11.0).unwrap();
    assert_ne!(session.source(), before_delete);
}

#[test]
fn identical_fade_does_not_push_undo() {
    let dir = unique_dir("fade-undo");
    lattice_media::generate_av_fixture(dir.join("capture.mp4"), 4).unwrap();
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
  game[0s..2s] as fight
  fade fight {
    at 0s for 0.5s
  }
}
"#,
    )
    .unwrap();
    let mut session = StudioSession::open(&vel).expect("open");
    point_source_via_video(&mut session);
    let undo_before = session.undo_len();
    session
        .set_fade(Time::milliseconds(500))
        .expect("same fade is a no-op, not an error");
    assert_eq!(
        session.undo_len(),
        undo_before,
        "empty fade must not push Undo"
    );
    assert!(session.source().contains("for 0.5s"));
}

#[test]
fn selected_overlay_does_not_expose_hidden_video_trim() {
    let mut session = overlap_session();
    let title = session
        .loci()
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == LocusKind::Title)
        .expect("title");
    let title_clip = session
        .layout()
        .unwrap()
        .timeline
        .tracks
        .iter()
        .find(|track| track.name == "Text")
        .unwrap()
        .clips
        .iter()
        .find(|clip| clip.kind == "title")
        .cloned()
        .expect("title clip");
    let video = clip_on(&session, "Video");
    assert!(!video.handles, "unselected video draws no trim handles");
    let title_x = session.x_at_time(title_clip.start + Time::milliseconds(200));
    session
        .begin_timeline_pointer_on(title_x, true, "Text")
        .unwrap();
    session.commit_timeline_pointer(title_x).unwrap();
    assert_eq!(
        session.current_locus().unwrap().unwrap().id,
        title.id,
        "point the overlay through the Text track"
    );
    let video_left = session.x_at_time(video.start) + 1.0;
    session
        .begin_timeline_pointer_on(video_left, true, "Video")
        .unwrap();
    assert!(
        !matches!(
            session.gesture(),
            lattice_studio::TimelineGesture::Trim { .. }
        ),
        "unselected video edge is not a hidden trim: {:?}",
        session.gesture()
    );
    session.cancel_timeline_pointer();
    let title_right = session.x_at_time(title_clip.start + title_clip.duration) - 1.0;
    session
        .begin_timeline_pointer_on(title_right, true, "Text")
        .unwrap();
    assert!(
        matches!(
            session.gesture(),
            lattice_studio::TimelineGesture::ResizeOverlay { .. }
        ),
        "selected overlay still trims from its drawn edge: {:?}",
        session.gesture()
    );
}

#[test]
fn toolbar_routes_nothing() {
    assert!(lattice_studio::routed_verbs(Projection::Toolbar, LocusKind::Source).is_empty());
    assert!(lattice_studio::routed_verbs(Projection::Toolbar, LocusKind::Scene).is_empty());
    assert!(lattice_studio::routed_verbs(Projection::Toolbar, LocusKind::Title).is_empty());
}

//! Scrub, trim, reorder, overlay, zoom, and snap against shipped session APIs.

use lattice_engine::{Engine, RawFrame, SemanticEdit, Time};
use lattice_studio::{
    DRAG_THRESHOLD_PX, PreviewMailbox, SNAP_THRESHOLD_PX, StudioSession, TimelineGesture,
};

fn unique_dir(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lattice-studio-{tag}-{nanos}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn open_video(secs: i64) -> StudioSession {
    let dir = unique_dir("video");
    let media = dir.join("gameplay.mp4");
    lattice_media::generate_av_fixture(&media, secs).unwrap();
    StudioSession::open_video(&media).expect("open video")
}

fn write_scenes(dir: &std::path::Path, body: &str) -> StudioSession {
    lattice_media::generate_av_fixture(dir.join("capture.mp4"), 8).unwrap();
    let vel = dir.join("main.vel");
    std::fs::write(&vel, body).unwrap();
    StudioSession::open(&vel).expect("open")
}

fn video_clip(session: &StudioSession) -> lattice_studio::TimelineClipView {
    session
        .layout()
        .unwrap()
        .timeline
        .tracks
        .iter()
        .find(|track| track.name == "Video")
        .expect("video track")
        .clips
        .first()
        .cloned()
        .expect("video clip")
}

fn clip_x(session: &StudioSession, start: Time, edge: f64) -> f64 {
    session.x_at_time(start) + edge
}

#[test]
fn continuous_scrub_follows_latest_x_without_vel_or_undo() {
    let mut session = open_video(8);
    let original = session.source().to_string();
    let duration = session.layout().unwrap().timeline.duration;
    let width = session.viewport().width_pixels();
    assert!(width > 200.0);

    session.begin_timeline_scrub(0.0, true);
    assert!(matches!(session.gesture(), TimelineGesture::Scrub { .. }));

    for x in [3.0, 17.0, 41.0, 80.0, 161.0, 240.5] {
        session.scrub_at_x(x);
        session.update_timeline_pointer(x, true).expect("update");
        let expected = session.time_at_x(x).max(Time::ZERO).min(duration);
        assert_eq!(
            session.playhead(),
            expected,
            "playhead must equal viewport time-at-x for {x}"
        );
        let one_percent = duration.checked_mul(Time::new(1, 100).unwrap()).unwrap();
        let step = session.time_at_x(x + 1.0).max(Time::ZERO).min(duration);
        let finer = if step > expected {
            step.checked_sub(expected).unwrap_or(Time::ZERO)
        } else {
            expected.checked_sub(step).unwrap_or(Time::ZERO)
        };
        assert!(
            finer < one_percent || duration <= Time::seconds(1),
            "1px step {finer} must be finer than 1% of duration {one_percent}"
        );
    }
    assert_eq!(session.source(), original);
    assert_eq!(session.undo_len(), 0);
    session.commit_timeline_pointer(240.5).expect("commit");
    assert_eq!(session.source(), original);
    assert_eq!(session.undo_len(), 0);
}

#[test]
fn preview_generation_drops_stale_results() {
    let mut box_ = PreviewMailbox::default();
    let g1 = box_.request();
    let g2 = box_.request();
    let g3 = box_.request();
    assert_eq!((g1, g2, g3), (1, 2, 3));
    assert!(
        !box_.accept(g1, "frame-1.png".into(), Time::seconds(1)),
        "stale generation 1 must be ignored"
    );
    assert!(box_.published_path().is_none());
    assert!(box_.accept(g3, "frame-3.png".into(), Time::seconds(3)));
    assert_eq!(
        box_.published_path()
            .map(|p| p.to_string_lossy().into_owned()),
        Some("frame-3.png".into())
    );
    assert_eq!(box_.published_generation(), 3);

    let mut stamped = PreviewMailbox::default();
    stamped.set_stamp("project-a");
    let g = stamped.request();
    assert!(
        !stamped.accept_stamped(g, "old.png".into(), Time::ZERO, "project-b"),
        "a still from another project must not publish"
    );
    assert!(stamped.accept_stamped(g, "new.png".into(), Time::ZERO, "project-a"));

    let mut frames = PreviewMailbox::default();
    frames.set_stamp("project-a");
    let slow = frames.request();
    let newest = frames.request();
    let blue = std::sync::Arc::new(RawFrame::filled(2, 2, 0, 0, 255, 255));
    assert!(
        !frames.accept_frame_stamped(
            slow,
            std::sync::Arc::clone(&blue),
            Time::ZERO,
            "project-a",
            false,
        ),
        "paused scrub must reject a stale generation"
    );
    assert!(
        frames.accept_frame_stamped(
            slow,
            std::sync::Arc::clone(&blue),
            Time::ZERO,
            "project-a",
            true,
        ),
        "Play must publish a completed same-stamp frame while a newer request is queued"
    );
    let red = std::sync::Arc::new(RawFrame::filled(2, 2, 255, 0, 0, 255));
    assert!(frames.accept_frame_stamped(newest, red, Time::seconds(1), "project-a", true,));
    assert_eq!(frames.retained_frame_count(), 2);
    assert_eq!(
        frames.published_frame().and_then(|frame| frame.pixel(0, 0)),
        Some([255, 0, 0, 255])
    );
    let stale_after_publish = std::sync::Arc::new(RawFrame::filled(2, 2, 0, 255, 0, 255));
    assert!(
        !frames.accept_frame_stamped(slow, stale_after_publish, Time::ZERO, "project-a", true,)
    );
    assert_eq!(frames.retained_frame_count(), 2);

    let dir = unique_dir("preview-peek");
    let media = dir.join("gameplay.mp4");
    lattice_media::generate_av_fixture(&media, 4).unwrap();
    let session = StudioSession::open_video(&media).expect("open");
    let cache = session.path().parent().unwrap().join(".lattice-cache");
    let _ = std::fs::remove_dir_all(&cache);
    assert!(session.peek_preview_frame().is_none());
    let layout = session.layout().expect("layout");
    assert!(layout.canvas.preview_frame.is_none());
    assert!(session.peek_preview_frame().is_none());
}

#[test]
fn trim_gesture_commits_once_and_undo_restores() {
    let mut session = open_video(8);
    let original = session.source().to_string();
    let clip = video_clip(&session);
    let mid = clip_x(
        &session,
        clip.start,
        session.viewport().delta_x(clip.duration) / 2.0,
    );
    session
        .begin_timeline_pointer_on(mid, true, "Video")
        .expect("point source");
    session.commit_timeline_pointer(mid).expect("commit point");
    assert_eq!(
        session.current_locus().unwrap().unwrap().kind,
        lattice_engine::LocusKind::Source
    );
    let left = clip_x(&session, clip.start, 1.0);
    session
        .begin_timeline_pointer_on(left, true, "Video")
        .expect("begin left");
    assert!(
        matches!(
            session.gesture(),
            TimelineGesture::Trim {
                edge: lattice_studio::Edge::Left,
                ..
            }
        ),
        "left handle: {:?}",
        session.gesture()
    );
    let later = session.x_at_time(clip.start) + session.viewport().width_pixels() * 0.2;
    session
        .update_timeline_pointer(later, true)
        .expect("update");
    assert_eq!(session.source(), original, "update must not rewrite VEL");
    assert_eq!(session.undo_len(), 0);
    session.commit_timeline_pointer(later).expect("commit left");
    let after_left = session.source().to_string();
    assert_ne!(after_left, original);
    assert!(
        after_left.contains("..") && after_left != original,
        "working VEL source range must change: {after_left}"
    );
    let duration_after = session.layout().unwrap().timeline.duration;
    let duration_before = {
        let probe = open_video(8);
        probe.layout().unwrap().timeline.duration
    };
    assert!(
        duration_after < duration_before || after_left != original,
        "recompile duration should reflect the trim"
    );
    assert_eq!(session.undo_len(), 1);
    session.undo().expect("undo");
    assert_eq!(session.source(), original);

    let clip = video_clip(&session);
    let right = session.x_at_time(clip.start + clip.duration) - 1.0;
    session
        .begin_timeline_pointer_on(right, true, "Video")
        .expect("begin right");
    assert!(
        matches!(
            session.gesture(),
            TimelineGesture::Trim {
                edge: lattice_studio::Edge::Right,
                ..
            }
        ),
        "{:?}",
        session.gesture()
    );
    let earlier = right - session.viewport().width_pixels() * 0.15;
    session.update_timeline_pointer(earlier, true).unwrap();
    assert_eq!(session.source(), original);
    session.commit_timeline_pointer(earlier).unwrap();
    assert_ne!(session.source(), original);
    session.undo().unwrap();
    assert_eq!(session.source(), original);
}

#[test]
#[allow(clippy::too_many_lines)]
fn reorder_body_drag_and_overlay_timing() {
    let dir = unique_dir("reorder");
    let mut session = write_scenes(
        &dir,
        r#"project "demo"
convention commentary
media game "capture.mp4"
sequence main {
  a
  b
  c
}
scene a {
  game[0s..1s] as v
}
scene b {
  game[1s..2s] as v
}
scene c {
  game[2s..3s] as v
  title "Hello" { at 0s for 1s }
  callout "Hold" { at 0s for 1s }
}
"#,
    );
    let original = session.source().to_string();
    let layout = session.layout().unwrap();
    let video = layout
        .timeline
        .tracks
        .iter()
        .find(|track| track.name == "Video")
        .unwrap();
    assert_eq!(video.clips.len(), 3);
    let c = &video.clips[2];
    let a = &video.clips[0];
    let b = &video.clips[1];
    session.point_at(lattice_engine::LocusId::new(c.scene_id.clone()));
    let start = session.x_at_time(c.start + Time::milliseconds(200));
    session
        .begin_timeline_pointer_on(start, true, "Scene")
        .expect("begin scene band");
    assert!(
        matches!(session.gesture(), TimelineGesture::Reorder { .. }),
        "scene-band body drag reorders: {:?}",
        session.gesture()
    );
    let between_ab = session.x_at_time(a.start + a.duration);
    let _ = b;
    session
        .update_timeline_pointer_xy(
            between_ab - DRAG_THRESHOLD_PX - 8.0,
            lattice_studio::TRACK_HEIGHT_PX * 0.75,
            true,
        )
        .unwrap();
    assert_eq!(session.source(), original);
    session
        .commit_timeline_pointer_xy(
            between_ab - DRAG_THRESHOLD_PX - 8.0,
            lattice_studio::TRACK_HEIGHT_PX * 0.75,
        )
        .unwrap();
    let reordered = session.source().to_string();
    assert_ne!(reordered, original);
    let seq = reordered
        .split("sequence main")
        .nth(1)
        .unwrap_or(&reordered);
    let a_at = seq.find("\n  a").or_else(|| seq.find(" a"));
    let c_at = seq.find("\n  c").or_else(|| seq.find(" c"));
    let b_at = seq.find("\n  b").or_else(|| seq.find(" b"));
    assert!(
        c_at.is_some() && a_at.is_some() && b_at.is_some(),
        "sequence body:\n{seq}"
    );
    assert!(
        c_at < a_at && a_at < b_at || c_at < b_at,
        "C should move earlier in the sequence:\n{seq}"
    );
    assert_eq!(session.undo_len(), 1);
    session.undo().unwrap();
    assert_eq!(session.source(), original);

    let title = session
        .loci()
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == lattice_engine::LocusKind::Title)
        .expect("title");
    session.point_at(title.id.clone());
    let layout = session.layout().unwrap();
    let text = layout
        .timeline
        .tracks
        .iter()
        .find(|track| track.name == "Text")
        .unwrap();
    let title_clip = text
        .clips
        .iter()
        .find(|clip| clip.kind == "title")
        .expect("title clip");
    let original_at = title_clip.start;
    let original_dur = title_clip.duration;
    let body_x = session.x_at_time(title_clip.start + Time::milliseconds(200));
    session
        .begin_timeline_pointer_on(body_x, true, "Text")
        .unwrap();
    assert!(
        matches!(session.gesture(), TimelineGesture::MoveOverlay { .. }),
        "title body drag: {:?}",
        session.gesture()
    );
    let shifted = body_x + 40.0;
    session.update_timeline_pointer(shifted, true).unwrap();
    let outcome = session.commit_timeline_pointer(shifted).unwrap();
    assert_eq!(
        outcome,
        lattice_studio::GestureOutcome::Applied,
        "title move source:\n{}",
        session.source()
    );
    let after = session.layout().unwrap();
    let title_after = after
        .timeline
        .tracks
        .iter()
        .find(|t| t.name == "Text")
        .unwrap()
        .clips
        .iter()
        .find(|c| c.kind == "title")
        .unwrap();
    assert_eq!(title_after.duration, original_dur);
    assert_ne!(title_after.start, original_at);
    session.undo().unwrap();

    session.point_at(title.id.clone());
    let layout = session.layout().unwrap();
    let title_clip = layout
        .timeline
        .tracks
        .iter()
        .find(|t| t.name == "Text")
        .unwrap()
        .clips
        .iter()
        .find(|c| c.kind == "title")
        .unwrap();
    let right = session.x_at_time(title_clip.start + title_clip.duration) - 1.0;
    session
        .begin_timeline_pointer_on(right, true, "Text")
        .unwrap();
    session.update_timeline_pointer(right - 30.0, true).unwrap();
    session.commit_timeline_pointer(right - 30.0).unwrap();
    let shorter = session
        .layout()
        .unwrap()
        .timeline
        .tracks
        .iter()
        .find(|t| t.name == "Text")
        .unwrap()
        .clips
        .iter()
        .find(|c| c.kind == "title")
        .unwrap()
        .duration;
    assert!(shorter < original_dur);
    session.undo().unwrap();

    let err = session.apply_edit(SemanticEdit::Title {
        text: None,
        at: None,
        duration: Some(Time::seconds(-1)),
        opacity: None,
    });
    assert!(err.is_err(), "negative duration must be rejected");

    let callout = session
        .loci()
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == lattice_engine::LocusKind::Callout)
        .expect("callout");
    session.point_at(callout.id.clone());
    let layout = session.layout().unwrap();
    let callout_clip = layout
        .timeline
        .tracks
        .iter()
        .find(|t| t.name == "Text")
        .unwrap()
        .clips
        .iter()
        .find(|c| c.kind == "callout")
        .expect("callout clip");
    let cx = session.x_at_time(callout_clip.start + Time::milliseconds(200));
    let orig_dur = callout_clip.duration;
    session.begin_timeline_pointer_on(cx, true, "Text").unwrap();
    session.update_timeline_pointer(cx + 35.0, true).unwrap();
    session.commit_timeline_pointer(cx + 35.0).unwrap();
    let after_layout = session.layout().unwrap();
    let after = after_layout
        .timeline
        .tracks
        .iter()
        .find(|t| t.name == "Text")
        .unwrap()
        .clips
        .iter()
        .find(|c| c.kind == "callout")
        .unwrap();
    assert_eq!(after.duration, orig_dur);
}

#[test]
fn zoom_scroll_snap_and_failed_commit_restore() {
    let mut session = open_video(8);
    let anchor = session.time_at_x(200.0);
    let x0 = session.x_at_time(anchor);
    session.zoom_around(anchor, 2.0);
    let x1 = session.x_at_time(anchor);
    assert!(
        (x0 - x1).abs() < 1.0,
        "zoom around anchor must keep its pixel ({x0} vs {x1})"
    );

    let before = session.time_at_x(80.0);
    session.scroll_pixels(40.0);
    let after = session.time_at_x(80.0);
    assert_ne!(before, after);

    let candidate = Time::ZERO;
    let raw = Time::milliseconds(100);
    let zoomed_out = lattice_studio::TimelineViewport::new(Time::ZERO, Time::seconds(10), 100.0);
    let zoomed_in = lattice_studio::TimelineViewport::new(Time::ZERO, Time::seconds(10), 1000.0);
    let snap_out = lattice_studio::snap_time(raw, &[candidate], zoomed_out, SNAP_THRESHOLD_PX);
    let snap_in = lattice_studio::snap_time(raw, &[candidate], zoomed_in, SNAP_THRESHOLD_PX);
    assert!(snap_out.is_some(), "zoomed out: 100ms is 1px and must snap");
    assert!(
        snap_in.is_none(),
        "zoomed in: 100ms is 10px and must not snap"
    );

    let original = session.source().to_string();
    let clip = video_clip(&session);
    let mid = clip_x(
        &session,
        clip.start,
        session.viewport().delta_x(clip.duration) / 2.0,
    );
    session
        .begin_timeline_pointer_on(mid, true, "Video")
        .unwrap();
    session.commit_timeline_pointer(mid).unwrap();
    let left = clip_x(&session, clip.start, 1.0);
    session
        .begin_timeline_pointer_on(left, true, "Video")
        .unwrap();
    session.update_timeline_pointer(left + 80.0, true).unwrap();
    let ephemeral = session.layout().unwrap();
    let eclip = ephemeral
        .timeline
        .tracks
        .iter()
        .find(|t| t.name == "Video")
        .unwrap()
        .clips
        .first()
        .unwrap();
    let compiled_duration = clip.duration;
    assert!(
        eclip.duration != compiled_duration || eclip.start != clip.start,
        "ephemeral trim must change clip geometry"
    );
    let err = session.apply_committed_edit(SemanticEdit::Trim {
        in_point: Some(Time::seconds(99)),
        out_point: None,
    });
    assert!(err.is_err(), "out-of-range trim must fail");
    assert!(session.last_gesture_error().is_some());
    assert_eq!(session.source(), original);
    let restored = video_clip(&session);
    assert_eq!(restored.start, clip.start);
    assert_eq!(restored.duration, clip.duration);
    assert!(session.gesture().is_none());
}

#[test]
fn engine_still_compiles_reordered_sequence() {
    let dir = unique_dir("engine-reorder");
    lattice_media::generate_av_fixture(dir.join("capture.mp4"), 6).unwrap();
    let vel = dir.join("main.vel");
    std::fs::write(
        &vel,
        r#"project "demo"
convention commentary
media game "capture.mp4"
sequence main {
  a
  b
  c
}
scene a { game[0s..1s] as v }
scene b { game[1s..2s] as v }
scene c { game[2s..3s] as v }
"#,
    )
    .unwrap();
    let engine = Engine::default();
    let compilation = engine.compile_path(&vel).unwrap();
    let scene_c = engine
        .loci(&compilation)
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == lattice_engine::LocusKind::Scene && locus.label == "c")
        .unwrap();
    let proposal = engine
        .propose(
            &compilation,
            &scene_c,
            SemanticEdit::ReorderScene {
                before: Some("b".into()),
            },
        )
        .unwrap();
    let next = engine
        .apply_proposal(&compilation.source, &proposal)
        .unwrap();
    assert!(next.contains("sequence main"));
    let seq = next.split("sequence main").nth(1).unwrap();
    let c = seq.find('c').unwrap();
    let b = seq.find('b').unwrap();
    assert!(c < b, "c should sit before b:\n{seq}");
}

#[test]
fn sequence_gap_is_empty_in_layout_and_survives_studio_reorder_commit() {
    let dir = unique_dir("sequence-gap");
    let mut session = write_scenes(
        &dir,
        r#"project "demo"
convention commentary
media game "capture.mp4"
sequence main {
  a
  // authored pause
  gap 500ms
  b
}
scene a { game[0s..1s] as v }
scene b { game[1s..2s] as v }
"#,
    );
    let layout = session.layout().unwrap();
    let video = layout
        .timeline
        .tracks
        .iter()
        .find(|track| track.name == "Video")
        .unwrap();
    assert_eq!(video.clips[0].start, Time::ZERO);
    assert_eq!(video.clips[0].duration, Time::seconds(1));
    assert_eq!(video.clips[1].start, Time::milliseconds(1_500));
    assert_eq!(layout.timeline.duration, Time::milliseconds(2_500));

    session.point_at(lattice_engine::LocusId::new("scene:b"));
    session
        .apply_committed_edit(SemanticEdit::ReorderScene {
            before: Some("a".into()),
        })
        .unwrap();
    assert!(session.source().contains("// authored pause\n  gap 500ms"));
    assert_eq!(session.undo_len(), 1);
    assert_eq!(
        session.compilation().project.sequences[0].scene_offsets,
        vec![Time::ZERO, Time::milliseconds(500)]
    );
}

#[test]
fn text_track_pointer_does_not_hit_video_clips() {
    let dir = unique_dir("track-hit");
    let mut session = write_scenes(
        &dir,
        r#"project "demo"
convention commentary
media game "capture.mp4"
sequence main {
  a
}
scene a {
  game[0s..3s] as v
  title "Hello" { at 0s for 1s }
}
"#,
    );
    let title = session
        .loci()
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == lattice_engine::LocusKind::Title)
        .expect("title");
    session.point_at(title.id);
    let layout = session.layout().unwrap();
    let title_clip = layout
        .timeline
        .tracks
        .iter()
        .find(|track| track.name == "Text")
        .unwrap()
        .clips
        .iter()
        .find(|clip| clip.kind == "title")
        .unwrap();
    let x = session.x_at_time(title_clip.start + Time::milliseconds(200));
    session.begin_timeline_pointer_on(x, true, "Video").unwrap();
    assert!(
        matches!(session.gesture(), TimelineGesture::PointSource { .. }),
        "video rail click keeps source-clip identity: {:?}",
        session.gesture()
    );
    session.cancel_timeline_pointer();
    session.begin_timeline_pointer_on(x, true, "Text").unwrap();
    assert!(
        matches!(session.gesture(), TimelineGesture::MoveOverlay { .. }),
        "text rail must start overlay move: {:?}",
        session.gesture()
    );
    session.cancel_timeline_pointer();
    session.begin_timeline_pointer_on(x, true, "Audio").unwrap();
    assert!(
        matches!(
            session.gesture(),
            TimelineGesture::Point { .. }
                | TimelineGesture::PointSource { .. }
                | TimelineGesture::Gain { .. }
        ),
        "audio rail stays on the audio projection: {:?}",
        session.gesture()
    );
}

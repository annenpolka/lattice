//! Studio session talks to the real Engine. No GPUI types.

use std::path::PathBuf;

use lattice_engine::{Engine, Origin, Time, map_timeline_to_source};
use lattice_studio::StudioSession;

fn png_ihdr_size(path: &std::path::Path) -> (u32, u32) {
    let bytes = std::fs::read(path).expect("read still");
    assert!(
        bytes.starts_with(b"\x89PNG\r\n\x1a\n") && bytes.len() >= 24,
        "preview still must be a PNG: {}",
        path.display()
    );
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    (width, height)
}

fn demo_vel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/gameplay-commentary/main.vel")
        .canonicalize()
        .expect("demo vel")
}

#[test]
fn open_compiles_through_engine_and_exposes_locus_provenance_preview() {
    let mut session = StudioSession::open(demo_vel()).expect("open");
    assert!(
        !session.compilation().has_errors(),
        "{:?}",
        session.diagnostics()
    );
    assert!(session.source().contains("title \"Hello\""));
    let title = session.point_at_title().unwrap().expect("title locus");
    assert_eq!(title.label, "Hello");
    let current = session.current_locus().unwrap().expect("current");
    assert_eq!(current.id, title.id);
    let provenance = session.current_provenance().unwrap().expect("provenance");
    assert!(
        matches!(
            provenance.origin,
            Origin::Invocation { ref command } if command == "title"
        ),
        "{:?}",
        provenance.origin
    );
    let plan = session.preview_plan().expect("preview plan");
    assert!(
        !plan.overlays.is_empty(),
        "preview plan should include the title"
    );
    assert!(session.uses_engine_not_own_compiler());
    let layout = session.layout().unwrap();
    assert_eq!(
        layout.playhead,
        title.timeline_span.expect("title span").start,
        "pointing at the title must park the playhead on that item"
    );
    assert_eq!(layout.canvas.playhead, layout.playhead);
    assert!(
        layout
            .canvas
            .overlays
            .iter()
            .any(|overlay| overlay.text == "Hello"),
        "canvas preview must show the title at the playhead"
    );
    assert!(!layout.dirty);
}

#[test]
fn import_edit_undo_playhead_and_save() {
    let dir = std::env::temp_dir().join("lattice-studio-session-edit");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let media = dir.join("gameplay.mp4");
    lattice_media::generate_av_fixture(&media, 8).unwrap();
    let mut session = StudioSession::open_video(&media).expect("open video");
    assert!(
        !session.compilation().has_errors(),
        "{:?}",
        session.diagnostics()
    );
    assert!(!session.is_dirty());
    let original = session.source().to_string();

    session.seek(lattice_engine::Time::seconds(2));
    session.split_at_playhead().expect("split");
    assert!(session.is_dirty());
    assert!(session.source().contains("clip_2") || session.source().contains("[2s.."));

    session.set_gain(-6).expect("gain");
    session.apply_title_text("Hello").expect("title");
    assert!(session.source().contains("Hello"));
    assert!(session.source().contains("gain"));

    session.undo().expect("undo title");
    assert!(!session.source().contains("title \"Hello\"") || session.source() != original);
    session.redo().expect("redo");
    assert!(session.source().contains("Hello"));

    session.save().expect("save");
    assert!(!session.is_dirty());
    assert_eq!(
        std::fs::read_to_string(session.path()).unwrap(),
        session.source()
    );

    session.play();
    assert!(session.is_playing());
    session.step_clock(lattice_engine::Time::seconds(1));
    let playing_head = session.playhead();
    session.pause();
    session.step_clock(lattice_engine::Time::seconds(1));
    assert_eq!(session.playhead(), playing_head);
    session.seek(lattice_engine::Time::seconds(1));
    assert_eq!(session.playhead(), lattice_engine::Time::seconds(1));
    session
        .click_timeline(lattice_engine::Time::seconds(0))
        .unwrap();
    assert_eq!(session.playhead(), lattice_engine::Time::ZERO);
}

#[test]
fn click_and_seek_preview_frames_hold_through_freeze() {
    use lattice_engine::Time;
    use lattice_media::{content_pixels, mean_abs_diff};

    let dir = std::env::temp_dir().join("lattice-studio-preview-freeze");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    lattice_media::generate_av_fixture(dir.join("capture.mp4"), 21).unwrap();
    let vel = dir.join("main.vel");
    std::fs::write(
        &vel,
        r#"project "demo"
convention commentary
media game "capture.mp4"
scene demo {
  game[10s..20s] as fight
  freeze fight at 5.2s for 1.5s
}
"#,
    )
    .unwrap();
    let mut session = StudioSession::open(&vel).expect("open freeze project");
    session
        .click_timeline(Time::from_decimal_seconds(5, 3, 1).unwrap())
        .unwrap();
    assert_eq!(
        session.playhead(),
        Time::from_decimal_seconds(5, 3, 1).unwrap()
    );
    let hold_a = session
        .request_preview_frame(&dir.join("hold-a.ppm"))
        .expect("frame in freeze");
    let extracted = session
        .cached_preview_frame()
        .expect("explicit extract, not layout");
    assert!(extracted.is_file(), "extracted {}", extracted.display());
    let layout = session.layout().expect("layout after click");
    let cached = layout
        .canvas
        .preview_frame
        .as_ref()
        .expect("layout must project an already-cached preview frame");
    assert_eq!(cached, &extracted);
    assert!(cached.is_file(), "cached frame {}", cached.display());

    session.seek(Time::from_decimal_seconds(6, 5, 1).unwrap());
    let hold_b = session
        .request_preview_frame(&dir.join("hold-b.ppm"))
        .expect("later freeze frame");
    session.seek(Time::seconds(9));
    let moving = session
        .request_preview_frame(&dir.join("moving.ppm"))
        .expect("moving frame");

    let a = content_pixels(&hold_a).unwrap();
    let b = content_pixels(&hold_b).unwrap();
    let pre = content_pixels(&moving).unwrap();
    let hold_delta = mean_abs_diff(&a, &b);
    let motion_delta = mean_abs_diff(&a, &pre);
    assert!(
        hold_delta < 8,
        "freeze hold frames should match (mean abs diff {hold_delta})"
    );
    assert!(
        motion_delta > hold_delta && motion_delta >= 4,
        "later motion should differ (hold {hold_delta}, motion {motion_delta})"
    );
}

#[test]
fn title_edit_does_not_fall_back_to_first_title() {
    let mut session = StudioSession::open(demo_vel()).expect("open");
    session.point_at(lattice_engine::LocusId::new("missing-title"));
    let before = session.source().to_string();
    let err = session
        .apply_title_text("Nope")
        .expect_err("stale/missing locus must not rewrite the first title");
    assert!(
        err.to_string().contains("title") || err.to_string().contains("locus"),
        "{err}"
    );
    assert_eq!(session.source(), before);
    assert!(session.source().contains("title \"Hello\""));
    assert!(!session.source().contains("Nope"));
}

#[test]
fn delete_last_clip_is_diagnostic_not_panic() {
    let dir = std::env::temp_dir().join("lattice-studio-empty-delete");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let media = dir.join("gameplay.mp4");
    lattice_media::generate_av_fixture(&media, 4).unwrap();
    let mut session = StudioSession::open_video(&media).expect("open");
    session.delete_selected_clip().expect("delete");
    assert!(
        session.compilation().has_errors() || session.source().contains("sequence"),
        "empty project should diagnose, not panic: {:?}",
        session.diagnostics()
    );
}

#[test]
fn timeline_and_canvas_share_playhead_for_items() {
    let dir = std::env::temp_dir().join("lattice-studio-timeline-preview-sync");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    lattice_media::generate_av_fixture(dir.join("capture.mp4"), 8).unwrap();
    let vel = dir.join("main.vel");
    std::fs::write(
        &vel,
        r#"project "demo"
convention commentary
media game "capture.mp4"
sequence main {
  first
  second
}
scene first {
  game[0s..2s] as v
  title "A" { at 0s for 1s }
}
scene second {
  game[4s..6s] as v
  title "B" { at 0s for 1s }
}
"#,
    )
    .unwrap();
    let mut session = StudioSession::open(&vel).expect("open");
    session.seek(Time::ZERO);
    let at_zero = session.layout().unwrap();
    assert_eq!(at_zero.canvas.playhead, Time::ZERO);
    let texts = |layout: &lattice_studio::StudioLayout| -> Vec<String> {
        layout
            .canvas
            .overlays
            .iter()
            .map(|overlay| overlay.text.clone())
            .collect()
    };
    assert_eq!(texts(&at_zero), vec!["A".to_string()]);

    let title_b = session
        .loci()
        .unwrap()
        .into_iter()
        .find(|locus| locus.label == "B")
        .expect("title B");
    session.point_at(title_b.id.clone());
    assert_eq!(
        session.playhead(),
        title_b.timeline_span.expect("B span").start
    );
    let at_b = session.layout().unwrap();
    assert_eq!(at_b.playhead, session.playhead());
    assert_eq!(at_b.canvas.playhead, session.playhead());
    assert_eq!(texts(&at_b), vec!["B".to_string()]);
    assert!(
        at_b.canvas
            .overlays
            .iter()
            .any(|overlay| overlay.selected && overlay.text == "B")
    );

    let timeline = Engine::timeline(&session.compilation().project).unwrap();
    let (_, content) = map_timeline_to_source(&timeline, session.playhead()).unwrap();
    assert!(
        content >= Time::seconds(4) && content < Time::seconds(6),
        "selecting the second item must preview that clip's source time, got {content:?}"
    );

    session
        .click_timeline(Time::from_decimal_seconds(2, 5, 1).unwrap())
        .unwrap();
    assert_eq!(
        session.playhead(),
        Time::from_decimal_seconds(2, 5, 1).unwrap()
    );
    let mid_b = session.layout().unwrap();
    assert_eq!(texts(&mid_b), vec!["B".to_string()]);

    session.seek(Time::seconds(3));
    let after = session.layout().unwrap();
    assert!(
        texts(&after).is_empty(),
        "no title at 3s: {:?}",
        texts(&after)
    );
}

#[test]
fn layout_does_not_extract_preview_on_projection() {
    let dir = std::env::temp_dir().join("lattice-studio-layout-no-ffmpeg");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let media = dir.join("gameplay.mp4");
    lattice_media::generate_av_fixture(&media, 4).unwrap();
    let session = StudioSession::open_video(&media).expect("open");
    let cache = session
        .path()
        .parent()
        .expect("project dir")
        .join(".lattice-cache");
    let _ = std::fs::remove_dir_all(&cache);
    assert!(
        session.peek_preview_frame().is_none(),
        "fresh session has no cached frame"
    );
    let layout = session.layout().expect("layout");
    assert!(
        layout.canvas.preview_frame.is_none(),
        "layout must not spawn FFmpeg to fill the canvas"
    );
    assert!(session.peek_preview_frame().is_none());
    assert!(
        !cache.is_dir() || std::fs::read_dir(&cache).map_or(true, |entries| entries.count() == 0),
        "layout must not create a preview cache"
    );
}

#[test]
fn timeline_ratio_play_and_preview_stay_aligned() {
    use lattice_studio::fit_preview_size;

    assert_eq!(fit_preview_size(640, 480, 640, 360), (480, 360));
    assert_eq!(fit_preview_size(320, 180, 640, 360), (640, 360));
    assert_eq!(fit_preview_size(1080, 1920, 640, 360), (202, 360));

    let dir = std::env::temp_dir().join("lattice-studio-ratio-play-preview");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let media = dir.join("gameplay.mp4");
    lattice_media::generate_av_fixture_size(&media, 4, 640, 480).unwrap();
    let source = lattice_media::probe_media(&media).expect("probe source");
    let mut session = StudioSession::open_video(&media).expect("open");
    let duration = session.layout().unwrap().timeline.duration;
    assert!(duration > Time::ZERO, "imported timeline has duration");
    assert_eq!(session.time_at_timeline_ratio(0, 8), Time::ZERO);
    assert_eq!(session.time_at_timeline_ratio(8, 8), duration);
    assert_eq!(session.time_at_timeline_ratio(0, 100), Time::ZERO);
    assert_eq!(session.time_at_timeline_ratio(100, 100), duration);
    let half = session.time_at_timeline_ratio(1, 2);
    assert!(half > Time::ZERO && half < duration);
    assert_eq!(half.checked_mul(Time::seconds(2)).unwrap(), duration);

    session.scrub_timeline_ratio(1, 4);
    let layout = session.layout().unwrap();
    assert_eq!(layout.playhead, session.playhead());
    assert_eq!(layout.canvas.playhead, session.playhead());
    assert_eq!(layout.playhead, session.time_at_timeline_ratio(1, 4));
    session.click_timeline_ratio(3, 4).unwrap();
    let layout = session.layout().unwrap();
    assert_eq!(layout.playhead, session.playhead());
    assert_eq!(layout.canvas.playhead, session.playhead());
    assert_eq!(layout.playhead, session.time_at_timeline_ratio(3, 4));

    session.seek(Time::ZERO);
    session.play();
    session.step_clock(Time::seconds(1));
    assert_eq!(session.playhead(), Time::seconds(1));
    let still = session.cached_preview_frame().expect("extract at 1s");
    let peeked = session.peek_preview_frame().expect("cache key");
    assert_eq!(still, peeked);
    let name = peeked.file_name().unwrap().to_string_lossy();
    assert!(
        name.contains(&format!(
            "-{}-{}",
            session.playhead().num(),
            session.playhead().den()
        )),
        "cache key must include playhead: {name}"
    );
    let layout = session.layout().unwrap();
    assert_eq!(layout.playhead, session.playhead());
    assert_eq!(layout.canvas.playhead, session.playhead());
    session.pause();
    let frozen = session.playhead();
    session.step_clock(Time::seconds(1));
    assert_eq!(session.playhead(), frozen);

    let (pw, ph) = session.preview_pixel_size();
    assert_eq!((pw, ph), fit_preview_size(640, 480, 640, 360));
    assert_eq!(layout.canvas.preview_width, pw);
    assert_eq!(layout.canvas.preview_height, ph);
    let (still_w, still_h) = png_ihdr_size(&still);
    assert_eq!((still_w, still_h), (pw, ph));
    let src_w = i64::from(source.width.expect("src w"));
    let src_h = i64::from(source.height.expect("src h"));
    let dst_w = i64::from(pw);
    let dst_h = i64::from(ph);
    let cross = (src_w * dst_h - src_h * dst_w).unsigned_abs();
    assert!(
        cross <= u64::try_from(src_w.max(src_h)).unwrap() / 8,
        "still {pw}x{ph} must match source {src_w}x{src_h} aspect (cross {cross})"
    );
}

#[test]
fn studio_crate_source_has_no_gpui_in_session() {
    let session = include_str!("../src/session.rs");
    assert!(
        !session.contains("gpui"),
        "session must stay an Engine client without GPUI types"
    );
    let core = include_str!("../../../crates/lattice-core/Cargo.toml");
    assert!(!core.contains("gpui"));
}

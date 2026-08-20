//! Import probe → VEL → compile, and preview-frame `TimeMap` mapping.

use lattice_core::Time;
use lattice_engine::{Engine, PreviewFrameRequest, generate_av_fixture};
use lattice_media::{content_pixels, mean_abs_diff, preview_frame};

#[test]
fn import_probe_compile_timeline_matches_duration() {
    let dir = std::env::temp_dir().join("lattice-import-preview");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let media = dir.join("gameplay.mp4");
    generate_av_fixture(&media, 8).expect("fixture");
    let engine = Engine::default();
    let imported = engine
        .import_media(&media, Some(&dir.join("proj")))
        .expect("import");
    assert!(imported.media_info.has_video);
    assert!(imported.media_info.has_audio);
    assert!(imported.media_info.width.is_some());
    assert!(imported.media_info.height.is_some());
    assert!(imported.source.contains(&imported.locator));
    assert!(!imported.source.contains("testsrc"));
    let compilation = engine.compile_path(&imported.vel_path).unwrap();
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    let timeline = Engine::timeline(&compilation.project).unwrap();
    let probed = imported.media_info.duration;
    let delta = if timeline.duration > probed {
        timeline.duration - probed
    } else {
        probed - timeline.duration
    };
    assert!(
        delta < Time::milliseconds(50),
        "timeline {} vs probed {}",
        timeline.duration,
        probed
    );
}

#[test]
fn preview_frames_hold_through_freeze() {
    let dir = std::env::temp_dir().join("lattice-preview-freeze");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    generate_av_fixture(dir.join("capture.mp4"), 21).unwrap();
    let vel = r#"project "demo"
convention commentary
media game "capture.mp4"
scene demo {
  game[10s..20s] as fight
  freeze fight at 5.2s for 1.5s
}
"#;
    let engine = Engine::default();
    let compilation = engine.compile(vel).unwrap();
    let timeline = Engine::timeline(&compilation.project).unwrap();
    let hold_a = preview_frame(
        &timeline,
        &PreviewFrameRequest {
            timeline_time: Time::from_decimal_seconds(5, 3, 1).unwrap(),
            width: 320,
            height: 180,
        },
        &dir,
        &dir.join("hold-a.ppm"),
        false,
    )
    .unwrap();
    let hold_b = preview_frame(
        &timeline,
        &PreviewFrameRequest {
            timeline_time: Time::from_decimal_seconds(6, 5, 1).unwrap(),
            width: 320,
            height: 180,
        },
        &dir,
        &dir.join("hold-b.ppm"),
        false,
    )
    .unwrap();
    let moving = preview_frame(
        &timeline,
        &PreviewFrameRequest {
            timeline_time: Time::seconds(9),
            width: 320,
            height: 180,
        },
        &dir,
        &dir.join("moving.ppm"),
        false,
    )
    .unwrap();
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

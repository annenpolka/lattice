//! Shipped parse → compile → timeline → export path.
//! Starts from the user VEL file, not a pre-lowered IR fixture.

use lattice_core::Time;
use lattice_engine::{Engine, PreviewOptions};
use lattice_media::{
    content_pixels, extract_frame, generate_av_fixture, mean_abs_diff, near_white_pixels,
    plan_from_timeline, probe_duration, title_bar_present,
};

const VEL: &str = include_str!("../../../examples/gameplay-commentary/main.vel");

fn expected_duration() -> Time {
    let source = Time::seconds(20)
        .checked_sub(Time::seconds(10))
        .expect("10s slice");
    let hold = Time::from_decimal_seconds(1, 5, 1).expect("1.5s hold");
    source.checked_add(hold).expect("10s + 1.5s")
}

fn title_at() -> Time {
    Time::seconds(2)
}

fn title_for() -> Time {
    Time::seconds(3)
}

#[test]
fn compile_timeline_from_user_vel() {
    let compilation = Engine::default().compile(VEL).expect("compile");
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);

    let timeline = Engine::timeline(&compilation.project).expect("flatten");
    assert_eq!(timeline.duration, expected_duration());

    let holds = timeline.freeze_segments();
    assert_eq!(holds.len(), 1);
    assert_eq!(holds[0].rate, Time::ZERO);
    assert_eq!(
        holds[0].local_duration,
        Time::from_decimal_seconds(1, 5, 1).unwrap()
    );

    let title = timeline
        .title_clips()
        .next()
        .expect("title placement on timeline");
    assert_eq!(title.span.start, title_at());
    assert_eq!(title.span.duration, title_for());
    assert_eq!(title.text.as_deref(), Some("Hello"));

    let plan = plan_from_timeline(&timeline).expect("render plan");
    assert_eq!(plan.duration, expected_duration());
    assert!(
        plan.segments.iter().any(|segment| segment.hold),
        "plan must include a freeze hold"
    );
    assert_eq!(plan.overlays[0].span.start, title_at());
    assert_eq!(plan.overlays[0].span.duration, title_for());
}

#[test]
fn export_preview_matches_timeline_duration_and_title_window() {
    let compilation = Engine::default().compile(VEL).expect("compile");
    let dir = std::env::temp_dir().join("lattice-walking-export");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    generate_av_fixture(dir.join("capture.mp4"), 21).expect("fixture");
    let out = dir.join("preview.mp4");
    let report = Engine::default()
        .render(&compilation.project, &out, &dir)
        .expect("render");
    assert!(out.is_file(), "missing {}", out.display());

    let expected = expected_duration();
    assert_eq!(report.duration, expected);
    let probed = probe_duration(&out).expect("ffprobe");
    assert_eq!(probed, expected);

    let during = extract_frame(&out, Time::seconds(3), &dir.join("during.ppm")).expect("frame @3s");
    let before = extract_frame(&out, Time::seconds(1), &dir.join("before.ppm")).expect("frame @1s");
    let during_white = near_white_pixels(&during, 0.7, 1.0).unwrap_or(0);
    let before_white = near_white_pixels(&before, 0.7, 1.0).unwrap_or(0);
    let yellow = title_bar_present(&during).unwrap_or(false);
    assert!(
        during_white > before_white.saturating_add(20) || yellow,
        "title overlay should be visible at 3s (white {during_white} vs {before_white}, yellow {yellow})"
    );
    assert!(
        !title_bar_present(&before).expect("scan before") && before_white < during_white.max(1),
        "title overlay should be off at 1s"
    );

    let hold_a = extract_frame(
        &out,
        Time::from_decimal_seconds(5, 3, 1).unwrap(),
        &dir.join("hold-a.ppm"),
    )
    .expect("frame in hold");
    let hold_b = extract_frame(
        &out,
        Time::from_decimal_seconds(6, 5, 1).unwrap(),
        &dir.join("hold-b.ppm"),
    )
    .expect("later frame in hold");
    let moving = extract_frame(&out, Time::seconds(1), &dir.join("moving.ppm")).expect("pre-hold");
    let a = content_pixels(&hold_a).unwrap();
    let b = content_pixels(&hold_b).unwrap();
    let pre = content_pixels(&moving).unwrap();
    let hold_delta = mean_abs_diff(&a, &b);
    let motion_delta = mean_abs_diff(&a, &pre);
    assert!(
        hold_delta < 8,
        "freeze hold frames should stay still (mean abs diff {hold_delta})"
    );
    assert!(
        motion_delta > hold_delta.saturating_add(8),
        "pre-freeze motion should differ more than the hold (hold {hold_delta}, motion {motion_delta})"
    );
}

#[test]
fn compile_is_deterministic() {
    let a = Engine::default().compile(VEL).unwrap();
    let b = Engine::default().compile(VEL).unwrap();
    let ta = Engine::timeline(&a.project).unwrap();
    let tb = Engine::timeline(&b.project).unwrap();
    assert_eq!(ta, tb);
    assert_eq!(ta.duration, expected_duration());
}

#[test]
fn missing_media_production_render_fails_without_testsrc() {
    let compilation = Engine::default().compile(VEL).expect("compile");
    let dir = std::env::temp_dir().join("lattice-missing-media");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("should-not-exist.mp4");
    let err = Engine::default()
        .render(&compilation.project, &out, &dir)
        .expect_err("missing media must fail");
    assert!(
        err.to_string().contains("missing") || err.to_string().contains("Missing"),
        "{err}"
    );
    assert!(!out.is_file(), "must not write a testsrc stand-in");
}

#[test]
fn missing_media_fixture_hook_still_works_when_requested() {
    let compilation = Engine::default().compile(VEL).expect("compile");
    let dir = std::env::temp_dir().join("lattice-fixture-hook");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("preview.mp4");
    Engine::default()
        .render_with_options(
            &compilation.project,
            &PreviewOptions {
                output: out.clone(),
                media_root: dir.clone(),
                lock: None,
                allow_fixtures: true,
                font: None,
            },
        )
        .expect("explicit fixture hook");
    assert!(out.is_file());
}

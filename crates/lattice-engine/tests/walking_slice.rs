//! Shipped parse → compile → timeline → export path.
//! Starts from the user VEL file, not a pre-lowered IR fixture.

use lattice_core::Time;
use lattice_engine::Engine;
use lattice_media::{extract_frame, plan_from_timeline, probe_duration, title_bar_present};

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
    assert!(
        title_bar_present(&during).expect("scan during"),
        "title overlay should be on at 3s"
    );
    assert!(
        !title_bar_present(&before).expect("scan before"),
        "title overlay should be off at 1s"
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

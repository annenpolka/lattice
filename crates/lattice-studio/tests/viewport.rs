//! GPUI-free viewport Time ↔ pixel conversions.

use lattice_engine::Time;
use lattice_studio::TimelineViewport;

fn secs(n: i64) -> Time {
    Time::seconds(n)
}

fn near(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

#[test]
fn x_time_x_round_trip_within_one_pixel() {
    for duration in [secs(8), secs(21)] {
        for (start, visible) in [(Time::ZERO, duration), (secs(2), secs(4))] {
            let vp = TimelineViewport::new(start, visible, 640.0);
            for x in [0.0, 17.0, 160.0, 319.5, 512.0, 639.0] {
                let time = vp.time_at_x(x);
                let back = vp.x_at_time(time);
                assert!(
                    (back - x).abs() < 1.0,
                    "round-trip x={x} time={time} back={back} duration={duration:?} visible={visible:?}"
                );
            }
        }
    }
}

#[test]
fn zoom_and_scroll_change_the_mapping() {
    let mut vp = TimelineViewport::fit(secs(10), 1000.0);
    let mid = vp.time_at_x(250.0);
    let x_before = vp.x_at_time(mid);
    assert!(near(x_before, 250.0, 1.0));

    vp.zoom_around(mid, 2.0);
    let x_after_zoom = vp.x_at_time(mid);
    assert!(
        (x_after_zoom - x_before).abs() < 1.0,
        "zoom must keep the anchor pixel, {x_after_zoom} vs {x_before}"
    );
    assert!(
        vp.visible_duration() < secs(10),
        "zoom in shrinks visible duration"
    );

    let before_scroll = vp.time_at_x(100.0);
    vp.scroll_by_pixels(80.0);
    let after_scroll = vp.time_at_x(100.0);
    assert_ne!(
        before_scroll, after_scroll,
        "scroll must change which time sits at x=100"
    );
}

#[test]
fn zero_length_and_out_of_rail_do_not_panic() {
    let empty = TimelineViewport::new(Time::ZERO, Time::ZERO, 640.0);
    assert_eq!(empty.time_at_x(0.0), Time::ZERO);
    assert_eq!(empty.time_at_x(-40.0), Time::ZERO);
    assert_eq!(empty.time_at_x(900.0), Time::ZERO);
    assert!(empty.x_at_time(secs(3)).abs() < f64::EPSILON);
    let _ = empty.delta_time(12.0);

    let zero_width = TimelineViewport::new(secs(1), secs(5), 0.0);
    assert_eq!(zero_width.time_at_x(10.0), secs(1));
    assert!(zero_width.x_at_time(secs(3)).abs() < f64::EPSILON);

    let vp = TimelineViewport::fit(secs(10), 200.0);
    let before = vp.time_at_x(-50.0);
    let after = vp.time_at_x(250.0);
    assert!(before < Time::ZERO, "x left of rail is unclamped: {before}");
    assert!(after > secs(10), "x right of rail is unclamped: {after}");
}

#[test]
fn rational_time_survives_pixel_mapping() {
    let duration = Time::new(5, 2).unwrap();
    let vp = TimelineViewport::fit(duration, 400.0);
    let x = vp.x_at_time(Time::new(1, 2).unwrap());
    let back = vp.time_at_x(x);
    let x2 = vp.x_at_time(back);
    assert!(
        (x - x2).abs() < 1.0,
        "rational 1/2s must round-trip in pixels"
    );
}

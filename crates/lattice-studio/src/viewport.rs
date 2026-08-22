//! GPUI-free timeline viewport: Time ↔ pixel conversions.
//!
//! Pixel coordinates stay in Studio. Core never sees them.

#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

use lattice_engine::Time;

/// Visible slice of the timeline mapped onto a rail width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineViewport {
    visible_start: Time,
    visible_duration: Time,
    width_pixels: f64,
}

impl TimelineViewport {
    pub const DEFAULT_WIDTH: f64 = 640.0;

    #[must_use]
    pub fn new(visible_start: Time, visible_duration: Time, width_pixels: f64) -> Self {
        Self {
            visible_start,
            visible_duration: if visible_duration < Time::ZERO {
                Time::ZERO
            } else {
                visible_duration
            },
            width_pixels: width_pixels.max(0.0),
        }
    }

    /// Show the whole project in `width_pixels`. Zero-length projects stay zero.
    #[must_use]
    pub fn fit(project_duration: Time, width_pixels: f64) -> Self {
        Self::new(
            Time::ZERO,
            max_time(project_duration, Time::ZERO),
            width_pixels,
        )
    }

    #[must_use]
    pub fn visible_start(self) -> Time {
        self.visible_start
    }

    #[must_use]
    pub fn visible_duration(self) -> Time {
        self.visible_duration
    }

    #[must_use]
    pub fn visible_end(self) -> Time {
        self.visible_start
            .checked_add(self.visible_duration)
            .unwrap_or(self.visible_start)
    }

    #[must_use]
    pub fn width_pixels(self) -> f64 {
        self.width_pixels
    }

    pub fn set_width(&mut self, width_pixels: f64) {
        self.width_pixels = width_pixels.max(0.0);
    }

    pub fn set_visible_start(&mut self, start: Time) {
        self.visible_start = start;
    }

    /// Timeline time at rail-local `x`. Does not clamp: x left of the rail is
    /// before `visible_start`, x right of the rail is after `visible_end`.
    /// Zero-length or zero-width viewports return `visible_start`.
    #[must_use]
    pub fn time_at_x(self, x: f64) -> Time {
        if self.width_pixels == 0.0 || self.visible_duration.is_zero() {
            return self.visible_start;
        }
        if !x.is_finite() {
            return self.visible_start;
        }
        let fraction = x / self.width_pixels;
        let delta = time_from_secs(time_as_secs(self.visible_duration) * fraction);
        self.visible_start
            .checked_add(delta)
            .unwrap_or(self.visible_start)
    }

    /// Rail-local x for `time`. Times outside the visible range map outside
    /// `[0, width]`. Zero-length or zero-width viewports return `0.0`.
    #[must_use]
    pub fn x_at_time(self, time: Time) -> f64 {
        if self.width_pixels == 0.0 || self.visible_duration.is_zero() {
            return 0.0;
        }
        let delta = time_as_secs(time) - time_as_secs(self.visible_start);
        let duration = time_as_secs(self.visible_duration);
        if duration == 0.0 {
            return 0.0;
        }
        (delta / duration) * self.width_pixels
    }

    /// Time delta matching a pixel delta at the current scale.
    #[must_use]
    pub fn delta_time(self, delta_x: f64) -> Time {
        if self.width_pixels == 0.0 || self.visible_duration.is_zero() || !delta_x.is_finite() {
            return Time::ZERO;
        }
        time_from_secs(time_as_secs(self.visible_duration) * (delta_x / self.width_pixels))
    }

    /// Pixel delta matching a time delta at the current scale.
    #[must_use]
    pub fn delta_x(self, delta: Time) -> f64 {
        if self.width_pixels == 0.0 || self.visible_duration.is_zero() {
            return 0.0;
        }
        let duration = time_as_secs(self.visible_duration);
        if duration == 0.0 {
            return 0.0;
        }
        (time_as_secs(delta) / duration) * self.width_pixels
    }

    /// Zoom so `anchor` stays at the same pixel. `factor` > 1 zooms in.
    pub fn zoom_around(&mut self, anchor: Time, factor: f64) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let old_duration = time_as_secs(self.visible_duration);
        if old_duration <= 0.0 {
            return;
        }
        let new_duration = (old_duration / factor).clamp(MIN_VISIBLE_SECS, MAX_VISIBLE_SECS);
        let anchor_secs = time_as_secs(anchor);
        let start_secs = time_as_secs(self.visible_start);
        let t = (anchor_secs - start_secs) / old_duration;
        let new_start = anchor_secs - t * new_duration;
        self.visible_start = time_from_secs(new_start);
        self.visible_duration = time_from_secs(new_duration);
    }

    /// Shift the visible window by a pixel delta (positive = later times).
    pub fn scroll_by_pixels(&mut self, delta_x: f64) {
        let delta = self.delta_time(delta_x);
        self.visible_start = self
            .visible_start
            .checked_add(delta)
            .unwrap_or(self.visible_start);
    }

    /// Keep the visible window intersecting `[0, project_duration]`.
    pub fn clamp_to_project(&mut self, project_duration: Time) {
        let max = max_time(project_duration, Time::ZERO);
        if self.visible_duration.is_zero() {
            self.visible_start = Time::ZERO;
            return;
        }
        if self.visible_duration > max && max > Time::ZERO {
            self.visible_duration = max;
            self.visible_start = Time::ZERO;
            return;
        }
        if self.visible_start < Time::ZERO {
            self.visible_start = Time::ZERO;
        }
        let end = self.visible_end();
        if end > max {
            let overflow = end.checked_sub(max).unwrap_or(Time::ZERO);
            self.visible_start = self
                .visible_start
                .checked_sub(overflow)
                .unwrap_or(Time::ZERO);
            if self.visible_start < Time::ZERO {
                self.visible_start = Time::ZERO;
            }
        }
    }

    /// Fit the project again (zoom-to-fit).
    pub fn fit_project(&mut self, project_duration: Time) {
        *self = Self::fit(project_duration, self.width_pixels);
    }
}

const MIN_VISIBLE_SECS: f64 = 0.05;
const MAX_VISIBLE_SECS: f64 = 86_400.0;

#[must_use]
pub fn time_as_secs(time: Time) -> f64 {
    let den = time.den().max(1);
    time.num() as f64 / den as f64
}

#[must_use]
pub fn time_from_secs(secs: f64) -> Time {
    if !secs.is_finite() {
        return Time::ZERO;
    }
    let us = (secs * 1_000_000.0).round();
    let us = if us >= i64::MAX as f64 {
        i64::MAX
    } else if us <= i64::MIN as f64 {
        i64::MIN
    } else {
        us as i64
    };
    Time::new(us, 1_000_000).unwrap_or(Time::ZERO)
}

#[must_use]
pub fn clamp_time(time: Time, max: Time) -> Time {
    if time < Time::ZERO {
        Time::ZERO
    } else if time > max {
        max
    } else {
        time
    }
}

fn max_time(a: Time, b: Time) -> Time {
    if a > b { a } else { b }
}

/// Interaction clamp: playhead / scrub stay inside `[0, duration]`.
#[must_use]
pub fn clamp_interaction_time(time: Time, project_duration: Time) -> Time {
    clamp_time(time, max_time(project_duration, Time::ZERO))
}

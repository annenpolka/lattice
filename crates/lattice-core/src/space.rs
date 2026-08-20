use serde::{Deserialize, Serialize};
use std::fmt;

/// Canvas-space precision used by [`NormalizedPosition`].
///
/// `10_000` basis points is `100%`. Core never stores preview pixels, so the
/// same position survives canvas resizing and is shared by preview/export.
pub const CANVAS_BASIS_POINTS: u16 = 10_000;

/// Uniform overlay scale precision (`1000` = `100%`).
pub const OVERLAY_SCALE_ONE: u16 = 1_000;
pub const OVERLAY_SCALE_MIN: u16 = 250;
pub const OVERLAY_SCALE_MAX: u16 = 2_000;

/// Uniform, aspect-preserving overlay scale stored independently of pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedScale {
    pub milli: u16,
}

impl NormalizedScale {
    pub const ONE: Self = Self {
        milli: OVERLAY_SCALE_ONE,
    };

    pub const fn new(milli: u16) -> Option<Self> {
        if milli >= OVERLAY_SCALE_MIN && milli <= OVERLAY_SCALE_MAX {
            Some(Self { milli })
        } else {
            None
        }
    }

    /// Interaction constructor bounded by the product's 25%..=200% range.
    pub fn clamped(milli: i64) -> Self {
        Self {
            milli: u16::try_from(
                milli.clamp(i64::from(OVERLAY_SCALE_MIN), i64::from(OVERLAY_SCALE_MAX)),
            )
            .unwrap_or(OVERLAY_SCALE_ONE),
        }
    }

    #[must_use]
    pub fn scaled_extent(self, extent: u32) -> u32 {
        let scaled = u64::from(extent) * u64::from(self.milli) / u64::from(OVERLAY_SCALE_ONE);
        u32::try_from(scaled).unwrap_or(u32::MAX).max(1)
    }

    /// Clamp a requested scale so the scaled bounds still fit the canvas.
    #[must_use]
    pub fn fit_within(
        self,
        base_width: u32,
        base_height: u32,
        canvas_width: u32,
        canvas_height: u32,
    ) -> Self {
        let width_limit = if base_width == 0 {
            u64::from(OVERLAY_SCALE_MAX)
        } else {
            u64::from(canvas_width) * u64::from(OVERLAY_SCALE_ONE) / u64::from(base_width)
        };
        let height_limit = if base_height == 0 {
            u64::from(OVERLAY_SCALE_MAX)
        } else {
            u64::from(canvas_height) * u64::from(OVERLAY_SCALE_ONE) / u64::from(base_height)
        };
        let upper = width_limit
            .min(height_limit)
            .min(u64::from(OVERLAY_SCALE_MAX));
        Self::clamped(i64::try_from(u64::from(self.milli).min(upper)).unwrap_or(i64::MAX))
    }
}

impl Default for NormalizedScale {
    fn default() -> Self {
        Self::ONE
    }
}

impl fmt::Display for NormalizedScale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let whole = self.milli / 10;
        let fraction = self.milli % 10;
        if fraction == 0 {
            write!(f, "{whole}%")
        } else {
            write!(f, "{whole}.{fraction}%")
        }
    }
}

/// A point in normalized Canvas Space (`0..=10_000` on each axis).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedPosition {
    pub x: u16,
    pub y: u16,
}

impl NormalizedPosition {
    pub const ORIGIN: Self = Self { x: 0, y: 0 };

    /// Construct a position, rejecting coordinates outside Canvas Space.
    pub const fn new(x: u16, y: u16) -> Option<Self> {
        if x <= CANVAS_BASIS_POINTS && y <= CANVAS_BASIS_POINTS {
            Some(Self { x, y })
        } else {
            None
        }
    }

    /// Construct a position for pointer interaction, clamping it to Canvas Space.
    pub fn clamped(x: i64, y: i64) -> Self {
        let max = i64::from(CANVAS_BASIS_POINTS);
        Self {
            x: u16::try_from(x.clamp(0, max)).unwrap_or(CANVAS_BASIS_POINTS),
            y: u16::try_from(y.clamp(0, max)).unwrap_or(CANVAS_BASIS_POINTS),
        }
    }

    /// Project an axis into pixels. Bounds belonging to an overlay are clamped
    /// separately because their size is renderer-owned.
    #[must_use]
    pub fn axis_pixels(value: u16, extent: u32) -> i32 {
        let pixels = u64::from(value) * u64::from(extent) / u64::from(CANVAS_BASIS_POINTS);
        i32::try_from(pixels).unwrap_or(i32::MAX)
    }

    /// Project a top-left point and clamp it so the full overlay stays inside
    /// the canvas. Preview chrome and renderer evaluation share this rule.
    #[must_use]
    pub fn pixel_origin(
        self,
        canvas_width: u32,
        canvas_height: u32,
        overlay_width: u32,
        overlay_height: u32,
    ) -> (i32, i32) {
        let max_x = canvas_width.saturating_sub(overlay_width);
        let max_y = canvas_height.saturating_sub(overlay_height);
        (
            Self::axis_pixels(self.x, canvas_width)
                .clamp(0, i32::try_from(max_x).unwrap_or(i32::MAX)),
            Self::axis_pixels(self.y, canvas_height)
                .clamp(0, i32::try_from(max_y).unwrap_or(i32::MAX)),
        )
    }
}

impl fmt::Display for NormalizedPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({:.2}%, {:.2}%)",
            f64::from(self.x) / 100.0,
            f64::from(self.y) / 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_space_is_checked_and_pixel_independent() {
        let middle = NormalizedPosition::new(5_000, 2_500).expect("inside canvas");
        assert_eq!(NormalizedPosition::axis_pixels(middle.x, 320), 160);
        assert_eq!(NormalizedPosition::axis_pixels(middle.y, 180), 45);
        assert!(NormalizedPosition::new(10_001, 0).is_none());
    }

    #[test]
    fn pointer_coordinates_clamp_to_canvas() {
        assert_eq!(
            NormalizedPosition::clamped(-1, 20_000),
            NormalizedPosition { x: 0, y: 10_000 }
        );
        assert_eq!(
            NormalizedPosition {
                x: 10_000,
                y: 10_000
            }
            .pixel_origin(320, 180, 80, 30),
            (240, 150)
        );
    }

    #[test]
    fn uniform_scale_has_stable_bounds_and_pixel_projection() {
        assert!(NormalizedScale::new(249).is_none());
        assert!(NormalizedScale::new(2_001).is_none());
        let scale = NormalizedScale::new(1_250).unwrap();
        assert_eq!(scale.to_string(), "125%");
        assert_eq!(scale.scaled_extent(240), 300);
        assert_eq!(NormalizedScale::clamped(99).milli, OVERLAY_SCALE_MIN);
        assert_eq!(
            NormalizedScale::new(2_000)
                .unwrap()
                .fit_within(240, 38, 320, 180)
                .milli,
            1_333
        );
    }
}

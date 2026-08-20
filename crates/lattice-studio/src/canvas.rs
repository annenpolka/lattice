//! GPUI-free canvas overlay drag lifecycle.
//!
//! Pointer pixels are ephemeral input only. The patch leaving this module is a
//! shared locus plus normalized Canvas Space and can therefore become a
//! source-backed [`lattice_engine::SemanticEdit`].

use std::fmt;

use lattice_engine::{
    LocusId, NormalizedPosition, NormalizedScale, OVERLAY_SCALE_MAX, OVERLAY_SCALE_MIN,
};

const CANVAS_DRAG_THRESHOLD_PX: f64 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasPoint {
    pub x: f64,
    pub y: f64,
}

impl CanvasPoint {
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSize {
    pub width: f64,
    pub height: f64,
}

impl CanvasSize {
    #[must_use]
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl CanvasRect {
    #[must_use]
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanvasEditPatch {
    pub locus_id: LocusId,
    pub before: NormalizedPosition,
    pub after: NormalizedPosition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl ResizeCorner {
    fn signs(self) -> (f64, f64) {
        match self {
            Self::TopLeft => (-1.0, -1.0),
            Self::TopRight => (1.0, -1.0),
            Self::BottomLeft => (-1.0, 1.0),
            Self::BottomRight => (1.0, 1.0),
        }
    }

    fn fixed_corner(self, rect: CanvasRect) -> CanvasPoint {
        match self {
            Self::TopLeft => CanvasPoint::new(rect.x + rect.width, rect.y + rect.height),
            Self::TopRight => CanvasPoint::new(rect.x, rect.y + rect.height),
            Self::BottomLeft => CanvasPoint::new(rect.x + rect.width, rect.y),
            Self::BottomRight => CanvasPoint::new(rect.x, rect.y),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasResizePreview {
    pub rect: CanvasRect,
    pub position: NormalizedPosition,
    pub scale: NormalizedScale,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasResizePatch {
    pub locus_id: LocusId,
    pub before: CanvasResizePreview,
    pub after: CanvasResizePreview,
}

/// In-flight aspect-preserving corner resize. The opposite corner is immutable.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasResize {
    locus_id: LocusId,
    corner: ResizeCorner,
    canvas: CanvasSize,
    original: CanvasResizePreview,
    preview: CanvasResizePreview,
    fixed: CanvasPoint,
    start_pointer: CanvasPoint,
    moved: bool,
}

pub type CanvasResizeError = CanvasDragError;

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasDrag {
    locus_id: LocusId,
    canvas: CanvasSize,
    overlay_width: f64,
    overlay_height: f64,
    grab_x: f64,
    grab_y: f64,
    start_pointer: CanvasPoint,
    original: NormalizedPosition,
    preview: NormalizedPosition,
    moved: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasDragError {
    InvalidCanvas,
    InvalidOverlay,
    InvalidPointer,
    PointerOutsideOverlay,
}

impl fmt::Display for CanvasDragError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidCanvas => "canvas dimensions must be finite and positive",
            Self::InvalidOverlay => "overlay bounds must be finite and non-negative",
            Self::InvalidPointer => "pointer coordinates must be finite",
            Self::PointerOutsideOverlay => "canvas drag must begin inside the overlay",
        };
        f.write_str(message)
    }
}

impl std::error::Error for CanvasDragError {}

impl CanvasDrag {
    pub fn begin(
        locus_id: LocusId,
        overlay: CanvasRect,
        canvas: CanvasSize,
        pointer: CanvasPoint,
    ) -> Result<Self, CanvasDragError> {
        validate_canvas(canvas)?;
        validate_rect(overlay)?;
        validate_pointer(pointer)?;
        let width = overlay.width.min(canvas.width);
        let height = overlay.height.min(canvas.height);
        let max_x = (canvas.width - width).max(0.0);
        let max_y = (canvas.height - height).max(0.0);
        let x = overlay.x.clamp(0.0, max_x);
        let y = overlay.y.clamp(0.0, max_y);
        if pointer.x < overlay.x
            || pointer.x > overlay.x + overlay.width
            || pointer.y < overlay.y
            || pointer.y > overlay.y + overlay.height
        {
            return Err(CanvasDragError::PointerOutsideOverlay);
        }
        let original = normalize_point(CanvasPoint::new(x, y), canvas);
        Ok(Self {
            locus_id,
            canvas,
            overlay_width: width,
            overlay_height: height,
            grab_x: (pointer.x - x).clamp(0.0, width),
            grab_y: (pointer.y - y).clamp(0.0, height),
            start_pointer: pointer,
            original,
            preview: original,
            moved: false,
        })
    }

    pub fn update(&mut self, pointer: CanvasPoint) -> Result<NormalizedPosition, CanvasDragError> {
        validate_pointer(pointer)?;
        let max_x = (self.canvas.width - self.overlay_width).max(0.0);
        let max_y = (self.canvas.height - self.overlay_height).max(0.0);
        let origin = CanvasPoint::new(
            (pointer.x - self.grab_x).clamp(0.0, max_x),
            (pointer.y - self.grab_y).clamp(0.0, max_y),
        );
        self.preview = normalize_point(origin, self.canvas);
        self.moved |= (pointer.x - self.start_pointer.x).abs() >= CANVAS_DRAG_THRESHOLD_PX
            || (pointer.y - self.start_pointer.y).abs() >= CANVAS_DRAG_THRESHOLD_PX;
        Ok(self.preview)
    }

    #[must_use]
    pub fn locus_id(&self) -> &LocusId {
        &self.locus_id
    }

    #[must_use]
    pub fn preview_position(&self) -> NormalizedPosition {
        self.preview
    }

    #[must_use]
    pub fn cancel(self) -> NormalizedPosition {
        self.original
    }

    #[must_use]
    pub fn commit(self) -> Option<CanvasEditPatch> {
        (self.moved && self.preview != self.original).then_some(CanvasEditPatch {
            locus_id: self.locus_id,
            before: self.original,
            after: self.preview,
        })
    }
}

impl CanvasResize {
    pub fn begin(
        locus_id: LocusId,
        corner: ResizeCorner,
        overlay: CanvasRect,
        canvas: CanvasSize,
        pointer: CanvasPoint,
        scale: NormalizedScale,
    ) -> Result<Self, CanvasResizeError> {
        validate_canvas(canvas)?;
        validate_rect(overlay)?;
        validate_pointer(pointer)?;
        if overlay.width <= 0.0 || overlay.height <= 0.0 {
            return Err(CanvasDragError::InvalidOverlay);
        }
        let rect = CanvasRect::new(
            overlay
                .x
                .clamp(0.0, (canvas.width - overlay.width).max(0.0)),
            overlay
                .y
                .clamp(0.0, (canvas.height - overlay.height).max(0.0)),
            overlay.width.min(canvas.width),
            overlay.height.min(canvas.height),
        );
        let original = CanvasResizePreview {
            rect,
            position: normalize_point(CanvasPoint::new(rect.x, rect.y), canvas),
            scale,
        };
        Ok(Self {
            locus_id,
            corner,
            canvas,
            original,
            preview: original,
            fixed: corner.fixed_corner(rect),
            start_pointer: pointer,
            moved: false,
        })
    }

    pub fn update(
        &mut self,
        pointer: CanvasPoint,
    ) -> Result<CanvasResizePreview, CanvasResizeError> {
        validate_pointer(pointer)?;
        let (sign_x, sign_y) = self.corner.signs();
        let diagonal = (
            sign_x * self.original.rect.width,
            sign_y * self.original.rect.height,
        );
        let pointer_vector = (pointer.x - self.fixed.x, pointer.y - self.fixed.y);
        let denominator = diagonal.0.mul_add(diagonal.0, diagonal.1 * diagonal.1);
        let projected = pointer_vector
            .0
            .mul_add(diagonal.0, pointer_vector.1 * diagonal.1)
            / denominator;
        let canvas_factor = self.max_canvas_factor();
        let min_factor = f64::from(OVERLAY_SCALE_MIN) / f64::from(self.original.scale.milli);
        let max_factor = (f64::from(OVERLAY_SCALE_MAX) / f64::from(self.original.scale.milli))
            .min(canvas_factor);
        let factor = projected.clamp(min_factor.min(max_factor), max_factor);
        #[allow(clippy::cast_possible_truncation)]
        let requested = (f64::from(self.original.scale.milli) * factor).round() as i64;
        let scale = NormalizedScale::clamped(requested);
        let applied_factor = f64::from(scale.milli) / f64::from(self.original.scale.milli);
        let width = self.original.rect.width * applied_factor;
        let height = self.original.rect.height * applied_factor;
        let (x, y) = match self.corner {
            ResizeCorner::TopLeft => (self.fixed.x - width, self.fixed.y - height),
            ResizeCorner::TopRight => (self.fixed.x, self.fixed.y - height),
            ResizeCorner::BottomLeft => (self.fixed.x - width, self.fixed.y),
            ResizeCorner::BottomRight => (self.fixed.x, self.fixed.y),
        };
        let rect = CanvasRect::new(
            x.clamp(0.0, (self.canvas.width - width).max(0.0)),
            y.clamp(0.0, (self.canvas.height - height).max(0.0)),
            width,
            height,
        );
        self.preview = CanvasResizePreview {
            rect,
            position: normalize_point(CanvasPoint::new(rect.x, rect.y), self.canvas),
            scale,
        };
        self.moved |= (pointer.x - self.start_pointer.x).abs() >= CANVAS_DRAG_THRESHOLD_PX
            || (pointer.y - self.start_pointer.y).abs() >= CANVAS_DRAG_THRESHOLD_PX;
        Ok(self.preview)
    }

    fn max_canvas_factor(&self) -> f64 {
        let available_width = match self.corner {
            ResizeCorner::TopLeft | ResizeCorner::BottomLeft => self.fixed.x,
            ResizeCorner::TopRight | ResizeCorner::BottomRight => self.canvas.width - self.fixed.x,
        };
        let available_height = match self.corner {
            ResizeCorner::TopLeft | ResizeCorner::TopRight => self.fixed.y,
            ResizeCorner::BottomLeft | ResizeCorner::BottomRight => {
                self.canvas.height - self.fixed.y
            }
        };
        (available_width / self.original.rect.width)
            .min(available_height / self.original.rect.height)
            .max(0.0)
    }

    #[must_use]
    pub fn locus_id(&self) -> &LocusId {
        &self.locus_id
    }

    #[must_use]
    pub fn preview(&self) -> CanvasResizePreview {
        self.preview
    }

    #[must_use]
    pub fn cancel(self) -> CanvasResizePreview {
        self.original
    }

    #[must_use]
    pub fn commit(self) -> Option<CanvasResizePatch> {
        (self.moved && self.preview != self.original).then_some(CanvasResizePatch {
            locus_id: self.locus_id,
            before: self.original,
            after: self.preview,
        })
    }
}

fn validate_canvas(canvas: CanvasSize) -> Result<(), CanvasDragError> {
    if canvas.width.is_finite()
        && canvas.height.is_finite()
        && canvas.width > 0.0
        && canvas.height > 0.0
    {
        Ok(())
    } else {
        Err(CanvasDragError::InvalidCanvas)
    }
}

fn validate_rect(rect: CanvasRect) -> Result<(), CanvasDragError> {
    if rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && rect.width >= 0.0
        && rect.height >= 0.0
    {
        Ok(())
    } else {
        Err(CanvasDragError::InvalidOverlay)
    }
}

fn validate_pointer(pointer: CanvasPoint) -> Result<(), CanvasDragError> {
    if pointer.x.is_finite() && pointer.y.is_finite() {
        Ok(())
    } else {
        Err(CanvasDragError::InvalidPointer)
    }
}

#[allow(clippy::cast_possible_truncation)]
fn normalize_axis(value: f64, extent: f64) -> i64 {
    (value / extent * 10_000.0).round() as i64
}

fn normalize_point(point: CanvasPoint, canvas: CanvasSize) -> NormalizedPosition {
    NormalizedPosition::clamped(
        normalize_axis(point.x, canvas.width),
        normalize_axis(point.y, canvas.height),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_emits_one_locus_backed_normalized_patch() {
        let locus = LocusId::new("scene:title:1");
        let mut drag = CanvasDrag::begin(
            locus.clone(),
            CanvasRect::new(20.0, 20.0, 40.0, 30.0),
            CanvasSize::new(200.0, 100.0),
            CanvasPoint::new(30.0, 25.0),
        )
        .expect("begin");
        assert_eq!(
            drag.preview_position(),
            NormalizedPosition::new(1_000, 2_000).unwrap()
        );
        drag.update(CanvasPoint::new(400.0, 200.0)).expect("update");
        let patch = drag.commit().expect("moved patch");
        assert_eq!(patch.locus_id, locus);
        // Overlay is 40x30, so its top-left clamps at 160x70.
        assert_eq!(patch.after, NormalizedPosition::new(8_000, 7_000).unwrap());
    }

    #[test]
    fn cancel_and_click_do_not_create_history_patch() {
        let drag = CanvasDrag::begin(
            LocusId::new("title"),
            CanvasRect::new(0.0, 70.0, 100.0, 30.0),
            CanvasSize::new(100.0, 100.0),
            CanvasPoint::new(50.0, 80.0),
        )
        .unwrap();
        assert_eq!(
            drag.clone().cancel(),
            NormalizedPosition::new(0, 7_000).unwrap()
        );
        assert!(drag.commit().is_none());
    }

    #[test]
    fn rejects_non_finite_and_outside_begin() {
        let result = CanvasDrag::begin(
            LocusId::new("title"),
            CanvasRect::new(10.0, 10.0, 20.0, 20.0),
            CanvasSize::new(100.0, 100.0),
            CanvasPoint::new(0.0, 0.0),
        );
        assert_eq!(result.unwrap_err(), CanvasDragError::PointerOutsideOverlay);
    }

    #[test]
    fn four_corners_keep_opposite_anchor_and_aspect_ratio() {
        let canvas = CanvasSize::new(400.0, 240.0);
        let original = CanvasRect::new(80.0, 60.0, 160.0, 80.0);
        let cases = [
            (ResizeCorner::TopLeft, CanvasPoint::new(40.0, 40.0)),
            (ResizeCorner::TopRight, CanvasPoint::new(280.0, 40.0)),
            (ResizeCorner::BottomLeft, CanvasPoint::new(40.0, 160.0)),
            (ResizeCorner::BottomRight, CanvasPoint::new(280.0, 160.0)),
        ];
        for (corner, pointer) in cases {
            let fixed = corner.fixed_corner(original);
            let mut resize = CanvasResize::begin(
                LocusId::new("title"),
                corner,
                original,
                canvas,
                corner.fixed_corner(original),
                NormalizedScale::ONE,
            )
            .unwrap();
            let preview = resize.update(pointer).unwrap();
            assert!((preview.rect.width / preview.rect.height - 2.0).abs() < 0.001);
            let opposite = corner.fixed_corner(preview.rect);
            assert!((opposite.x - fixed.x).abs() < 0.001);
            assert!((opposite.y - fixed.y).abs() < 0.001);
        }
    }

    #[test]
    fn resize_clamps_to_canvas_and_scale_limits() {
        let mut resize = CanvasResize::begin(
            LocusId::new("callout"),
            ResizeCorner::BottomRight,
            CanvasRect::new(20.0, 20.0, 100.0, 50.0),
            CanvasSize::new(180.0, 100.0),
            CanvasPoint::new(120.0, 70.0),
            NormalizedScale::ONE,
        )
        .unwrap();
        let minimum = resize.update(CanvasPoint::new(20.0, 20.0)).unwrap();
        assert_eq!(minimum.scale.milli, OVERLAY_SCALE_MIN);
        let preview = resize.update(CanvasPoint::new(1_000.0, 1_000.0)).unwrap();
        assert!(preview.rect.x + preview.rect.width <= 180.001);
        assert!(preview.rect.y + preview.rect.height <= 100.001);
        assert_eq!(preview.scale.milli, 1_600);
        assert!(resize.commit().is_some());

        let mut roomy = CanvasResize::begin(
            LocusId::new("title"),
            ResizeCorner::BottomRight,
            CanvasRect::new(0.0, 0.0, 40.0, 20.0),
            CanvasSize::new(400.0, 240.0),
            CanvasPoint::new(40.0, 20.0),
            NormalizedScale::ONE,
        )
        .unwrap();
        assert_eq!(
            roomy
                .update(CanvasPoint::new(1_000.0, 1_000.0))
                .unwrap()
                .scale
                .milli,
            OVERLAY_SCALE_MAX
        );
    }
}

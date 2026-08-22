//! Explicit timeline gesture lifecycle. No GPUI types.

use lattice_engine::{Time, TimeSpan};

use crate::viewport::{TimelineViewport, time_as_secs, time_from_secs};

pub const TRIM_HANDLE_PX: f64 = 8.0;
pub const DRAG_THRESHOLD_PX: f64 = 4.0;
pub const SNAP_THRESHOLD_PX: f64 = 8.0;

#[must_use]
pub fn min_duration() -> Time {
    Time::milliseconds(1)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorKind {
    Select,
    Scrub,
    Trim,
    Grab,
    Grabbing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipKind {
    Video,
    Title,
    Callout,
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TimelineHit {
    Trim {
        clip_id: String,
        edge: Edge,
        kind: ClipKind,
    },
    ClipBody {
        clip_id: String,
        kind: ClipKind,
    },
    Rail,
}

/// One in-flight pointer gesture. `None` is idle.
#[derive(Clone, Debug, PartialEq)]
pub enum TimelineGesture {
    None,
    Scrub {
        start_playhead: Time,
        start_x: f64,
    },
    Trim {
        clip_id: String,
        edge: Edge,
        original_in: Time,
        original_out: Time,
        preview_in: Time,
        preview_out: Time,
        timeline_start: Time,
        start_x: f64,
        scene_id: String,
    },
    Reorder {
        clip_id: String,
        scene_id: String,
        original_index: usize,
        proposed_index: usize,
        start_x: f64,
        moved: bool,
    },
    MoveOverlay {
        clip_id: String,
        kind: ClipKind,
        original: TimeSpan,
        preview: TimeSpan,
        grab_offset: Time,
        start_x: f64,
        moved: bool,
        scene_offset: Time,
    },
    ResizeOverlay {
        clip_id: String,
        kind: ClipKind,
        edge: Edge,
        original: TimeSpan,
        preview: TimeSpan,
        start_x: f64,
        scene_offset: Time,
    },
}

impl TimelineGesture {
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.is_none()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureOutcome {
    Idle,
    Scrubbed,
    Clicked,
    Applied,
    Cancelled,
    Failed,
}

/// Layout-facing clip used for hit-testing (GPUI-free).
#[derive(Clone, Debug, PartialEq)]
pub struct HitClip {
    pub id: String,
    pub kind: ClipKind,
    pub start: Time,
    pub duration: Time,
    pub selected: bool,
    pub scene_id: String,
}

#[must_use]
pub fn clip_kind_from_track(kind: &str, track: &str) -> ClipKind {
    match kind {
        _ if track == "video" => ClipKind::Video,
        "title" => ClipKind::Title,
        "callout" => ClipKind::Callout,
        _ => ClipKind::Other,
    }
}

#[must_use]
pub fn hit_test(clips: &[HitClip], x: f64, viewport: TimelineViewport) -> TimelineHit {
    let mut best_trim: Option<(f64, TimelineHit)> = None;
    let mut body: Option<(bool, TimelineHit)> = None;
    for clip in clips {
        let left = viewport.x_at_time(clip.start);
        let right = viewport.x_at_time(clip.end());
        let width = (right - left).abs();
        if width <= 0.0 {
            continue;
        }
        let lo = left.min(right);
        let hi = left.max(right);
        let handle = TRIM_HANDLE_PX.min(width / 2.0).max(1.0);
        let on_clip = x >= lo - handle && x <= hi + handle;
        if !on_clip {
            continue;
        }
        let handleable = clip.selected
            && matches!(
                clip.kind,
                ClipKind::Video | ClipKind::Title | ClipKind::Callout
            );
        if handleable {
            let dist_left = (x - lo).abs();
            let dist_right = (x - hi).abs();
            if dist_left <= handle && dist_left <= dist_right {
                let dist = dist_left;
                let better = best_trim.as_ref().is_none_or(|(d, _)| dist <= *d);
                if better {
                    best_trim = Some((
                        dist,
                        TimelineHit::Trim {
                            clip_id: clip.id.clone(),
                            edge: Edge::Left,
                            kind: clip.kind,
                        },
                    ));
                }
            } else if dist_right <= handle {
                let dist = dist_right;
                let better = best_trim.as_ref().is_none_or(|(d, _)| dist <= *d);
                if better {
                    best_trim = Some((
                        dist,
                        TimelineHit::Trim {
                            clip_id: clip.id.clone(),
                            edge: Edge::Right,
                            kind: clip.kind,
                        },
                    ));
                }
            }
        }
        if x >= lo && x <= hi {
            let hit = TimelineHit::ClipBody {
                clip_id: clip.id.clone(),
                kind: clip.kind,
            };
            let take = match &body {
                None => true,
                Some((was_selected, _)) => clip.selected || !was_selected,
            };
            if take {
                body = Some((clip.selected, hit));
            }
        }
    }
    if let Some((_, hit)) = best_trim {
        return hit;
    }
    body.map_or(TimelineHit::Rail, |(_, hit)| hit)
}

impl HitClip {
    #[must_use]
    pub fn end(&self) -> Time {
        self.start.checked_add(self.duration).unwrap_or(self.start)
    }
}

#[must_use]
pub fn cursor_for_hit(hit: &TimelineHit, dragging: bool) -> CursorKind {
    if dragging {
        return match hit {
            TimelineHit::Trim { .. } => CursorKind::Trim,
            TimelineHit::ClipBody { .. } => CursorKind::Grabbing,
            TimelineHit::Rail => CursorKind::Scrub,
        };
    }
    match hit {
        TimelineHit::Trim { .. } => CursorKind::Trim,
        TimelineHit::ClipBody {
            kind: ClipKind::Video | ClipKind::Title | ClipKind::Callout,
            ..
        } => CursorKind::Grab,
        TimelineHit::ClipBody { .. } | TimelineHit::Rail => CursorKind::Scrub,
    }
}

#[must_use]
pub fn crossed_drag_threshold(start_x: f64, x: f64) -> bool {
    (x - start_x).abs() >= DRAG_THRESHOLD_PX
}

/// Left trim: later in → shorter. Right trim: earlier out → shorter.
/// Stays inside `[original_in, original_out]` with a positive duration.
#[must_use]
pub fn preview_trim(
    original_in: Time,
    original_out: Time,
    edge: Edge,
    delta: Time,
) -> (Time, Time) {
    let min_end = original_in
        .checked_add(min_duration())
        .unwrap_or(original_out);
    let max_start = original_out
        .checked_sub(min_duration())
        .unwrap_or(original_in);
    match edge {
        Edge::Left => {
            let mut new_in = original_in.checked_add(delta).unwrap_or(original_in);
            if new_in < original_in {
                new_in = original_in;
            }
            if new_in > max_start {
                new_in = max_start;
            }
            if new_in >= original_out {
                new_in = max_start;
            }
            (new_in, original_out)
        }
        Edge::Right => {
            let mut new_out = original_out.checked_add(delta).unwrap_or(original_out);
            if new_out > original_out {
                new_out = original_out;
            }
            if new_out < min_end {
                new_out = min_end;
            }
            if new_out <= original_in {
                new_out = min_end;
            }
            (original_in, new_out)
        }
    }
}

#[must_use]
pub fn preview_overlay_move(original: TimeSpan, new_start: Time) -> TimeSpan {
    let start = if new_start < Time::ZERO {
        Time::ZERO
    } else {
        new_start
    };
    TimeSpan::new(start, original.duration)
}

#[must_use]
pub fn preview_overlay_resize(original: TimeSpan, edge: Edge, delta: Time) -> TimeSpan {
    match edge {
        Edge::Left => {
            let mut start = original.start.checked_add(delta).unwrap_or(original.start);
            if start < Time::ZERO {
                start = Time::ZERO;
            }
            let end = original.end();
            let mut duration = end.checked_sub(start).unwrap_or_else(|_| min_duration());
            if duration < min_duration() {
                duration = min_duration();
                start = end.checked_sub(duration).unwrap_or(Time::ZERO);
                if start < Time::ZERO {
                    start = Time::ZERO;
                    duration = end;
                }
            }
            TimeSpan::new(start, duration)
        }
        Edge::Right => {
            let mut duration = original
                .duration
                .checked_add(delta)
                .unwrap_or(original.duration);
            if duration < min_duration() {
                duration = min_duration();
            }
            TimeSpan::new(original.start, duration)
        }
    }
}

#[must_use]
pub fn overlay_duration_valid(span: TimeSpan) -> bool {
    span.duration > Time::ZERO
}

/// Snap `raw` to the nearest candidate within `threshold_px` at this viewport.
/// Returns `(snapped, target)` when a snap fires.
#[must_use]
pub fn snap_time(
    raw: Time,
    candidates: &[Time],
    viewport: TimelineViewport,
    threshold_px: f64,
) -> Option<(Time, Time)> {
    if threshold_px <= 0.0 || candidates.is_empty() {
        return None;
    }
    let x = viewport.x_at_time(raw);
    let mut best: Option<(f64, Time)> = None;
    for &candidate in candidates {
        let dx = (viewport.x_at_time(candidate) - x).abs();
        if dx <= threshold_px {
            let better = best.as_ref().is_none_or(|(d, _)| dx < *d);
            if better {
                best = Some((dx, candidate));
            }
        }
    }
    best.map(|(_, target)| (target, target))
}

/// Nearest frame boundary using probed fps. No assumed 30 fps.
#[must_use]
pub fn nearest_frame(time: Time, fps_num: i64, fps_den: i64) -> Option<Time> {
    if fps_num <= 0 || fps_den <= 0 {
        return None;
    }
    let fps = time_as_secs(Time::new(fps_num, fps_den).ok()?);
    if !fps.is_finite() || fps <= 0.0 {
        return None;
    }
    let secs = time_as_secs(time);
    let frame = (secs * fps).round();
    Some(time_from_secs(frame / fps))
}

/// Insertion index in `order` after dragging `from` so the pointer sits at `at`.
#[must_use]
pub fn reorder_index(starts_and_ends: &[(Time, Time)], from: usize, at: Time) -> usize {
    if starts_and_ends.is_empty() {
        return 0;
    }
    let mut insert = starts_and_ends.len();
    for (i, (start, end)) in starts_and_ends.iter().enumerate() {
        if i == from {
            continue;
        }
        let mid = time_from_secs(f64::midpoint(time_as_secs(*start), time_as_secs(*end)));
        if at < mid {
            insert = i;
            break;
        }
    }
    let n = starts_and_ends.len();
    if from < insert {
        insert.saturating_sub(1).min(n.saturating_sub(1))
    } else {
        insert.min(n.saturating_sub(1))
    }
}

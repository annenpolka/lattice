//! Explicit timeline gesture lifecycle. No GPUI types.

use lattice_engine::{Time, TimeSpan};

use crate::viewport::{TimelineViewport, time_as_secs, time_from_secs};

pub const TRIM_HANDLE_PX: f64 = 8.0;
pub const FADE_WEDGE_PX: f64 = 20.0;
pub const DELETE_HANDLE_PX: f64 = 22.0;
pub const GRAB_HANDLE_PX: f64 = 16.0;
pub const GRAB_HANDLE_Y: f64 = 8.0;
pub const CUT_LANE_Y_RATIO: f64 = 0.45;
pub const MIN_DRAW_WIDTH_PX: f64 = 24.0;
pub const TRACK_HEIGHT_PX: f64 = 22.0;
pub const DRAG_THRESHOLD_PX: f64 = 4.0;
pub const SNAP_THRESHOLD_PX: f64 = 8.0;
pub const GAIN_DB_TOP: i32 = 12;
pub const GAIN_DB_BOTTOM: i32 = -24;
pub const GAIN_LINE_HEIGHT_PX: f64 = 4.0;
pub const GAIN_LINE_SLOP_PX: f64 = 1.0;

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
    Adjust,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipKind {
    Video,
    Audio,
    Title,
    Callout,
    Scene,
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TimelineHit {
    Trim {
        clip_id: String,
        edge: Edge,
        kind: ClipKind,
    },
    Fade {
        clip_id: String,
    },
    Gain {
        clip_id: String,
    },
    CutLane {
        clip_id: String,
    },
    DeleteHandle {
        clip_id: String,
    },
    /// Explicit move grip on a selected Video/Audio clip. Not the clip body.
    GrabHandle {
        clip_id: String,
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
    /// Empty-rail coordinate point. Drag past the threshold becomes scrub.
    Point {
        start_x: f64,
    },
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
    Gain {
        clip_id: String,
        original_db: i32,
        preview_db: i32,
        start_y: f64,
        moved: bool,
    },
    Fade {
        clip_id: String,
        original: Time,
        preview: Time,
        clip_start: Time,
        clip_duration: Time,
        start_x: f64,
        moved: bool,
    },
    Split {
        scene_id: String,
        at: Time,
    },
    Delete {
        scene_id: String,
    },
    PointSource {
        clip_id: String,
    },
    PointScene {
        scene_id: String,
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
    pub gain_db: i32,
}

#[must_use]
pub fn clip_kind_from_track(kind: &str, track: &str) -> ClipKind {
    match (kind, track) {
        (_, "scene") => ClipKind::Scene,
        (_, "video") => ClipKind::Video,
        ("audio", _) | (_, "audio") => ClipKind::Audio,
        ("title", _) => ClipKind::Title,
        ("callout", _) => ClipKind::Callout,
        _ => ClipKind::Other,
    }
}

#[must_use]
pub fn db_from_y_ratio(ratio: f64) -> i32 {
    let span = f64::from(GAIN_DB_TOP - GAIN_DB_BOTTOM);
    let db = f64::from(GAIN_DB_TOP) - ratio.clamp(0.0, 1.0) * span;
    let rounded = db.round();
    if rounded >= f64::from(i32::MAX) {
        i32::MAX
    } else if rounded <= f64::from(i32::MIN) {
        i32::MIN
    } else {
        #[allow(clippy::cast_possible_truncation)]
        {
            rounded as i32
        }
    }
}

#[must_use]
pub fn gain_line_top(db: i32) -> f64 {
    (TRACK_HEIGHT_PX * y_ratio_from_db(db)).clamp(2.0, 18.0)
}

#[must_use]
pub fn on_gain_line(y: f64, db: i32) -> bool {
    let top = gain_line_top(db);
    y >= top - GAIN_LINE_SLOP_PX && y <= top + GAIN_LINE_HEIGHT_PX + GAIN_LINE_SLOP_PX
}

#[must_use]
pub fn y_ratio_from_db(db: i32) -> f64 {
    let span = f64::from(GAIN_DB_TOP - GAIN_DB_BOTTOM);
    if span <= 0.0 {
        return 0.5;
    }
    ((f64::from(GAIN_DB_TOP - db)) / span).clamp(0.0, 1.0)
}

#[must_use]
pub fn clip_too_small(width: f64) -> bool {
    width < MIN_DRAW_WIDTH_PX
}

#[must_use]
/// Clips whose body contains `x`. Used for overlap on one track.
pub fn clips_at_x(clips: &[HitClip], x: f64, viewport: TimelineViewport) -> Vec<HitClip> {
    clips
        .iter()
        .filter(|clip| {
            let left = viewport.x_at_time(clip.start);
            let right = viewport.x_at_time(clip.end());
            let lo = left.min(right);
            let hi = left.max(right);
            x >= lo && x <= hi
        })
        .cloned()
        .collect()
}

pub fn hit_test(clips: &[HitClip], x: f64, viewport: TimelineViewport) -> TimelineHit {
    hit_test_xy(clips, x, TRACK_HEIGHT_PX / 2.0, viewport)
}

#[allow(clippy::too_many_lines)]
pub fn hit_test_xy(clips: &[HitClip], x: f64, y: f64, viewport: TimelineViewport) -> TimelineHit {
    let mut best_trim: Option<(f64, u8, TimelineHit)> = None;
    let mut best_fade: Option<TimelineHit> = None;
    let mut best_gain: Option<TimelineHit> = None;
    let mut best_cut: Option<TimelineHit> = None;
    let mut best_delete: Option<TimelineHit> = None;
    let mut best_grab: Option<TimelineHit> = None;
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
        let drawable = !clip_too_small(width);
        if clip.selected && clip.kind == ClipKind::Scene && x >= lo && x <= hi {
            if drawable && x >= hi - DELETE_HANDLE_PX {
                best_delete = Some(TimelineHit::DeleteHandle {
                    clip_id: clip.id.clone(),
                });
            } else if drawable && y <= TRACK_HEIGHT_PX * CUT_LANE_Y_RATIO {
                best_cut = Some(TimelineHit::CutLane {
                    clip_id: clip.id.clone(),
                });
            }
        }
        let handleable = clip.selected
            && drawable
            && matches!(
                clip.kind,
                ClipKind::Video | ClipKind::Title | ClipKind::Callout
            );
        if handleable {
            let dist_left = (x - lo).abs();
            let dist_right = (x - hi).abs();
            if dist_left <= handle && dist_left <= dist_right {
                let dist = dist_left;
                let rank = trim_rank(clip);
                let better = best_trim
                    .as_ref()
                    .is_none_or(|(d, old_rank, _)| dist < *d || (dist <= *d && rank >= *old_rank));
                if better {
                    best_trim = Some((
                        dist,
                        rank,
                        TimelineHit::Trim {
                            clip_id: clip.id.clone(),
                            edge: Edge::Left,
                            kind: clip.kind,
                        },
                    ));
                }
            } else if dist_right <= handle {
                let dist = dist_right;
                let rank = trim_rank(clip);
                let better = best_trim
                    .as_ref()
                    .is_none_or(|(d, old_rank, _)| dist < *d || (dist <= *d && rank >= *old_rank));
                if better {
                    best_trim = Some((
                        dist,
                        rank,
                        TimelineHit::Trim {
                            clip_id: clip.id.clone(),
                            edge: Edge::Right,
                            kind: clip.kind,
                        },
                    ));
                }
            } else if clip.selected
                && clip.kind == ClipKind::Video
                && y <= TRACK_HEIGHT_PX * CUT_LANE_Y_RATIO
                && x >= lo + handle
                && x <= lo + handle + FADE_WEDGE_PX.min(width / 3.0).max(handle)
            {
                best_fade = Some(TimelineHit::Fade {
                    clip_id: clip.id.clone(),
                });
            }
        }
        if clip.selected
            && clip.kind == ClipKind::Audio
            && drawable
            && x >= lo
            && x <= hi
            && on_gain_line(y, clip.gain_db)
        {
            best_gain = Some(TimelineHit::Gain {
                clip_id: clip.id.clone(),
            });
        }
        if clip.selected
            && drawable
            && matches!(clip.kind, ClipKind::Video | ClipKind::Audio)
            && y <= GRAB_HANDLE_Y
            && x >= lo
            && x <= hi
        {
            let center = f64::midpoint(lo, hi);
            let grab = GRAB_HANDLE_PX.min(width / 3.0).max(8.0) / 2.0;
            if (x - center).abs() <= grab {
                best_grab = Some(TimelineHit::GrabHandle {
                    clip_id: clip.id.clone(),
                });
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
    if let Some(hit) = best_delete {
        return hit;
    }
    if let Some(hit) = best_cut {
        return hit;
    }
    if let Some((_, _, hit)) = best_trim {
        return hit;
    }
    if let Some(hit) = best_fade {
        return hit;
    }
    if let Some(hit) = best_gain {
        return hit;
    }
    if let Some(hit) = best_grab {
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

fn trim_rank(clip: &HitClip) -> u8 {
    let overlay = u8::from(matches!(clip.kind, ClipKind::Title | ClipKind::Callout));
    (overlay << 1) | u8::from(clip.selected)
}

#[must_use]
pub fn cursor_for_hit(hit: &TimelineHit, dragging: bool) -> CursorKind {
    if dragging {
        return match hit {
            TimelineHit::Trim { .. } => CursorKind::Trim,
            TimelineHit::Fade { .. } | TimelineHit::Gain { .. } | TimelineHit::CutLane { .. } => {
                CursorKind::Adjust
            }
            TimelineHit::ClipBody { .. } | TimelineHit::GrabHandle { .. } => CursorKind::Grabbing,
            TimelineHit::DeleteHandle { .. } | TimelineHit::Rail => CursorKind::Scrub,
        };
    }
    match hit {
        TimelineHit::Trim { .. } => CursorKind::Trim,
        TimelineHit::Fade { .. } | TimelineHit::Gain { .. } | TimelineHit::CutLane { .. } => {
            CursorKind::Adjust
        }
        TimelineHit::DeleteHandle { .. } => CursorKind::Select,
        TimelineHit::GrabHandle { .. }
        | TimelineHit::ClipBody {
            kind: ClipKind::Video | ClipKind::Title | ClipKind::Callout | ClipKind::Scene,
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

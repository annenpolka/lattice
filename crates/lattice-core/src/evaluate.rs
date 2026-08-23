//! Evaluate a flattened timeline at time `t` into a `RenderScene` snapshot + `AudioPlan`.
#![allow(clippy::cast_possible_truncation)]

use crate::NormalizedScale;
use crate::ir::{PlacementKind, TimeSpan};
use crate::locator::MediaLocator;
use crate::overlay::{OverlayAlign, OverlayBar, OverlaySize, OverlayStyle};
use crate::property::{Curve, Easing, Keyframe, Property};
use crate::scene::{
    AnimatedStyle, AssetRef, AudioClip, AudioPlan, BlendMode, Canvas, FontIdentity, FontSpec,
    GroupNode, NodeProps, Rect, RenderNode, RenderScene, Rgba, ShapeKind, ShapeNode, TextNode,
    Transform, VideoNode,
};
use crate::time::Time;
use crate::timeline::{Timeline, TimelineClip, TimelineError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EvaluateError {
    #[error(transparent)]
    Timeline(#[from] TimelineError),
    #[error("time {0} is outside the timeline")]
    TimeOutOfRange(Time),
    #[error("time map: {0}")]
    TimeMap(String),
}

/// Options for [`evaluate`]. Font identity is stamped onto `Text` nodes.
#[derive(Clone, Copy, Debug, Default)]
pub struct EvaluateOpts<'a> {
    pub style: Option<&'a AnimatedStyle>,
    pub font: Option<&'a FontIdentity>,
}

/// Shared preview/export entry: same resolved timeline + same `t` => same scene.
pub fn evaluate_at(
    timeline: &Timeline,
    time: Time,
    canvas: Canvas,
) -> Result<RenderScene, EvaluateError> {
    evaluate(timeline, time, canvas, EvaluateOpts::default())
}

/// Like [`evaluate_at`], with optional extra animated style applied to every node.
pub fn evaluate_with_style(
    timeline: &Timeline,
    time: Time,
    canvas: Canvas,
    style: Option<&AnimatedStyle>,
) -> Result<RenderScene, EvaluateError> {
    evaluate(timeline, time, canvas, EvaluateOpts { style, font: None })
}

pub fn evaluate(
    timeline: &Timeline,
    time: Time,
    canvas: Canvas,
    opts: EvaluateOpts<'_>,
) -> Result<RenderScene, EvaluateError> {
    if time < Time::ZERO || time > timeline.duration {
        return Err(EvaluateError::TimeOutOfRange(time));
    }
    let mut nodes = Vec::new();
    for clip in &timeline.clips {
        if !clip_covers(timeline, clip, time) {
            continue;
        }
        let local = time.checked_sub(clip.span.start).unwrap_or(Time::ZERO);
        match clip.kind {
            PlacementKind::Video => {
                if let Some(node) = video_node(clip, local, canvas, opts.style)? {
                    nodes.push(node);
                }
            }
            PlacementKind::Title => {
                nodes.push(overlay_group(
                    clip, local, canvas, false, opts.style, opts.font,
                ));
            }
            PlacementKind::Callout => {
                nodes.push(overlay_group(
                    clip, local, canvas, true, opts.style, opts.font,
                ));
            }
            PlacementKind::Audio => {}
        }
    }
    Ok(RenderScene { canvas, nodes })
}

/// Shared title/callout extent used by evaluate and Studio selection chrome.
///
/// The current `3/4` canvas width is **implementation shaping**, not a frozen
/// product API. CHI-90 `align` must work inside whatever `Rect` evaluate
/// assigns to `TextNode.bounds` — do not encode 75% as a product spec.
///
/// It intentionally leaves horizontal travel so a normalized x position is
/// visible; Studio selection chrome uses this same Engine-exported geometry.
#[must_use]
pub fn text_overlay_size(canvas: Canvas) -> (u32, u32) {
    let width = canvas
        .width
        .saturating_mul(3)
        .checked_div(4)
        .unwrap_or(0)
        .max(1);
    let height = 8u32.saturating_add((canvas.height / 6).max(24));
    (width, height)
}

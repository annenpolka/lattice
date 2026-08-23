//! Evaluate a flattened timeline at time `t` into a `RenderScene` snapshot + `AudioPlan`.
#![allow(clippy::cast_possible_truncation)]

use crate::NormalizedScale;
use crate::ir::{PlacementKind, TimeSpan};
use crate::locator::MediaLocator;
use crate::overlay::{OverlayAlign, OverlayAnchor, OverlayBar, OverlaySize, OverlayStyle};
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
/// product API. CHI-90 `align` and CHI-91 `anchor` must work inside whatever
/// box evaluate assigns — do not encode 75% into the anchor API.
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

pub fn audio_plan_from_timeline(timeline: &Timeline) -> AudioPlan {
    let mut windows = Vec::new();
    for clip in timeline.audio_clips() {
        let generated = clip
            .source
            .as_ref()
            .is_some_and(|source| matches!(source.locator, MediaLocator::Generated { .. }));
        if generated {
            windows.push(AudioClip {
                span: clip.span,
                gain_db: clip.gain_db.unwrap_or(0),
                generated: true,
                asset: clip.source.as_ref().map(asset_ref),
                content_start: Time::ZERO,
                hold: false,
            });
            continue;
        }
        let Some(source) = clip.source.as_ref() else {
            windows.push(AudioClip {
                span: clip.span,
                gain_db: clip.gain_db.unwrap_or(0),
                generated: false,
                asset: None,
                content_start: Time::ZERO,
                hold: true,
            });
            continue;
        };
        for segment in &source.time_map.segments {
            windows.push(AudioClip {
                span: TimeSpan::new(
                    clip.span.start + segment.local_start,
                    segment.local_duration,
                ),
                gain_db: clip.gain_db.unwrap_or(0),
                generated: false,
                asset: Some(asset_ref(source)),
                content_start: segment.content_start,
                hold: segment.rate == Time::ZERO,
            });
        }
    }
    AudioPlan {
        duration: timeline.duration,
        windows,
    }
}

fn asset_ref(source: &crate::timeline::TimelineSource) -> AssetRef {
    AssetRef {
        media_name: source.media_name.clone(),
        locator: source.locator.clone(),
    }
}

fn clip_covers(timeline: &Timeline, clip: &TimelineClip, time: Time) -> bool {
    if clip.span.contains(time) {
        return true;
    }
    time == timeline.duration && clip.span.end() == timeline.duration
}

fn fade_style(clip: &TimelineClip) -> AnimatedStyle {
    let base = clip.opacity.unwrap_or(100);
    let mut keys = Vec::new();
    if let Some(fade_in) = clip.fade_in.filter(|t| *t > Time::ZERO) {
        keys.push(Keyframe {
            time: Time::ZERO,
            value: 0,
            easing: Easing::Linear,
        });
        keys.push(Keyframe {
            time: fade_in,
            value: base,
            easing: Easing::Linear,
        });
    } else {
        keys.push(Keyframe {
            time: Time::ZERO,
            value: base,
            easing: Easing::Linear,
        });
    }
    if let Some(fade_out) = clip.fade_out.filter(|t| *t > Time::ZERO) {
        let start = clip
            .span
            .duration
            .checked_sub(fade_out)
            .unwrap_or(Time::ZERO);
        keys.push(Keyframe {
            time: start,
            value: base,
            easing: Easing::Linear,
        });
        keys.push(Keyframe {
            time: clip.span.duration,
            value: 0,
            easing: Easing::Linear,
        });
    }
    AnimatedStyle {
        opacity: if keys.len() == 1 {
            Property::Static(base)
        } else {
            Property::Animated(Curve { keyframes: keys })
        },
        translate_x: Property::Static(0),
        translate_y: Property::Static(0),
        scale_x: Property::Static(1000),
        scale_y: Property::Static(1000),
        rotation_mdeg: Property::Static(0),
    }
}

fn compose_style(
    clip: &TimelineClip,
    extra: Option<&AnimatedStyle>,
    local: Time,
) -> (Transform, u8) {
    let base = fade_style(clip);
    let (mut transform, mut opacity) = base.snapshot(local);
    if let Some(extra) = extra {
        let (over, extra_opacity) = extra.snapshot(local);
        transform.translate_x += over.translate_x;
        transform.translate_y += over.translate_y;
        transform.scale_x = mul_milli(transform.scale_x, over.scale_x);
        transform.scale_y = mul_milli(transform.scale_y, over.scale_y);
        transform.rotation_mdeg += over.rotation_mdeg;
        opacity =
            u8::try_from(u32::from(opacity) * u32::from(extra_opacity) / 100).unwrap_or(opacity);
    }
    (transform, opacity)
}

fn mul_milli(a: i32, b: i32) -> i32 {
    i32::try_from(i64::from(a) * i64::from(b) / 1000).unwrap_or(a)
}

fn video_node(
    clip: &TimelineClip,
    local: Time,
    canvas: Canvas,
    extra: Option<&AnimatedStyle>,
) -> Result<Option<RenderNode>, EvaluateError> {
    let Some(source) = clip.source.as_ref() else {
        return Ok(None);
    };
    let content_time = source
        .time_map
        .content_at(local)
        .map_err(|err| EvaluateError::TimeMap(err.to_string()))?;
    let hold = source.time_map.segments.iter().any(|segment| {
        local >= segment.local_start
            && local < segment.local_start + segment.local_duration
            && segment.rate == Time::ZERO
    });
    let (transform, opacity) = compose_style(clip, extra, local);
    Ok(Some(RenderNode::Video(VideoNode {
        props: NodeProps {
            transform,
            opacity,
            clip: None,
            z: 0,
            blend: BlendMode::SrcOver,
        },
        bounds: Rect::from_canvas(canvas),
        asset: asset_ref(source),
        content_time,
        hold,
        time_map: source.time_map.clone(),
    })))
}

fn overlay_group(
    clip: &TimelineClip,
    local: Time,
    canvas: Canvas,
    callout: bool,
    extra: Option<&AnimatedStyle>,
    font: Option<&FontIdentity>,
) -> RenderNode {
    let (transform, opacity) = compose_style(clip, extra, local);
    let bar_h = 8u32;
    let text_h = (canvas.height / 6).max(24);
    let (overlay_width, overlay_height) = text_overlay_size(canvas);
    let (bar_bounds, bounds) = if callout {
        (
            Rect {
                x: 0,
                y: 0,
                width: overlay_width,
                height: bar_h,
            },
            Rect {
                x: 0,
                y: i32::try_from(bar_h).unwrap_or(0),
                width: overlay_width,
                height: text_h,
            },
        )
    } else {
        let bar_y = i32::try_from(canvas.height.saturating_sub(bar_h)).unwrap_or(0);
        let text_y = i32::try_from(canvas.height.saturating_sub(bar_h + text_h)).unwrap_or(0);
        (
            Rect {
                x: 0,
                y: bar_y,
                width: overlay_width,
                height: bar_h,
            },
            Rect {
                x: 0,
                y: text_y,
                width: overlay_width,
                height: text_h,
            },
        )
    };
    let (transform, bar_transform, text_transform) = overlay_transforms(
        clip,
        canvas,
        callout,
        overlay_width,
        overlay_height,
        bar_bounds,
        transform,
    );
    let style = clip.style.as_ref();
    let fill = overlay_bar_fill(style, callout);
    let z = if callout { 20 } else { 10 };
    let text = clip.text.clone().unwrap_or_default();
    let mut children = Vec::new();
    if let Some(fill) = fill {
        children.push(RenderNode::Shape(ShapeNode {
            props: NodeProps {
                transform: bar_transform,
                opacity,
                clip: None,
                z: 0,
                blend: BlendMode::SrcOver,
            },
            bounds: bar_bounds,
            kind: ShapeKind::Rectangle,
            fill,
        }));
    }
    if !text.is_empty() {
        children.push(RenderNode::Text(TextNode {
            props: NodeProps {
                transform: text_transform,
                opacity,
                clip: None,
                z: 1,
                blend: BlendMode::SrcOver,
            },
            bounds,
            text,
            font: overlay_font_spec(style, canvas, callout),
            resolved_font: font.cloned(),
            color: overlay_text_color(style),
            align: style.and_then(|s| s.align).unwrap_or(OverlayAlign::Left),
        }));
    }
    RenderNode::Group(GroupNode {
        props: NodeProps {
            transform,
            opacity: 100,
            clip: None,
            z,
            blend: BlendMode::SrcOver,
        },
        children,
    })
}

#[allow(clippy::too_many_arguments)]
fn overlay_transforms(
    clip: &TimelineClip,
    canvas: Canvas,
    callout: bool,
    overlay_width: u32,
    overlay_height: u32,
    bar_bounds: Rect,
    mut transform: Transform,
) -> (Transform, Transform, Transform) {
    let scale = clip.scale.unwrap_or_default().fit_within(
        overlay_width,
        overlay_height,
        canvas.width,
        canvas.height,
    );
    let scaled_width = scale.scaled_extent(overlay_width);
    let scaled_height = scale.scaled_extent(overlay_height);
    let base_y = if callout {
        0
    } else {
        i32::try_from(canvas.height.saturating_sub(overlay_height)).unwrap_or(0)
    };
    // `anchor` owns the scale pivot. None / top-left keeps today's math:
    // pin the scaled box's top-left via pixel_origin and scale about (0, base_y).
    // Named non-top-left points pin that point: place the *unscaled* box with
    // the same top-left mapping, then scale about the named pixel so the
    // attachment does not drift when scale changes. Visual.position is still
    // a top-left Canvas % — not a center, not px.
    let attachment = clip.anchor.unwrap_or(OverlayAnchor::TopLeft);
    let (place_w, place_h) = if attachment.places_scaled_top_left() {
        (scaled_width, scaled_height)
    } else {
        (overlay_width, overlay_height)
    };
    let target = clip.position.map_or_else(
        || {
            (
                0,
                if callout {
                    0
                } else {
                    i32::try_from(canvas.height.saturating_sub(place_h)).unwrap_or(0)
                },
            )
        },
        |position| position.pixel_origin(canvas.width, canvas.height, place_w, place_h),
    );
    transform.translate_x = transform.translate_x.saturating_add(target.0);
    transform.translate_y = transform
        .translate_y
        .saturating_add(target.1.saturating_sub(base_y));
    let anchor = attachment.scale_pivot(overlay_width, overlay_height, base_y);
    let bar_pivot = (
        i64::from(bar_bounds.x) * 2 + i64::from(bar_bounds.width),
        i64::from(bar_bounds.y) * 2 + i64::from(bar_bounds.height),
    );
    let bar_transform = uniform_scale_about_anchor(scale, anchor, bar_pivot);
    // Text is rasterized into a canvas-sized transparent layer by both CPU and
    // GPU command collectors, so its transform pivot is the canvas center.
    let text_transform = uniform_scale_about_anchor(
        scale,
        anchor,
        (i64::from(canvas.width), i64::from(canvas.height)),
    );
    (transform, bar_transform, text_transform)
}

/// Convention type size used when overlay `size` is omitted.
/// Title: `canvas.height / 16` (min 14). Callout: `/ 20` (min 12).
fn convention_font_size(canvas: Canvas, callout: bool) -> u32 {
    if callout {
        (canvas.height / 20).max(12)
    } else {
        (canvas.height / 16).max(14)
    }
}

/// Priority: explicit `size` > convention. Percent is a ratio of the convention base.
fn overlay_font_size(style: Option<&OverlayStyle>, canvas: Canvas, callout: bool) -> u32 {
    let base = convention_font_size(canvas, callout);
    match style.and_then(|style| style.size) {
        Some(OverlaySize::Px { px }) => px.max(1),
        Some(OverlaySize::Percent { milli }) => u32::try_from(
            u64::from(base)
                .saturating_mul(u64::from(milli))
                .saturating_div(1_000),
        )
        .unwrap_or(1)
        .max(1),
        None => base,
    }
}

fn overlay_font_spec(style: Option<&OverlayStyle>, canvas: Canvas, callout: bool) -> FontSpec {
    let mut spec = FontSpec::preview_sans(overlay_font_size(style, canvas, callout));
    if let Some(family) = style.and_then(|style| style.family.as_ref()) {
        spec.family.clone_from(family);
    }
    if let Some(weight) = style.and_then(|style| style.weight) {
        spec.weight = weight;
    }
    spec
}

fn overlay_text_color(style: Option<&OverlayStyle>) -> Rgba {
    style.and_then(|style| style.color).unwrap_or(Rgba::WHITE)
}

fn overlay_bar_fill(style: Option<&OverlayStyle>, callout: bool) -> Option<Rgba> {
    match style.and_then(|style| style.bar) {
        Some(OverlayBar::Off) => None,
        Some(OverlayBar::Fill { color }) => Some(color),
        None => Some(if callout { Rgba::CYAN } else { Rgba::YELLOW }),
    }
}

fn uniform_scale_about_anchor(
    scale: NormalizedScale,
    anchor: (i32, i32),
    pivot_twice: (i64, i64),
) -> Transform {
    let delta = i64::from(scale.milli).saturating_sub(1_000);
    let translate = |pivot: i64, anchor: i32| {
        let from_anchor = pivot.saturating_sub(i64::from(anchor) * 2);
        i32::try_from(delta.saturating_mul(from_anchor) / 2_000).unwrap_or_else(|_| {
            if delta.is_negative() {
                i32::MIN
            } else {
                i32::MAX
            }
        })
    };
    Transform {
        translate_x: translate(pivot_twice.0, anchor.0),
        translate_y: translate(pivot_twice.1, anchor.1),
        scale_x: i32::from(scale.milli),
        scale_y: i32::from(scale.milli),
        rotation_mdeg: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::PlacementKind;
    use crate::locator::MediaLocator;
    use crate::time_map::TimeMap;
    use crate::timeline::{Timeline, TimelineClip, TimelineSource};

    fn video_clip() -> TimelineClip {
        TimelineClip {
            id: "v".into(),
            kind: PlacementKind::Video,
            span: TimeSpan::new(Time::ZERO, Time::seconds(10)),
            source: Some(TimelineSource {
                media_name: "game".into(),
                locator: MediaLocator::File {
                    path: "capture.mp4".into(),
                },
                time_map: TimeMap::identity(Time::seconds(10), Time::seconds(10)),
            }),
            text: None,
            opacity: None,
            fade_in: Some(Time::from_decimal_seconds(0, 5, 1).unwrap()),
            fade_out: None,
            position: None,
            scale: None,
            anchor: None,
            style: None,
            gain_db: Some(-3),
        }
    }

    fn title_clip() -> TimelineClip {
        TimelineClip {
            id: "t".into(),
            kind: PlacementKind::Title,
            span: TimeSpan::new(Time::seconds(2), Time::seconds(3)),
            source: None,
            text: Some("Hello".into()),
            opacity: Some(90),
            fade_in: None,
            fade_out: None,
            position: None,
            scale: None,
            anchor: None,
            style: None,
            gain_db: None,
        }
    }

    fn timeline() -> Timeline {
        Timeline {
            duration: Time::seconds(10),
            clips: vec![video_clip(), title_clip()],
        }
    }

    fn font_paths(nodes: &[RenderNode]) -> Vec<String> {
        let mut out = Vec::new();
        for node in nodes {
            match node {
                RenderNode::Text(text) => {
                    if let Some(id) = &text.resolved_font {
                        out.push(id.path.clone());
                    }
                }
                RenderNode::Group(group) => out.extend(font_paths(&group.children)),
                _ => {}
            }
        }
        out
    }

    fn has_text(scene: &RenderScene, expected: &str) -> bool {
        fn walk(nodes: &[RenderNode], expected: &str) -> bool {
            nodes.iter().any(|node| match node {
                RenderNode::Text(text) => text.text == expected,
                RenderNode::Group(group) => walk(&group.children, expected),
                _ => false,
            })
        }
        walk(&scene.nodes, expected)
    }

    fn video_opacity(scene: &RenderScene) -> Option<u8> {
        scene.nodes.iter().find_map(|node| match node {
            RenderNode::Video(video) => Some(video.props.opacity),
            _ => None,
        })
    }

    #[test]
    fn same_t_is_deterministic() {
        let tl = timeline();
        let a = evaluate_at(&tl, Time::seconds(3), Canvas::PREVIEW).unwrap();
        let b = evaluate_at(&tl, Time::seconds(3), Canvas::PREVIEW).unwrap();
        assert_eq!(a, b);
        assert!(has_text(&a, "Hello"));
    }

    #[test]
    fn stamps_resolved_font_on_text_nodes() {
        use crate::resolve::AssetIdentity;
        use crate::scene::FontSource;
        let font = FontIdentity {
            path: "fonts/locked.ttf".into(),
            face_index: 0,
            identity: AssetIdentity::new("abc"),
            source: FontSource::Lock,
        };
        let scene = evaluate(
            &timeline(),
            Time::seconds(3),
            Canvas::PREVIEW,
            EvaluateOpts {
                style: None,
                font: Some(&font),
            },
        )
        .unwrap();
        assert_eq!(
            font_paths(&scene.nodes),
            vec!["fonts/locked.ttf".to_string()]
        );
        assert!(scene.has_text());
    }

    #[test]
    fn title_absent_before_window() {
        let scene = evaluate_at(&timeline(), Time::seconds(1), Canvas::PREVIEW).unwrap();
        assert!(!has_text(&scene, "Hello"));
        assert!(matches!(scene.nodes[0], RenderNode::Video(_)));
    }

    #[test]
    fn normalized_overlay_position_becomes_clamped_group_transform() {
        let mut tl = timeline();
        tl.clips[1].position = Some(crate::NormalizedPosition::ORIGIN);
        let scene = evaluate_at(&tl, Time::seconds(3), Canvas::PREVIEW).unwrap();
        let group = scene.nodes.iter().find_map(|node| match node {
            RenderNode::Group(group) => Some(group),
            _ => None,
        });
        let group = group.expect("title group");
        let (_, overlay_height) = text_overlay_size(Canvas::PREVIEW);
        assert_eq!(group.props.transform.translate_x, 0);
        assert_eq!(
            group.props.transform.translate_y,
            -i32::try_from(Canvas::PREVIEW.height - overlay_height).unwrap()
        );

        tl.clips[1].position = crate::NormalizedPosition::new(10_000, 0);
        let scene = evaluate_at(&tl, Time::seconds(3), Canvas::PREVIEW).unwrap();
        let group = scene.nodes.iter().find_map(|node| match node {
            RenderNode::Group(group) => Some(group),
            _ => None,
        });
        let group = group.expect("moved title group");
        assert_eq!(
            group.props.transform.translate_x,
            i32::try_from(Canvas::PREVIEW.width / 4).unwrap()
        );
    }

    #[test]
    fn normalized_overlay_scale_is_uniform_and_canvas_clamped() {
        let mut tl = timeline();
        tl.clips[1].position = Some(crate::NormalizedPosition::ORIGIN);
        tl.clips[1].scale = crate::NormalizedScale::new(500);
        let scene = evaluate_at(&tl, Time::seconds(3), Canvas::PREVIEW).unwrap();
        let group = scene.nodes.iter().find_map(|node| match node {
            RenderNode::Group(group) => Some(group),
            _ => None,
        });
        let group = group.expect("scaled title group");
        assert_eq!(group.children[0].props().transform.scale_x, 500);
        assert_eq!(group.children[0].props().transform.scale_y, 500);

        tl.clips[1].scale = crate::NormalizedScale::new(2_000);
        let scene = evaluate_at(&tl, Time::seconds(3), Canvas::PREVIEW).unwrap();
        let group = scene.nodes.iter().find_map(|node| match node {
            RenderNode::Group(group) => Some(group),
            _ => None,
        });
        let group = group.expect("canvas-clamped title group");
        assert_eq!(group.children[0].props().transform.scale_x, 1_333);
    }

    fn overlay_group(scene: &RenderScene) -> &GroupNode {
        scene
            .nodes
            .iter()
            .find_map(|node| match node {
                RenderNode::Group(group) => Some(group),
                _ => None,
            })
            .expect("overlay group")
    }

    fn unscaled_box_center(scene: &RenderScene, canvas: Canvas) -> (i32, i32) {
        let group = overlay_group(scene);
        let (ow, oh) = text_overlay_size(canvas);
        let base_y = i32::try_from(canvas.height.saturating_sub(oh)).unwrap();
        let tx = group.props.transform.translate_x;
        let ty = group.props.transform.translate_y;
        (
            tx + i32::try_from(ow).unwrap() / 2,
            base_y + ty + i32::try_from(oh).unwrap() / 2,
        )
    }

    fn scaled_box_center(scene: &RenderScene, canvas: Canvas, milli: u16) -> (i32, i32) {
        let group = overlay_group(scene);
        let (ow, oh) = text_overlay_size(canvas);
        let scale = crate::NormalizedScale { milli };
        let sw = scale.scaled_extent(ow);
        let sh = scale.scaled_extent(oh);
        let base_y = i32::try_from(canvas.height.saturating_sub(oh)).unwrap();
        let tx = group.props.transform.translate_x;
        let ty = group.props.transform.translate_y;
        (
            tx + i32::try_from(sw).unwrap() / 2,
            base_y + ty + i32::try_from(sh).unwrap() / 2,
        )
    }

    #[test]
    fn overlay_anchor_center_owns_scale_pivot() {
        let canvas = Canvas::PREVIEW;
        let mut tl = timeline();
        tl.clips[1].position = crate::NormalizedPosition::new(2_500, 1_000);
        tl.clips[1].anchor = Some(OverlayAnchor::Center);
        tl.clips[1].scale = crate::NormalizedScale::new(1_000);
        let at_100 = evaluate_at(&tl, Time::seconds(3), canvas).unwrap();
        tl.clips[1].scale = crate::NormalizedScale::new(1_500);
        let at_150 = evaluate_at(&tl, Time::seconds(3), canvas).unwrap();
        let c100 = unscaled_box_center(&at_100, canvas);
        let c150 = unscaled_box_center(&at_150, canvas);
        assert!(
            (c100.0 - c150.0).abs() <= 1 && (c100.1 - c150.1).abs() <= 1,
            "center must stay under scale: {c100:?} vs {c150:?}"
        );
        let child_100 = overlay_group(&at_100).children[0].props().transform;
        let child_150 = overlay_group(&at_150).children[0].props().transform;
        assert_eq!(child_100.scale_x, 1_000);
        assert!(
            child_150.scale_x > 1_000,
            "150% must still scale after canvas clamp, got {}",
            child_150.scale_x
        );
    }

    #[test]
    fn omitted_and_explicit_top_left_match_and_drift() {
        let canvas = Canvas::PREVIEW;
        let mut omitted = timeline();
        omitted.clips[1].position = crate::NormalizedPosition::new(2_500, 1_000);
        omitted.clips[1].scale = crate::NormalizedScale::new(1_000);
        let mut explicit = omitted.clone();
        explicit.clips[1].anchor = Some(OverlayAnchor::TopLeft);
        let o100 = evaluate_at(&omitted, Time::seconds(3), canvas).unwrap();
        let e100 = evaluate_at(&explicit, Time::seconds(3), canvas).unwrap();
        assert_eq!(
            overlay_group(&o100).props.transform,
            overlay_group(&e100).props.transform
        );
        omitted.clips[1].scale = crate::NormalizedScale::new(1_500);
        explicit.clips[1].scale = crate::NormalizedScale::new(1_500);
        let o150 = evaluate_at(&omitted, Time::seconds(3), canvas).unwrap();
        let e150 = evaluate_at(&explicit, Time::seconds(3), canvas).unwrap();
        assert_eq!(
            overlay_group(&o150).props.transform,
            overlay_group(&e150).props.transform
        );
        let c100 = scaled_box_center(&o100, canvas, 1_000);
        let c150 = scaled_box_center(&o150, canvas, 1_500);
        assert_ne!(
            c100, c150,
            "top-left scale must still drift the visual center"
        );
    }

    #[test]
    fn empty_outside_clips_still_has_canvas() {
        let tl = Timeline {
            duration: Time::seconds(5),
            clips: vec![],
        };
        let scene = evaluate_at(&tl, Time::seconds(1), Canvas::PREVIEW).unwrap();
        assert!(scene.nodes.is_empty());
        assert_eq!(scene.canvas, Canvas::PREVIEW);
    }

    #[test]
    fn fade_in_endpoints() {
        let tl = timeline();
        let start = evaluate_at(&tl, Time::ZERO, Canvas::PREVIEW).unwrap();
        let mid = evaluate_at(
            &tl,
            Time::from_decimal_seconds(0, 25, 2).unwrap(),
            Canvas::PREVIEW,
        )
        .unwrap();
        let end = evaluate_at(
            &tl,
            Time::from_decimal_seconds(0, 5, 1).unwrap(),
            Canvas::PREVIEW,
        )
        .unwrap();
        assert_eq!(video_opacity(&start), Some(0));
        let mid_op = video_opacity(&mid).unwrap();
        assert!((40..=60).contains(&mid_op), "mid fade opacity {mid_op}");
        assert_eq!(video_opacity(&end), Some(100));
    }

    #[test]
    fn overlapping_z_order_callout_above_title() {
        let mut tl = timeline();
        tl.clips.push(TimelineClip {
            id: "c".into(),
            kind: PlacementKind::Callout,
            span: TimeSpan::new(Time::seconds(2), Time::seconds(3)),
            source: None,
            text: Some("Hold".into()),
            opacity: Some(100),
            fade_in: None,
            fade_out: None,
            position: None,
            scale: None,
            anchor: None,
            style: None,
            gain_db: None,
        });
        let scene = evaluate_at(&tl, Time::seconds(3), Canvas::PREVIEW).unwrap();
        let zs: Vec<i32> = scene.sorted_nodes().iter().map(|n| n.z()).collect();
        assert_eq!(zs, vec![0, 10, 20]);
    }

    #[test]
    fn audio_plan_preserves_gain() {
        let mut tl = timeline();
        tl.clips.push(TimelineClip {
            id: "a".into(),
            kind: PlacementKind::Audio,
            span: TimeSpan::new(Time::ZERO, Time::seconds(10)),
            source: video_clip().source,
            text: None,
            opacity: None,
            fade_in: None,
            fade_out: None,
            position: None,
            scale: None,
            anchor: None,
            style: None,
            gain_db: Some(-3),
        });
        let plan = audio_plan_from_timeline(&tl);
        assert_eq!(plan.windows[0].gain_db, -3);
        assert_eq!(plan.duration, Time::seconds(10));
    }

    fn overlay_text(scene: &RenderScene) -> Option<&TextNode> {
        fn walk(nodes: &[RenderNode]) -> Option<&TextNode> {
            for node in nodes {
                match node {
                    RenderNode::Text(text) => return Some(text),
                    RenderNode::Group(group) => {
                        if let Some(text) = walk(&group.children) {
                            return Some(text);
                        }
                    }
                    _ => {}
                }
            }
            None
        }
        walk(&scene.nodes)
    }

    fn overlay_bars(scene: &RenderScene) -> Vec<Rgba> {
        fn walk(nodes: &[RenderNode], out: &mut Vec<Rgba>) {
            for node in nodes {
                match node {
                    RenderNode::Shape(shape) => out.push(shape.fill),
                    RenderNode::Group(group) => walk(&group.children, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(&scene.nodes, &mut out);
        out
    }

    #[test]
    fn omitted_overlay_style_keeps_title_yellow_and_convention_font() {
        let scene = evaluate_at(&timeline(), Time::seconds(3), Canvas::PREVIEW).unwrap();
        let text = overlay_text(&scene).expect("title text");
        assert_eq!(text.color, Rgba::WHITE);
        assert_eq!(text.font, FontSpec::preview_sans(14));
        assert_eq!(overlay_bars(&scene), vec![Rgba::YELLOW]);
    }

    #[test]
    fn omitted_callout_style_keeps_cyan_bar_and_convention_font() {
        let mut tl = timeline();
        tl.clips.push(TimelineClip {
            id: "c".into(),
            kind: PlacementKind::Callout,
            span: TimeSpan::new(Time::seconds(2), Time::seconds(3)),
            source: None,
            text: Some("Hold".into()),
            opacity: None,
            fade_in: None,
            fade_out: None,
            position: None,
            scale: None,
            anchor: None,
            style: None,
            gain_db: None,
        });
        let scene = evaluate_at(&tl, Time::seconds(3), Canvas::PREVIEW).unwrap();
        let bars = overlay_bars(&scene);
        assert!(bars.contains(&Rgba::YELLOW));
        assert!(bars.contains(&Rgba::CYAN));
        let callout = scene.nodes.iter().find_map(|node| match node {
            RenderNode::Group(group) if group.props.z == 20 => {
                group.children.iter().find_map(|child| match child {
                    RenderNode::Text(text) => Some(text),
                    _ => None,
                })
            }
            _ => None,
        });
        let callout = callout.expect("callout text");
        assert_eq!(callout.font.size_px, 12);
        assert_eq!(callout.color, Rgba::WHITE);
    }

    #[test]
    fn explicit_overlay_style_sets_font_color_and_omits_bar() {
        let mut tl = timeline();
        tl.clips[1].style = Some(OverlayStyle {
            color: Rgba::from_hex_rrggbb("#00FF00"),
            size: Some(OverlaySize::Percent { milli: 500 }),
            weight: Some(700),
            family: Some("LatticeSans".into()),
            bar: Some(OverlayBar::Off),
            align: None,
        });
        let scene = evaluate_at(&tl, Time::seconds(3), Canvas::PREVIEW).unwrap();
        let text = overlay_text(&scene).expect("styled title");
        assert_eq!(text.color, Rgba::from_hex_rrggbb("#00FF00").unwrap());
        assert_eq!(text.font.family, "LatticeSans");
        assert_eq!(text.font.weight, 700);
        assert_eq!(text.font.size_px, 7);
        assert!(overlay_bars(&scene).is_empty());

        tl.clips[1].style = Some(OverlayStyle {
            size: Some(OverlaySize::Px { px: 24 }),
            bar: Some(OverlayBar::Fill {
                color: Rgba::from_hex_rrggbb("#FF00FF").unwrap(),
            }),
            ..OverlayStyle::default()
        });
        let scene = evaluate_at(&tl, Time::seconds(3), Canvas::PREVIEW).unwrap();
        let text = overlay_text(&scene).expect("px lock");
        assert_eq!(text.font.size_px, 24);
        assert_eq!(
            overlay_bars(&scene),
            vec![Rgba::from_hex_rrggbb("#FF00FF").unwrap()]
        );
    }
}

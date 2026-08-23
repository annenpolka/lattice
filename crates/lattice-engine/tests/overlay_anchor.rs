//! CHI-91: overlay `anchor` owns evaluate's scale pivot (never `origin`).
//!
//! `text_overlay_size` 3/4 width is CURRENT shared evaluate/chrome shaping,
//! NOT a frozen product API. Do not encode 75% into the anchor API.

use lattice_core::{
    Canvas, OverlayAnchor, PlacementKind, RenderNode, RenderScene, Time, Visual, evaluate_at,
    text_overlay_size,
};
use lattice_engine::Engine;

fn compile_overlay(word: &str, body: &str) -> lattice_engine::Compilation {
    let source = format!(
        r#"project "overlay-anchor"
media game "capture.mp4"
sequence main {{ intro }}
scene intro {{
  game[0s..4s] as clip
  {word} "Hello" {{
    at 0s for 3s
    {body}
  }}
}}
"#
    );
    Engine::default().compile(&source).unwrap()
}

fn overlay_visual(compilation: &lattice_engine::Compilation) -> Visual {
    compilation.project.scenes[0]
        .placements
        .iter()
        .find(|placement| {
            matches!(
                placement.kind,
                PlacementKind::Title | PlacementKind::Callout
            )
        })
        .and_then(|placement| placement.visual.clone())
        .expect("overlay visual")
}

fn overlay_group(scene: &RenderScene) -> &lattice_core::GroupNode {
    fn walk(nodes: &[RenderNode]) -> Option<&lattice_core::GroupNode> {
        for node in nodes {
            if let RenderNode::Group(group) = node {
                if group
                    .children
                    .iter()
                    .any(|child| matches!(child, RenderNode::Text(_)))
                {
                    return Some(group);
                }
                if let Some(found) = walk(&group.children) {
                    return Some(found);
                }
            }
        }
        None
    }
    walk(&scene.nodes).expect("overlay group")
}

fn evaluated(word: &str, body: &str, canvas: Canvas) -> RenderScene {
    let compilation = compile_overlay(word, body);
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    let timeline = Engine::timeline(&compilation.project).unwrap();
    evaluate_at(&timeline, Time::seconds(0), canvas).unwrap()
}

fn unscaled_center(scene: &RenderScene, canvas: Canvas, callout: bool) -> (i32, i32) {
    let group = overlay_group(scene);
    let (ow, oh) = text_overlay_size(canvas);
    let base_y = if callout {
        0
    } else {
        i32::try_from(canvas.height.saturating_sub(oh)).unwrap()
    };
    let tx = group.props.transform.translate_x;
    let ty = group.props.transform.translate_y;
    (
        tx + i32::try_from(ow).unwrap() / 2,
        base_y + ty + i32::try_from(oh).unwrap() / 2,
    )
}

fn scaled_center(scene: &RenderScene, canvas: Canvas, milli: u16, callout: bool) -> (i32, i32) {
    let group = overlay_group(scene);
    let (ow, oh) = text_overlay_size(canvas);
    let scale = lattice_core::NormalizedScale { milli };
    let sw = scale.scaled_extent(ow);
    let sh = scale.scaled_extent(oh);
    let base_y = if callout {
        0
    } else {
        i32::try_from(canvas.height.saturating_sub(oh)).unwrap()
    };
    let tx = group.props.transform.translate_x;
    let ty = group.props.transform.translate_y;
    (
        tx + i32::try_from(sw).unwrap() / 2,
        base_y + ty + i32::try_from(sh).unwrap() / 2,
    )
}

#[test]
fn overlay_anchor_center_keeps_center_under_scale() {
    let canvas = Canvas::PREVIEW;
    let pos = "position (25%, 10%)";
    let at_100 = evaluated(
        "title",
        &format!("{pos}\n    anchor center\n    scale 100%"),
        canvas,
    );
    let at_150 = evaluated(
        "title",
        &format!("{pos}\n    anchor center\n    scale 150%"),
        canvas,
    );
    let c100 = unscaled_center(&at_100, canvas, false);
    let c150 = unscaled_center(&at_150, canvas, false);
    assert!(
        (c100.0 - c150.0).abs() <= 1 && (c100.1 - c150.1).abs() <= 1,
        "anchor center + scale must not drift the box center: {c100:?} vs {c150:?}"
    );
    assert_eq!(
        overlay_visual(&compile_overlay(
            "title",
            &format!("{pos}\n    anchor center")
        ))
        .anchor,
        Some(OverlayAnchor::Center)
    );
}

#[test]
fn overlay_omitted_anchor_matches_explicit_top_left_and_drifts() {
    let canvas = Canvas::PREVIEW;
    let pos = "position (25%, 10%)";
    let omit_100 = evaluated("title", &format!("{pos}\n    scale 100%"), canvas);
    let left_100 = evaluated(
        "title",
        &format!("{pos}\n    anchor top-left\n    scale 100%"),
        canvas,
    );
    assert_eq!(
        overlay_group(&omit_100).props.transform,
        overlay_group(&left_100).props.transform
    );
    let omit_150 = evaluated("title", &format!("{pos}\n    scale 150%"), canvas);
    let left_150 = evaluated(
        "title",
        &format!("{pos}\n    anchor top-left\n    scale 150%"),
        canvas,
    );
    assert_eq!(
        overlay_group(&omit_150).props.transform,
        overlay_group(&left_150).props.transform
    );
    let c100 = scaled_center(&omit_100, canvas, 1_000, false);
    let c150 = scaled_center(&omit_150, canvas, 1_500, false);
    assert_ne!(
        c100, c150,
        "omitted/top-left must keep today's center drift"
    );
    assert_eq!(overlay_visual(&compile_overlay("title", pos)).anchor, None);
}

#[test]
fn overlay_invalid_anchor_is_lat_ovl_010() {
    let compilation = compile_overlay("title", "anchor top");
    assert!(
        compilation
            .diagnostics
            .iter()
            .any(|diag| diag.code == "LAT-OVL-010" && diag.message.contains("anchor")),
        "{:?}",
        compilation.diagnostics
    );
    assert_eq!(overlay_visual(&compilation).anchor, None);
}

#[test]
fn overlay_origin_stays_unknown() {
    let compilation = compile_overlay("callout", "origin center");
    assert!(
        compilation
            .diagnostics
            .iter()
            .any(|diag| diag.code == "LAT-OVL-003" && diag.message.contains("`origin`")),
        "{:?}",
        compilation.diagnostics
    );
    assert_eq!(overlay_visual(&compilation).anchor, None);
}

#[test]
fn overlay_visual_position_serde_is_still_top_left() {
    let compilation = compile_overlay("title", "position (25%, 10%)\n    anchor center");
    let visual = overlay_visual(&compilation);
    assert_eq!(
        visual.position,
        lattice_core::NormalizedPosition::new(2_500, 1_000)
    );
    let json = serde_json::to_string(&visual).unwrap();
    assert!(json.contains("\"x\":2500"), "{json}");
    assert!(!json.contains("origin"), "{json}");
}

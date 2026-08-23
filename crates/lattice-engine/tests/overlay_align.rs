//! CHI-90: overlay `align` is in-box `TextNode` alignment, not `Visual.position`.

use lattice_core::{
    Canvas, OverlayAlign, PlacementKind, RenderNode, RenderScene, TextNode, Time, Visual,
    evaluate_at,
};
use lattice_engine::Engine;

fn compile_overlay(word: &str, body: &str) -> lattice_engine::Compilation {
    let source = format!(
        r#"project "overlay-align"
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
        .find(|placement| placement.kind == PlacementKind::Title)
        .and_then(|placement| placement.visual.clone())
        .expect("overlay visual")
}

fn overlay_text(scene: &RenderScene) -> &TextNode {
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
    walk(&scene.nodes).expect("overlay text")
}

fn overlay_group_transform(scene: &RenderScene) -> lattice_core::Transform {
    fn walk(nodes: &[RenderNode]) -> Option<lattice_core::Transform> {
        for node in nodes {
            if let RenderNode::Group(group) = node {
                if group
                    .children
                    .iter()
                    .any(|child| matches!(child, RenderNode::Text(_)))
                {
                    return Some(group.props.transform);
                }
                if let Some(transform) = walk(&group.children) {
                    return Some(transform);
                }
            }
        }
        None
    }
    walk(&scene.nodes).expect("overlay group")
}

fn evaluated(body: &str, canvas: Canvas) -> RenderScene {
    let compilation = compile_overlay("title", body);
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    let timeline = Engine::timeline(&compilation.project).unwrap();
    evaluate_at(&timeline, Time::seconds(0), canvas).unwrap()
}

#[test]
fn overlay_omit_defaults_to_left() {
    let compilation = compile_overlay("title", "opacity 90");
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    assert_eq!(overlay_visual(&compilation).style, None);
    let scene = evaluated("opacity 90", Canvas::PREVIEW);
    assert_eq!(overlay_text(&scene).align, OverlayAlign::Left);
}

#[test]
fn overlay_same_position_center_vs_left_changes_text_align_not_group_transform() {
    let left = evaluated("position (25%, 10%)\n    align left", Canvas::PREVIEW);
    let center = evaluated("position (25%, 10%)\n    align center", Canvas::PREVIEW);
    assert_eq!(overlay_text(&left).align, OverlayAlign::Left);
    assert_eq!(overlay_text(&center).align, OverlayAlign::Center);
    assert_eq!(overlay_text(&left).bounds, overlay_text(&center).bounds);
    assert_eq!(
        overlay_group_transform(&left),
        overlay_group_transform(&center),
        "align must not move Visual.position / group transform"
    );
}

#[test]
fn overlay_invalid_align_is_lat_ovl_009() {
    let compilation = compile_overlay("title", "align middle");
    assert!(
        compilation
            .diagnostics
            .iter()
            .any(|diag| { diag.code == "LAT-OVL-009" && diag.message.contains("align") }),
        "{:?}",
        compilation.diagnostics
    );
    assert_eq!(
        overlay_visual(&compilation)
            .style
            .and_then(|style| style.align),
        None,
        "invalid align must not silent-set a style field"
    );
}

#[test]
fn overlay_preview_and_export_share_evaluate_align() {
    let compilation = compile_overlay("title", "align right");
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    let timeline = Engine::timeline(&compilation.project).unwrap();
    let preview = evaluate_at(&timeline, Time::seconds(0), Canvas::PREVIEW).unwrap();
    let export = evaluate_at(
        &timeline,
        Time::seconds(0),
        Canvas {
            width: 1920,
            height: 1080,
        },
    )
    .unwrap();
    assert_eq!(overlay_text(&preview).align, OverlayAlign::Right);
    assert_eq!(overlay_text(&export).align, OverlayAlign::Right);
    assert_eq!(
        overlay_group_transform(&preview).translate_x,
        overlay_group_transform(&export).translate_x
    );
}

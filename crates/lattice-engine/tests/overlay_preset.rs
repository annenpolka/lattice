//! CHI-85: `title using lower-third` is a stdlib preset, not a Core kind.

use lattice_core::{
    Canvas, OverlayBar, OverlaySize, PlacementKind, RenderNode, RenderScene, Rgba, TextNode, Time,
    Visual, evaluate_at,
};
use lattice_engine::Engine;

fn compile_scene(inner: &str) -> lattice_engine::Compilation {
    let source = format!(
        r#"project "overlay-preset"
media game "capture.mp4"
sequence main {{ intro }}
scene intro {{
  game[0s..4s] as clip
  {inner}
}}
"#
    );
    Engine::default().compile(&source).unwrap()
}

fn title_visual(compilation: &lattice_engine::Compilation) -> Visual {
    compilation.project.scenes[0]
        .placements
        .iter()
        .find(|placement| placement.kind == PlacementKind::Title)
        .and_then(|placement| placement.visual.clone())
        .expect("title visual")
}

fn callout_visual(compilation: &lattice_engine::Compilation) -> Visual {
    compilation.project.scenes[0]
        .placements
        .iter()
        .find(|placement| placement.kind == PlacementKind::Callout)
        .and_then(|placement| placement.visual.clone())
        .expect("callout visual")
}

fn has_diag(compilation: &lattice_engine::Compilation, code: &str, needle: &str) -> bool {
    compilation
        .diagnostics
        .iter()
        .any(|diag| diag.code == code && diag.message.contains(needle))
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

fn text_node_count(scene: &RenderScene) -> usize {
    fn walk(nodes: &[RenderNode]) -> usize {
        nodes
            .iter()
            .map(|node| match node {
                RenderNode::Text(_) => 1,
                RenderNode::Group(group) => walk(&group.children),
                _ => 0,
            })
            .sum()
    }
    walk(&scene.nodes)
}

fn visual_dump_has_preset_name(visual: &Visual) -> bool {
    let json = serde_json::to_string(visual).expect("visual json");
    json.contains("lower-third") || json.contains("preset")
}

#[test]
fn title_using_lower_third_expands_to_title_style_without_preset_name() {
    let compilation = compile_scene(
        r#"title "Ada Lovelace\nEditor" using lower-third {
    at 0s for 3s
  }"#,
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    let kind = compilation.project.scenes[0]
        .placements
        .iter()
        .find(|placement| {
            placement.visual.as_ref().and_then(|v| v.text.as_deref())
                == Some("Ada Lovelace\nEditor")
        })
        .map(|placement| placement.kind);
    assert_eq!(kind, Some(PlacementKind::Title));
    let visual = title_visual(&compilation);
    let style = visual.style.expect("preset fills OverlayStyle");
    assert_eq!(
        style.bar,
        Some(OverlayBar::Fill {
            color: Rgba::YELLOW
        })
    );
    assert_eq!(style.size, Some(OverlaySize::Percent { milli: 900 }));
    assert_eq!(style.family.as_deref(), Some("LatticeSans"));
    assert!(style.color.is_none());
    assert!(style.weight.is_none());
    assert!(style.align.is_none());
    assert!(visual.position.is_none());
    assert!(visual.scale.is_none());
    assert!(visual.anchor.is_none());
    assert!(
        !visual_dump_has_preset_name(&visual),
        "Core Visual must not store the preset IDENT: {visual:?}"
    );
    assert!(
        compilation
            .explain
            .iter()
            .any(|event| event.message.contains("using lower-third")),
        "{:?}",
        compilation.explain
    );
}

#[test]
fn bare_title_style_differs_from_lower_third_expansion() {
    let bare = compile_scene(
        r#"title "Ada Lovelace\nEditor" {
    at 0s for 3s
  }"#,
    );
    assert!(!bare.has_errors(), "{:?}", bare.diagnostics);
    assert_eq!(title_visual(&bare).style, None);

    let preset = compile_scene(
        r#"title "Ada Lovelace\nEditor" using lower-third {
    at 0s for 3s
  }"#,
    );
    let preset_style = title_visual(&preset).style.expect("expanded");
    assert_ne!(preset_style, lattice_core::OverlayStyle::default());
}

#[test]
fn lower_third_uses_existing_title_evaluate_pipeline() {
    let compilation = compile_scene(
        r#"title "Ada Lovelace\nEditor" using lower-third {
    at 0s for 3s
  }"#,
    );
    let timeline = Engine::timeline(&compilation.project).unwrap();
    assert_eq!(
        timeline.title_clips().next().unwrap().style,
        title_visual(&compilation).style
    );
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
    assert_eq!(overlay_bars(&preview), vec![Rgba::YELLOW]);
    assert_eq!(overlay_bars(&export), vec![Rgba::YELLOW]);
    assert_eq!(overlay_text(&preview).color, Rgba::WHITE);
    assert_eq!(overlay_text(&export).color, Rgba::WHITE);
    assert_eq!(overlay_text(&preview).font.family, "LatticeSans");
    assert_eq!(overlay_text(&preview).font.weight, 400);
    // 90% of title convention: PREVIEW 180 → max(14, 11) = 14 → 12; 1080 → 67 → 60.
    assert_eq!(overlay_text(&preview).font.size_px, 12);
    assert_eq!(overlay_text(&export).font.size_px, 60);
    assert_eq!(text_node_count(&preview), 1);
    assert_eq!(text_node_count(&export), 1);
}

#[test]
fn explicit_body_style_wins_over_lower_third() {
    let compilation = compile_scene(
        r##"title "Ada Lovelace\nEditor" using lower-third {
    at 0s for 3s
    color "#00FF00"
    size 50%
    bar off
    family "OtherSans"
    weight bold
    align center
    position (10%, 20%)
    scale 80%
    anchor bottom-left
  }"##,
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    let visual = title_visual(&compilation);
    let style = visual.style.expect("style");
    assert_eq!(style.color, Rgba::from_hex_rrggbb("#00FF00"));
    assert_eq!(style.size, Some(OverlaySize::Percent { milli: 500 }));
    assert_eq!(style.bar, Some(OverlayBar::Off));
    assert_eq!(style.family.as_deref(), Some("OtherSans"));
    assert_eq!(style.weight, Some(700));
    assert_eq!(style.align, Some(lattice_core::OverlayAlign::Center));
    assert!(visual.position.is_some());
    assert!(visual.scale.is_some());
    assert_eq!(visual.anchor, Some(lattice_core::OverlayAnchor::BottomLeft));
    let timeline = Engine::timeline(&compilation.project).unwrap();
    let preview = evaluate_at(&timeline, Time::seconds(0), Canvas::PREVIEW).unwrap();
    assert_eq!(
        overlay_text(&preview).color,
        Rgba::from_hex_rrggbb("#00FF00").unwrap()
    );
    assert!(overlay_bars(&preview).is_empty());
}

#[test]
fn unknown_title_preset_is_lowering_diag() {
    let compilation = compile_scene(
        r#"title "Ada Lovelace\nEditor" using upper-third {
    at 0s for 3s
  }"#,
    );
    assert!(
        has_diag(&compilation, "LAT-OVL-013", "`upper-third`"),
        "{:?}",
        compilation.diagnostics
    );
    assert_eq!(title_visual(&compilation).style, None);
    let json = serde_json::to_string(&title_visual(&compilation)).unwrap();
    assert!(!json.contains("upper-third"));
    assert!(!json.contains("lower-third"));
}

#[test]
fn callout_using_lower_third_still_diags() {
    let compilation = compile_scene(
        r#"callout "Hold" using lower-third {
    at 0s for 2s
  }"#,
    );
    assert!(
        has_diag(&compilation, "LAT-OVL-013", "title only"),
        "{:?}",
        compilation.diagnostics
    );
    assert!(
        compilation
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("lower-third")),
        "{:?}",
        compilation.diagnostics
    );
    assert!(
        callout_visual(&compilation)
            .style
            .as_ref()
            .is_none_or(lattice_core::OverlayStyle::is_empty)
    );
}

#[test]
fn caption_using_lower_third_still_diags() {
    let compilation = compile_scene(
        r#"caption "cue" using lower-third {
    at 0s for 2s
  }"#,
    );
    assert!(
        has_diag(&compilation, "LAT-OVL-013", "title only"),
        "{:?}",
        compilation.diagnostics
    );
}

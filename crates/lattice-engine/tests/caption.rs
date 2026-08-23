//! CHI-84: caption is a timed stdlib cue on the existing title overlay.

use lattice_core::{
    Canvas, LocusKind, Origin, OverlayBar, OverlayStyle, PlacementKind, RenderNode, RenderScene,
    Rgba, Time, evaluate_at,
};
use lattice_engine::Engine;

fn compile(source: &str) -> lattice_engine::Compilation {
    Engine::default().compile(source).unwrap()
}

fn caption_project(body: &str) -> String {
    format!(
        r#"project "caption-cue"
media game "capture.mp4"
sequence main {{ intro }}
scene intro {{
  game[0s..8s] as clip
{body}
}}
"#
    )
}

fn overlay_texts(scene: &RenderScene) -> Vec<String> {
    fn walk(nodes: &[RenderNode], out: &mut Vec<String>) {
        for node in nodes {
            match node {
                RenderNode::Text(text) => out.push(text.text.clone()),
                RenderNode::Group(group) => walk(&group.children, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(&scene.nodes, &mut out);
    out
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

fn has_diag(compilation: &lattice_engine::Compilation, code: &str, needle: &str) -> bool {
    compilation
        .diagnostics
        .iter()
        .any(|diag| diag.code == code && diag.message.contains(needle))
}

fn export_canvas() -> Canvas {
    Canvas {
        width: 1920,
        height: 1080,
    }
}

fn caption_placements(compilation: &lattice_engine::Compilation) -> Vec<&lattice_core::Placement> {
    compilation.project.scenes[0]
        .placements
        .iter()
        .filter(|placement| {
            matches!(
                placement.provenance.origin,
                Origin::Invocation { ref command } if command == "caption"
            )
        })
        .collect()
}

#[test]
fn three_oneline_cues_preview_matches_export_evaluate() {
    let source = caption_project(
        r#"  caption "one" at 0s for 1s
  caption "two" at 2s for 1s
  caption "three" at 4s for 1s
  title "Hello" {
    at 6s for 1s
  }
  callout "Tip" {
    at 7s for 1s
  }
"#,
    );
    let compilation = compile(&source);
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    assert_eq!(caption_placements(&compilation).len(), 3);
    let timeline = Engine::timeline(&compilation.project).unwrap();
    for (time, text) in [
        (Time::milliseconds(500), "one"),
        (Time::milliseconds(2500), "two"),
        (Time::milliseconds(4500), "three"),
    ] {
        let preview = evaluate_at(&timeline, time, Canvas::PREVIEW).unwrap();
        let export = evaluate_at(&timeline, time, export_canvas()).unwrap();
        assert_eq!(overlay_texts(&preview), vec![text.to_string()]);
        assert_eq!(overlay_texts(&preview), overlay_texts(&export));
        assert!(
            overlay_bars(&preview).is_empty(),
            "{:?}",
            overlay_bars(&preview)
        );
        assert_eq!(overlay_bars(&preview), overlay_bars(&export));
    }
}

#[test]
fn title_and_callout_still_compile_beside_caption() {
    let source = caption_project(
        r#"  caption "cue" at 0s for 1s
  title "Hello" {
    at 2s for 1s
  }
  callout "Tip" {
    at 4s for 1s
  }
"#,
    );
    let compilation = compile(&source);
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    let kinds: Vec<_> = compilation.project.scenes[0]
        .placements
        .iter()
        .map(|placement| placement.kind)
        .collect();
    assert!(kinds.contains(&PlacementKind::Title));
    assert!(kinds.contains(&PlacementKind::Callout));
    let engine = Engine::default();
    let loci = engine.loci(&compilation).unwrap();
    assert!(loci.iter().any(|locus| locus.kind == LocusKind::Title));
    assert!(loci.iter().any(|locus| locus.kind == LocusKind::Callout));
}

#[test]
fn missing_at_or_for_is_lat_ovl_011() {
    for body in [
        r#"  caption "cue""#,
        r#"  caption "cue" at 1s"#,
        r#"  caption "cue" for 1s"#,
    ] {
        let compilation = compile(&caption_project(body));
        assert!(
            has_diag(&compilation, "LAT-OVL-011", "caption"),
            "{body}: {:?}",
            compilation.diagnostics
        );
        assert!(
            caption_placements(&compilation).is_empty(),
            "{body} must not emit a cue"
        );
    }
}

#[test]
fn duplicate_at_or_for_is_lat_ovl_012() {
    for body in [
        r#"  caption "cue" at 1s for 2s { at 3s }"#,
        r#"  caption "cue" at 1s for 2s { for 3s }"#,
        r#"  caption "cue" at 1s at 2s for 3s"#,
    ] {
        let compilation = compile(&caption_project(body));
        assert!(
            has_diag(&compilation, "LAT-OVL-012", "caption"),
            "{body}: {:?}",
            compilation.diagnostics
        );
        assert!(
            caption_placements(&compilation).is_empty(),
            "{body} must not emit a cue"
        );
    }
}

#[test]
fn default_bar_is_off() {
    let compilation = compile(&caption_project(r#"  caption "cue" at 0s for 1s"#));
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    let visual = caption_placements(&compilation)[0]
        .visual
        .as_ref()
        .expect("caption visual");
    assert_eq!(
        visual.style,
        Some(OverlayStyle {
            bar: Some(OverlayBar::Off),
            ..OverlayStyle::default()
        })
    );
    let timeline = Engine::timeline(&compilation.project).unwrap();
    let scene = evaluate_at(&timeline, Time::ZERO, Canvas::PREVIEW).unwrap();
    assert!(
        overlay_bars(&scene).is_empty(),
        "{:?}",
        overlay_bars(&scene)
    );
}

#[test]
fn explicit_bar_overrides_default() {
    let compilation = compile(&caption_project(
        r##"  caption "cue" at 0s for 1s {
    bar "#00FF00"
  }"##,
    ));
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    assert_eq!(
        caption_placements(&compilation)[0]
            .visual
            .as_ref()
            .and_then(|visual| visual.style.clone())
            .and_then(|style| style.bar),
        Some(OverlayBar::Fill {
            color: Rgba::from_hex_rrggbb("#00FF00").unwrap()
        })
    );
    let timeline = Engine::timeline(&compilation.project).unwrap();
    let scene = evaluate_at(&timeline, Time::ZERO, Canvas::PREVIEW).unwrap();
    assert_eq!(
        overlay_bars(&scene),
        vec![Rgba::from_hex_rrggbb("#00FF00").unwrap()]
    );
}

#[test]
fn core_has_no_caption_kinds() {
    let ir = include_str!("../../lattice-core/src/ir.rs");
    let locus = include_str!("../../lattice-core/src/locus.rs");
    assert!(!ir.contains("CaptionNode"));
    assert!(!ir.contains("PlacementKind::Caption"));
    assert!(!locus.contains("LocusKind::Caption"));
    let compilation = compile(&caption_project(r#"  caption "cue" at 0s for 1s"#));
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    let placement = caption_placements(&compilation)[0];
    assert_eq!(placement.kind, PlacementKind::Title);
    assert!(matches!(
        placement.provenance.origin,
        Origin::Invocation { ref command } if command == "caption"
    ));
}

#[test]
fn caption_locus_is_placement_not_title() {
    let engine = Engine::default();
    assert!(
        engine.uses_wasm_stdlib(),
        "caption must lower through the Wasmtime-hosted WIT component"
    );
    let compilation = compile(&caption_project(
        r#"  caption "cue" at 0s for 1s
  title "Hello" {
    at 2s for 1s
  }
"#,
    ));
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    let loci = engine.loci(&compilation).unwrap();
    let caption = loci
        .iter()
        .find(|locus| {
            matches!(
                locus.provenance.origin,
                Origin::Invocation { ref command } if command == "caption"
            )
        })
        .expect("caption locus");
    assert_eq!(caption.kind, LocusKind::Placement);
    assert_eq!(caption.label, "cue");
    assert!(
        compilation
            .explain
            .iter()
            .any(|event| event.message.contains("caption \"cue\"")
                && event.message.contains("bar off")),
        "{:?}",
        compilation.explain
    );
    let title = loci
        .iter()
        .find(|locus| locus.kind == LocusKind::Title)
        .expect("title locus");
    assert_eq!(title.label, "Hello");
}

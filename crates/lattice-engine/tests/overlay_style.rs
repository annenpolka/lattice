//! CHI-87: overlay style vocab from generic VEL body into `FontSpec` / `TextNode`.

use lattice_core::{
    Canvas, OverlayBar, OverlaySize, OverlayStyle, PlacementKind, RenderNode, RenderScene, Rgba,
    TextNode, Time, Visual, evaluate_at,
};
use lattice_engine::Engine;

fn compile_overlay(word: &str, body: &str) -> lattice_engine::Compilation {
    let source = format!(
        r#"project "overlay-style"
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

fn overlay_visual(compilation: &lattice_engine::Compilation, word: &str) -> Visual {
    let kind = match word {
        "title" => PlacementKind::Title,
        "callout" => PlacementKind::Callout,
        other => panic!("unexpected overlay word `{other}`"),
    };
    compilation.project.scenes[0]
        .placements
        .iter()
        .find(|placement| placement.kind == kind)
        .and_then(|placement| placement.visual.clone())
        .expect("overlay visual")
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

fn evaluated_title(body: &str, canvas: Canvas) -> RenderScene {
    let compilation = compile_overlay("title", body);
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    let timeline = Engine::timeline(&compilation.project).unwrap();
    evaluate_at(&timeline, Time::seconds(0), canvas).unwrap()
}

#[test]
fn omitted_style_keeps_title_yellow_and_callout_cyan() {
    for word in ["title", "callout"] {
        let compilation = compile_overlay(word, "opacity 90");
        assert!(
            !compilation.has_errors(),
            "{word}: {:?}",
            compilation.diagnostics
        );
        assert_eq!(overlay_visual(&compilation, word).style, None);
        let timeline = Engine::timeline(&compilation.project).unwrap();
        let scene = evaluate_at(&timeline, Time::seconds(0), Canvas::PREVIEW).unwrap();
        let text = overlay_text(&scene);
        assert_eq!(text.color, Rgba::WHITE);
        assert_eq!(text.font.family, "LatticeSans");
        assert_eq!(text.font.weight, 400);
        match word {
            "title" => {
                assert_eq!(text.font.size_px, 14);
                assert_eq!(overlay_bars(&scene), vec![Rgba::YELLOW]);
            }
            "callout" => {
                assert_eq!(text.font.size_px, 12);
                assert_eq!(overlay_bars(&scene), vec![Rgba::CYAN]);
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn quoted_hex_color_sets_text_node_color() {
    let compilation = compile_overlay("title", r##"color "#00FF00""##);
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    let style = overlay_visual(&compilation, "title").style.expect("style");
    assert_eq!(style.color, Rgba::from_hex_rrggbb("#00FF00"));
    let timeline = Engine::timeline(&compilation.project).unwrap();
    let scene = evaluate_at(&timeline, Time::seconds(0), Canvas::PREVIEW).unwrap();
    assert_eq!(
        overlay_text(&scene).color,
        Rgba::from_hex_rrggbb("#00FF00").unwrap()
    );
}

#[test]
fn invalid_color_named_unquoted_or_bad_hex_diags() {
    for body in [
        "color green",
        r#"color "red""#,
        "color 00FF00",
        r##"color "#FFF""##,
        r##"color "#GG0000""##,
        r##"color "#00FF00AA""##,
    ] {
        let compilation = compile_overlay("title", body);
        assert!(
            has_diag(&compilation, "LAT-OVL-004", "color"),
            "{body}: {:?}",
            compilation.diagnostics
        );
        assert_eq!(
            overlay_visual(&compilation, "title")
                .style
                .and_then(|style| style.color),
            None,
            "{body} must not store a color"
        );
    }
}

#[test]
fn size_percent_resolves_against_convention_and_explain_shows_base_and_resolved() {
    let compilation = compile_overlay("title", "size 50%");
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    assert!(
        compilation.explain.iter().any(|event| {
            event.message.contains("size 50%")
                && event.message.contains("base")
                && event.message.contains("title height/16")
                && event.message.contains("resolved")
        }),
        "explain must print base + resolved: {:?}",
        compilation.explain
    );
    let style = overlay_visual(&compilation, "title").style.expect("style");
    assert_eq!(style.size, Some(OverlaySize::Percent { milli: 500 }));
    let timeline = Engine::timeline(&compilation.project).unwrap();
    assert_eq!(
        timeline.title_clips().next().unwrap().style,
        overlay_visual(&compilation, "title").style
    );
    let preview = evaluate_at(&timeline, Time::seconds(0), Canvas::PREVIEW).unwrap();
    assert_eq!(overlay_text(&preview).font.size_px, 7);
    let export = evaluate_at(
        &timeline,
        Time::seconds(0),
        Canvas {
            width: 1920,
            height: 1080,
        },
    )
    .unwrap();
    assert_eq!(overlay_text(&export).font.size_px, 33);

    let callout = compile_overlay("callout", "size 50%");
    assert!(
        callout.explain.iter().any(|event| {
            event.message.contains("size 50%")
                && event.message.contains("callout height/20")
                && event.message.contains("resolved")
        }),
        "{:?}",
        callout.explain
    );
    let callout_scene = evaluate_at(
        &Engine::timeline(&callout.project).unwrap(),
        Time::seconds(0),
        Canvas::PREVIEW,
    )
    .unwrap();
    assert_eq!(overlay_text(&callout_scene).font.size_px, 6);
}

#[test]
fn size_px_locks_pixels() {
    let compilation = compile_overlay("title", "size 24px");
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    assert!(
        compilation.explain.iter().any(|event| {
            event.message.contains("size 24px") && event.message.contains("resolved 24px")
        }),
        "{:?}",
        compilation.explain
    );
    let preview = evaluated_title("size 24px", Canvas::PREVIEW);
    let export = evaluated_title(
        "size 24px",
        Canvas {
            width: 1920,
            height: 1080,
        },
    );
    assert_eq!(overlay_text(&preview).font.size_px, 24);
    assert_eq!(overlay_text(&export).font.size_px, 24);
}

#[test]
fn invalid_size_diags() {
    for body in ["size 24", "size large", "size 0%"] {
        let compilation = compile_overlay("title", body);
        assert!(
            has_diag(&compilation, "LAT-OVL-005", "size"),
            "{body}: {:?}",
            compilation.diagnostics
        );
    }
}

#[test]
fn weight_bold_int_and_normal_map_to_font_spec() {
    for (body, expected) in [
        ("weight bold", 700),
        ("weight 700", 700),
        ("weight normal", 400),
    ] {
        let compilation = compile_overlay("title", body);
        assert!(
            !compilation.has_errors(),
            "{body}: {:?}",
            compilation.diagnostics
        );
        let style = overlay_visual(&compilation, "title").style.expect("style");
        assert_eq!(style.weight, Some(expected), "{body}");
        let scene = evaluate_at(
            &Engine::timeline(&compilation.project).unwrap(),
            Time::seconds(0),
            Canvas::PREVIEW,
        )
        .unwrap();
        assert_eq!(overlay_text(&scene).font.weight, expected, "{body}");
    }
}

#[test]
fn invalid_weight_diags() {
    for body in [
        "weight light",
        "weight 0",
        "weight 1001",
        r#"weight "bold""#,
    ] {
        let compilation = compile_overlay("title", body);
        assert!(
            has_diag(&compilation, "LAT-OVL-006", "weight"),
            "{body}: {:?}",
            compilation.diagnostics
        );
    }
}

#[test]
fn family_string_lands_on_font_spec() {
    let compilation = compile_overlay("title", r#"family "LatticeSans""#);
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    let style = overlay_visual(&compilation, "title").style.expect("style");
    assert_eq!(style.family.as_deref(), Some("LatticeSans"));
    let scene = evaluate_at(
        &Engine::timeline(&compilation.project).unwrap(),
        Time::seconds(0),
        Canvas::PREVIEW,
    )
    .unwrap();
    assert_eq!(overlay_text(&scene).font.family, "LatticeSans");
}

#[test]
fn invalid_family_diags() {
    let compilation = compile_overlay("title", "family LatticeSans");
    assert!(
        has_diag(&compilation, "LAT-OVL-007", "family"),
        "{:?}",
        compilation.diagnostics
    );
}

#[test]
fn bar_off_omits_shape_and_hex_sets_fill() {
    let off = compile_overlay("title", "bar off");
    assert!(!off.has_errors(), "{:?}", off.diagnostics);
    assert_eq!(
        overlay_visual(&off, "title").style,
        Some(OverlayStyle {
            bar: Some(OverlayBar::Off),
            ..OverlayStyle::default()
        })
    );
    let off_scene = evaluate_at(
        &Engine::timeline(&off.project).unwrap(),
        Time::seconds(0),
        Canvas::PREVIEW,
    )
    .unwrap();
    assert!(overlay_bars(&off_scene).is_empty());

    let fill = compile_overlay("title", r##"bar "#FF00FF""##);
    assert!(!fill.has_errors(), "{:?}", fill.diagnostics);
    let fill_scene = evaluate_at(
        &Engine::timeline(&fill.project).unwrap(),
        Time::seconds(0),
        Canvas::PREVIEW,
    )
    .unwrap();
    assert_eq!(
        overlay_bars(&fill_scene),
        vec![Rgba::from_hex_rrggbb("#FF00FF").unwrap()]
    );
}

#[test]
fn invalid_bar_diags() {
    for body in ["bar on", r#"bar "yellow""#, "bar 1"] {
        let compilation = compile_overlay("title", body);
        assert!(
            has_diag(&compilation, "LAT-OVL-008", "bar"),
            "{body}: {:?}",
            compilation.diagnostics
        );
    }
}

#[test]
fn align_center_stays_unknown() {
    for word in ["title", "callout"] {
        let compilation = compile_overlay(word, "align center");
        assert!(
            has_diag(&compilation, "LAT-OVL-003", "`align`"),
            "{word}: {:?}",
            compilation.diagnostics
        );
        assert!(
            compilation
                .diagnostics
                .iter()
                .all(|diag| diag.code != "LAT-OVL-003" || diag.message.contains("`align`")),
            "align must not be a working style word: {:?}",
            compilation.diagnostics
        );
    }
}

#[test]
fn styled_title_matches_on_evaluate_used_by_preview_and_export() {
    let compilation = compile_overlay(
        "title",
        r##"color "#00FF00"
    size 24px
    weight 700
    family "LatticeSans"
    bar "#FF00FF""##,
    );
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
    let preview_text = overlay_text(&preview);
    let export_text = overlay_text(&export);
    assert_eq!(preview_text.color, export_text.color);
    assert_eq!(preview_text.font.family, export_text.font.family);
    assert_eq!(preview_text.font.weight, export_text.font.weight);
    assert_eq!(preview_text.font.size_px, 24);
    assert_eq!(export_text.font.size_px, 24);
    assert_eq!(overlay_bars(&preview), overlay_bars(&export));
    assert_eq!(
        overlay_bars(&preview),
        vec![Rgba::from_hex_rrggbb("#FF00FF").unwrap()]
    );
}

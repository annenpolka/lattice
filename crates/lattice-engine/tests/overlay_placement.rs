use lattice_core::{PlacementKind, Visual};
use lattice_engine::{Compilation, Engine, NormalizedPosition, NormalizedScale};

fn compile_overlay(word: &str, body: &str) -> Compilation {
    let source = format!(
        r#"project "overlay-diag"
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

fn overlay_visual(compilation: &Compilation, word: &str) -> Visual {
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

fn has_diag(compilation: &Compilation, code: &str, needle: &str) -> bool {
    compilation
        .diagnostics
        .iter()
        .any(|diag| diag.code == code && diag.message.contains(needle))
}

#[test]
fn out_of_range_and_unitless_position_are_lowering_diagnostics() {
    for word in ["title", "callout"] {
        for position in ["150%", "10"] {
            let compilation = compile_overlay(word, &format!("position {position}"));
            assert!(
                compilation.diagnostics.iter().any(|diag| {
                    diag.code == "LAT-OVL-001"
                        && diag.message.contains("position")
                        && diag.message.contains("out-of-range")
                        && diag.message.contains("unit-less")
                        && diag.message.contains("non-tuple")
                }),
                "{word} position {position}: {:?}",
                compilation.diagnostics
            );
            assert_eq!(
                overlay_visual(&compilation, word).position,
                None,
                "{word} position {position} must stay unset"
            );
        }
    }
}

#[test]
fn unknown_overlay_body_word_does_not_vanish() {
    for word in ["title", "callout"] {
        let compilation = compile_overlay(word, "anchor top");
        assert!(
            compilation
                .diagnostics
                .iter()
                .any(|diag| { diag.code == "LAT-OVL-003" && diag.message.contains("`anchor`") }),
            "{word} anchor top: {:?}",
            compilation.diagnostics
        );
        assert!(
            compilation.project.scenes[0]
                .placements
                .iter()
                .any(|placement| placement.visual.is_some()),
            "{word} still lowers; the unknown word must not swallow the overlay"
        );
    }
}

#[test]
fn unknown_overlay_body_modifier_does_not_vanish() {
    for word in ["title", "callout"] {
        let compilation = compile_overlay(word, "over clip");
        assert!(
            compilation
                .diagnostics
                .iter()
                .any(|diag| { diag.code == "LAT-OVL-003" && diag.message.contains("`over`") }),
            "{word} over clip: {:?}",
            compilation.diagnostics
        );
    }
}

#[test]
fn valid_position_tuple_and_scale_percent_do_not_error() {
    for word in ["title", "callout"] {
        let compilation = compile_overlay(word, "position (25%, 10%)\n    scale 50%");
        assert!(
            !compilation.has_errors(),
            "{word}: {:?}",
            compilation.diagnostics
        );
        let visual = overlay_visual(&compilation, word);
        assert_eq!(visual.position, NormalizedPosition::new(2_500, 1_000));
        assert_eq!(visual.scale, NormalizedScale::new(500));
        assert!(
            compilation.explain.iter().any(|event| {
                event.message.contains("position (25.00%, 10.00%)")
                    && event.message.contains("scale 50%")
            }),
            "{word} explain: {:?}",
            compilation.explain
        );
    }
}

#[test]
fn invalid_scale_is_a_lowering_diagnostic() {
    for word in ["title", "callout"] {
        let compilation = compile_overlay(word, "scale 10");
        assert!(
            has_diag(&compilation, "LAT-OVL-002", "scale"),
            "{word} scale 10: {:?}",
            compilation.diagnostics
        );
        assert_eq!(overlay_visual(&compilation, word).scale, None);
    }
}

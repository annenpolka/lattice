use lattice_core::{LocusKind, SemanticEdit, Time};
use lattice_engine::Engine;

const VEL: &str = r#"project "gap-demo"
convention commentary
media game "capture.mp4"

sequence main {
  a
  // deliberate pause
  gap 500ms
  b
}

scene a {
  game[0s..2s] as clip_a
}

scene b {
  game[2s..5s] as clip_b
}
"#;

#[test]
fn gap_lowers_through_stdlib_to_empty_timeline_time() {
    let engine = Engine::default();
    assert!(engine.uses_wasm_stdlib());
    let compilation = engine.compile(VEL).unwrap();
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    let sequence = &compilation.project.sequences[0];
    assert_eq!(
        sequence.scene_offsets,
        vec![Time::ZERO, Time::milliseconds(500)]
    );
    assert!(compilation.explain.iter().any(|event| {
        matches!(
            event.origin,
            lattice_core::Origin::Invocation { ref command } if command == "gap"
        ) && event.message.contains("before scene `b`")
    }));

    let timeline = Engine::timeline(&compilation.project).unwrap();
    let a = timeline
        .video_clips()
        .find(|clip| clip.id.starts_with("a:video:"))
        .unwrap();
    let b = timeline
        .video_clips()
        .find(|clip| clip.id.starts_with("b:video:"))
        .unwrap();
    assert_eq!(a.span.start, Time::ZERO);
    assert_eq!(a.span.end(), Time::seconds(2));
    assert_eq!(b.span.start, Time::milliseconds(2_500));
    assert_eq!(timeline.duration, Time::milliseconds(5_500));
    assert!(
        timeline
            .clips
            .iter()
            .all(|clip| !clip.span.contains(Time::milliseconds(2_250))),
        "the explicit gap must contain no visual or audio clip"
    );
}

#[test]
fn reorder_preserves_gap_and_comment_at_the_flow_boundary() {
    let engine = Engine::default();
    let compilation = engine.compile(VEL).unwrap();
    let scene_b = engine
        .loci(&compilation)
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == LocusKind::Scene && locus.label == "b")
        .unwrap();
    let proposal = engine
        .propose(
            &compilation,
            &scene_b,
            SemanticEdit::ReorderScene {
                before: Some("a".into()),
            },
        )
        .unwrap();
    let source = engine.apply_proposal(VEL, &proposal).unwrap();
    assert!(source.contains("// deliberate pause\n  gap 500ms"));

    let compiled = engine.compile(&source).unwrap();
    assert_eq!(
        compiled.project.sequences[0].scene_ids,
        vec!["scene:b".to_string(), "scene:a".to_string()]
    );
    assert_eq!(
        compiled.project.sequences[0].scene_offsets,
        vec![Time::ZERO, Time::milliseconds(500)]
    );
    let timeline = Engine::timeline(&compiled.project).unwrap();
    let a = timeline
        .video_clips()
        .find(|clip| clip.id.starts_with("a:video:"))
        .unwrap();
    assert_eq!(a.span.start, Time::milliseconds(3_500));
}

#[test]
fn gap_rejects_ambiguous_or_negative_flow_positions() {
    let engine = Engine::default();
    for (sequence, expected) in [
        ("gap 1s\n  a", "must follow a scene"),
        ("a\n  gap 1s", "must be followed by a scene"),
        ("a\n  gap 1s\n  gap 2s\n  b", "consecutive"),
        ("a\n  gap -1s\n  b", "must not be negative"),
    ] {
        let source = format!(
            "project \"bad-gap\"\nsequence main {{\n  {sequence}\n}}\nscene a {{}}\nscene b {{}}\n"
        );
        let error = engine.compile(&source).unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
    }
}

#[test]
fn zero_gap_remains_explainable_without_changing_the_timeline() {
    let engine = Engine::default();
    let source = VEL.replace("gap 500ms", "gap 0s");
    let compilation = engine.compile(&source).unwrap();
    assert!(compilation.project.sequences[0].scene_offsets.is_empty());
    assert!(compilation.explain.iter().any(|event| {
        matches!(
            event.origin,
            lattice_core::Origin::Invocation { ref command } if command == "gap"
        ) && event.message.contains("inserts 0s before scene `b`")
    }));
    assert_eq!(
        Engine::timeline(&compilation.project).unwrap().duration,
        Time::seconds(5)
    );
}

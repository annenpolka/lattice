//! Propose → inspect → Apply / Reject against the shipped VEL, via Engine.

use lattice_core::SemanticEdit;
use lattice_engine::Engine;

const VEL: &str = include_str!("../../../examples/gameplay-commentary/main.vel");

#[test]
fn propose_inspect_reject_leaves_source_bytes_unchanged() {
    let dir = std::env::temp_dir().join("lattice-edit-reject");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.vel");
    std::fs::write(&path, VEL).unwrap();
    let original = std::fs::read(&path).unwrap();

    let engine = Engine::default();
    let compilation = engine.compile_path(&path).unwrap();
    let title = engine
        .locus_for_node(&compilation, "demo:title:1")
        .unwrap()
        .unwrap();
    let proposal = engine
        .propose(
            &compilation,
            &title,
            SemanticEdit::Title {
                text: Some("World".into()),
                at: None,
                duration: None,
                opacity: None,
            },
        )
        .unwrap();
    assert!(proposal.vel_diff.contains("World"), "{}", proposal.vel_diff);
    assert!(proposal.new_source.contains("World"));
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "proposal must not write VEL"
    );

    let rejected = engine.reject_proposal(&compilation.source, &proposal);
    assert_eq!(rejected.as_bytes(), original);
    std::fs::write(&path, rejected).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

#[test]
fn propose_apply_rewrites_source_and_recompile_reflects_title() {
    let dir = std::env::temp_dir().join("lattice-edit-apply");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.vel");
    std::fs::write(&path, VEL).unwrap();

    let engine = Engine::default();
    let compilation = engine.compile_path(&path).unwrap();
    let title = engine
        .locus_for_node(&compilation, "demo:title:1")
        .unwrap()
        .unwrap();
    let proposal = engine
        .propose(
            &compilation,
            &title,
            SemanticEdit::Title {
                text: Some("World".into()),
                at: Some(lattice_core::Time::seconds(1)),
                duration: Some(lattice_core::Time::seconds(4)),
                opacity: Some(80),
            },
        )
        .unwrap();
    assert!(proposal.description.contains("World"));

    let applied = engine.apply_proposal(&compilation.source, &proposal);
    std::fs::write(&path, &applied).unwrap();
    assert_ne!(applied, VEL);
    assert!(applied.contains("World"));
    assert!(!applied.contains("title \"Hello\""));

    let recompiled = engine.compile_path(&path).unwrap();
    assert!(!recompiled.has_errors(), "{:?}", recompiled.diagnostics);
    let timeline = Engine::timeline(&recompiled.project).unwrap();
    let title = timeline.title_clips().next().expect("title");
    assert_eq!(title.text.as_deref(), Some("World"));
    assert_eq!(title.span.start, lattice_core::Time::seconds(1));
    assert_eq!(title.span.duration, lattice_core::Time::seconds(4));
    assert_eq!(title.opacity, Some(80));
}

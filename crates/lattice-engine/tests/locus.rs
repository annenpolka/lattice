//! Shared locus identity from the shipped gameplay-commentary VEL.

use lattice_core::{LocusKind, Time};
use lattice_engine::Engine;

const VEL: &str = include_str!("../../../examples/gameplay-commentary/main.vel");

#[test]
fn title_locus_identity_matches_source_core_and_timeline() {
    let engine = Engine::default();
    let compilation = engine.compile(VEL).expect("compile shipped VEL");
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);

    let title = engine
        .locus_for_node(&compilation, "demo:title:1")
        .expect("lookup")
        .expect("title node");
    assert_eq!(title.kind, LocusKind::Title);
    assert_eq!(title.node_id, "demo:title:1");
    assert_eq!(title.id.as_str(), "demo:title:1");
    assert_eq!(title.label, "Hello");

    let span = title.source_span.expect("title source span");
    let mid = span.start + (span.end - span.start) / 2;
    let from_source = engine
        .locus_at_source(&compilation, mid)
        .expect("source projection")
        .expect("hit");
    assert_eq!(from_source.id, title.id);
    assert_eq!(from_source.node_id, title.node_id);

    let from_timeline = engine
        .locus_at_timeline(&compilation, Time::seconds(3))
        .expect("timeline projection")
        .expect("hit at 3s");
    assert_eq!(from_timeline.id, title.id);

    let projection = engine.inspect(&compilation, &title.id).expect("inspect");
    assert_eq!(projection.locus.id, title.id);
    assert_eq!(
        projection.core.node_id,
        projection.timeline.expect("timeline range").clip_id
    );
    assert_eq!(
        projection.source.expect("source span").span,
        title.source_span.expect("span")
    );

    let json = serde_json::to_value(&title).expect("agent JSON");
    assert_eq!(json["id"], "demo:title:1");
    assert_eq!(json["kind"], "title");
    assert_eq!(json["node_id"], "demo:title:1");
}

#[test]
fn freeze_and_title_go_through_wasm_stdlib() {
    let engine = Engine::default();
    assert!(
        engine.uses_wasm_stdlib(),
        "freeze/title must lower through the Wasmtime-hosted WIT component"
    );
    let compilation = engine.compile(VEL).expect("compile");
    assert!(
        compilation
            .explain
            .iter()
            .any(|event| event.message.contains("TimeMap hold (rate 0)"))
    );
}

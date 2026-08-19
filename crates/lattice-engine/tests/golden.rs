use lattice_engine::Engine;

const DEMO: &str = include_str!("../../../examples/gameplay-commentary/main.vel");

#[test]
fn compile_demo_matches_golden_ir() {
    let compilation = Engine::default().compile(DEMO).expect("compile");
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    let ir = serde_json::to_string_pretty(&compilation.project).unwrap() + "\n";
    let expected = include_str!("../../../tests/golden/compile/basic-scene/expected.ir.json");
    assert_eq!(ir, expected);
}

#[test]
fn explain_demo_matches_golden() {
    let compilation = Engine::default().compile(DEMO).expect("compile");
    let mut text = String::new();
    for event in &compilation.explain {
        text.push_str(&event.message);
        text.push('\n');
    }
    let expected = include_str!("../../../tests/golden/explain/basic-scene/expected.explain.txt");
    assert_eq!(text, expected);
}

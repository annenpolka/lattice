//! Playable example `examples/warframe-cut/main.vel` compiles to a multi-scene cut.

use lattice_core::Time;
use lattice_engine::Engine;

const VEL: &str = include_str!("../../../examples/warframe-cut/main.vel");

#[test]
fn warframe_cut_compiles_four_scenes_with_stdlib_vocab() {
    let compilation = Engine::default().compile(VEL).expect("compile");
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    assert_eq!(compilation.project.scenes.len(), 4);
    assert_eq!(compilation.project.sequences[0].scene_ids.len(), 4);

    let timeline = Engine::timeline(&compilation.project).expect("flatten");
    let hold = Time::from_decimal_seconds(1, 5, 1).unwrap();
    let expected = Time::seconds(4)
        + Time::seconds(8)
        + Time::seconds(6)
        + hold
        + Time::from_decimal_seconds(7, 5, 1).unwrap();
    assert_eq!(timeline.duration, expected);

    assert_eq!(timeline.freeze_segments().len(), 1);
    assert_eq!(timeline.title_clips().count(), 4);
    assert_eq!(timeline.callout_clips().count(), 2);
    assert!(
        compilation
            .explain
            .iter()
            .any(|event| event.message.contains("-15dB")),
        "hold scene must duck: {:?}",
        compilation.explain
    );
    assert!(
        compilation
            .explain
            .iter()
            .any(|event| event.message.contains("speech")),
        "speech intent at compile"
    );

    let title = Engine::default()
        .locus_for_node(&compilation, "hook:title:1")
        .expect("lookup")
        .expect("first hook title");
    assert_eq!(title.label, "Warframe");
}

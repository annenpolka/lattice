//! Studio session talks to the real Engine. No GPUI types.

use std::path::PathBuf;

use lattice_engine::Origin;
use lattice_studio::StudioSession;

fn demo_vel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/gameplay-commentary/main.vel")
        .canonicalize()
        .expect("demo vel")
}

#[test]
fn open_compiles_through_engine_and_exposes_locus_provenance_preview() {
    let mut session = StudioSession::open(demo_vel()).expect("open");
    assert!(
        !session.compilation().has_errors(),
        "{:?}",
        session.diagnostics()
    );
    assert!(session.source().contains("title \"Hello\""));
    let title = session.point_at_title().unwrap().expect("title locus");
    assert_eq!(title.label, "Hello");
    let current = session.current_locus().unwrap().expect("current");
    assert_eq!(current.id, title.id);
    let provenance = session.current_provenance().unwrap().expect("provenance");
    assert!(
        matches!(
            provenance.origin,
            Origin::Invocation { ref command } if command == "title"
        ),
        "{:?}",
        provenance.origin
    );
    let plan = session.preview_plan().expect("preview plan");
    assert!(
        !plan.overlays.is_empty(),
        "preview plan should include the title"
    );
    assert!(session.uses_engine_not_own_compiler());
}

#[test]
fn studio_crate_source_has_no_gpui_in_session() {
    let session = include_str!("../src/session.rs");
    assert!(
        !session.contains("gpui"),
        "session must stay an Engine client without GPUI types"
    );
    let core = include_str!("../../../crates/lattice-core/Cargo.toml");
    assert!(!core.contains("gpui"));
}

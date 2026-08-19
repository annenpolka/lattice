//! Resolve lock: second pass must not call the generated-media provider.

use lattice_engine::{CountingProvider, Engine, LocalToneProvider, ResolveOptions};

const VEL: &str = include_str!("../../../examples/gameplay-commentary/main.vel");

#[test]
fn compile_never_calls_generated_media_provider() {
    let engine = Engine::default();
    let compilation = engine.compile(VEL).unwrap();
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    assert!(
        compilation.project.media.iter().any(|media| matches!(
            media.locator,
            lattice_core::MediaLocator::Generated { ref generator, .. } if generator == "speech"
        )),
        "speech must exist as generated intent after compile"
    );
}

#[test]
fn second_resolve_against_lock_does_not_invoke_provider() {
    let engine = Engine::default();
    let compilation = engine.compile(VEL).unwrap();
    let dir = std::env::temp_dir().join("lattice-resolve-lock");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut provider = CountingProvider::new(LocalToneProvider);
    let first = engine
        .resolve(
            &compilation.project,
            &ResolveOptions {
                media_root: &dir,
                artifact_dir: &dir.join("artifacts"),
                lock: None,
            },
            &mut provider,
        )
        .unwrap();
    assert_eq!(provider.calls, 1, "first resolve materializes speech");
    assert!(
        first
            .lock
            .assets
            .iter()
            .any(|asset| asset.generator.as_deref() == Some("speech"))
    );
    assert!(first.assets.iter().any(|asset| {
        matches!(asset.locator, lattice_core::MediaLocator::Generated { .. })
            && std::path::Path::new(&asset.path).is_file()
    }));

    let second = engine
        .resolve(
            &compilation.project,
            &ResolveOptions {
                media_root: &dir,
                artifact_dir: &dir.join("artifacts"),
                lock: Some(&first.lock),
            },
            &mut provider,
        )
        .unwrap();
    assert_eq!(
        provider.calls, 1,
        "locked resolve must not call the provider again"
    );
    assert!(second.assets.iter().any(|asset| asset.from_lock));
}

//! GPUI Studio shell.
//!
//! Intentionally free of `gpui` types in Milestone 0 so the crate graph stays
//! honest: Studio is an engine client. When the shell lands, GPUI stays here.

use lattice_engine::Engine;

/// Marker that the Studio process would hold an [`Engine`].
pub fn engine() -> Engine {
    Engine::default()
}

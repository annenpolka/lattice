//! Lattice Studio: a GPUI client of `lattice-engine`.
//!
//! Compile semantics live in the engine. This crate holds a session and, when
//! built with `--features window`, a GPUI shell over that session.

mod layout;
mod session;
pub mod trace;

pub use layout::{
    CanvasOverlay, CanvasView, InspectorView, ReviewView, SourceView, StudioLayout,
    TimelineClipView, TimelineTrackView, TimelineView, TreeNode,
};
pub use session::{StudioSession, fit_preview_size};

use lattice_engine::Engine;

/// Marker that the Studio process holds an [`Engine`].
pub fn engine() -> Engine {
    Engine::default()
}

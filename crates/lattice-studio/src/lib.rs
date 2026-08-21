//! Lattice Studio: a GPUI client of `lattice-engine`.
//!
//! Compile semantics live in the engine. This crate holds a session and, when
//! built with `--features window`, a GPUI shell over that session.

pub mod audio;
mod canvas;
mod gesture;
mod interaction;
mod layout;
mod preview;
mod semantic_state;
mod session;
pub mod trace;
mod ui_fixture;
mod viewport;

pub use audio::{
    AudioDeviceFailureKind, AudioDeviceFormat, AudioDeviceInitError, AudioMonitor,
    AudioMonitorConfig, AudioMonitorStatus, AudioOpenError, AudioOutput, AudioPrepareError,
    AudioPrepareJob, AudioProgram, AudioReposition, AudioRuntimeError, AudioRuntimeStage,
    AudioSyncReport, AudioTransportChange,
};
pub use canvas::{
    CanvasDrag, CanvasDragError, CanvasEditPatch, CanvasPoint, CanvasRect, CanvasResize,
    CanvasResizeError, CanvasResizePatch, CanvasResizePreview, CanvasSize, ResizeCorner,
};
pub use gesture::{
    CursorKind, DRAG_THRESHOLD_PX, Edge, GestureOutcome, SNAP_THRESHOLD_PX, TRIM_HANDLE_PX,
    TimelineGesture, TimelineHit, snap_time,
};
pub use layout::{
    CanvasOverlay, CanvasView, InspectorView, ReviewView, SourceView, StudioLayout,
    TimelineClipView, TimelineTrackView, TimelineView, TreeNode,
};
pub use preview::{
    PLAYBACK_TICK, PreviewInbox, PreviewInboxStats, PreviewJob, PreviewMailbox, PreviewPush,
    playback_frame_at_or_before, playback_target,
};
pub use semantic_state::{snapshot as semantic_snapshot, write_geom_file, write_state_file};
pub use session::{StudioSession, fit_preview_size};
pub use ui_fixture::UiFixture;
pub use viewport::TimelineViewport;

use lattice_engine::Engine;

/// Marker that the Studio process holds an [`Engine`].
pub fn engine() -> Engine {
    Engine::default()
}

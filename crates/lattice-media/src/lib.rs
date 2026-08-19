//! Media and render backends.
//!
//! `FFmpeg` is the first planned backend. It must not leak into `lattice-core`.
//! Milestone 0 does not decode or encode.

use lattice_core::Project;

/// Placeholder for the future render-plan to `FFmpeg` driver.
pub fn render_unsupported(_project: &Project) -> Result<(), String> {
    Err("lattice-media: FFmpeg render is not implemented".into())
}

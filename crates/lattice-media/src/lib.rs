//! Media and render backends.
//!
//! `FFmpeg` is the first encode adapter. It consumes a flattened [`Timeline`],
//! never VEL source text.

mod export;
mod fixture;
mod plan;
mod probe;

pub use export::{ExportError, ExportReport, PreviewOptions, export_preview, extract_frame};
pub use fixture::{DEFAULT_SOURCE_DURATION_SECS, generate_test_source};
pub use plan::{OverlayWindow, PlanSegment, RenderPlan, plan_from_timeline};
pub use probe::{ProbeError, probe_duration, title_bar_present};

pub const PREVIEW_FPS_NUM: i64 = 10;
pub const PREVIEW_FPS_DEN: i64 = 1;
pub const PREVIEW_WIDTH: u32 = 320;
pub const PREVIEW_HEIGHT: u32 = 180;

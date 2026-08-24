//! Lowering registry.
//!
//! The host currently runs in-process builtins. The public types are shaped
//! like the WIT contract in `wit/lattice/` so Wasmtime can replace the bodies
//! later without changing Core IR.

mod builtins;
mod caption;
mod host;
mod overlay_body;
mod overlay_preset;
mod overlay_registry;
mod registry;
mod view;

pub use overlay_preset::{INVALID_PRESET, REDEFINED_PRESET, UNKNOWN_PRESET, register_dsl_preset};
pub use overlay_registry::{
    LOWER_THIRD, OverlayPresetRegistry, OverlayPresetSource, lower_third_style,
    merge_explicit_over_preset, title_preset_style,
};
pub use registry::LoweringRegistry;
pub use view::{
    BodyItem, ExplainLine, InvocationView, LoweringError, SceneDraft, ValueView, scene_duration,
};

//! Lowering registry.
//!
//! The host currently runs in-process builtins. The public types are shaped
//! like the WIT contract in `wit/lattice/` so Wasmtime can replace the bodies
//! later without changing Core IR.

mod builtins;
mod registry;
mod view;

pub use registry::LoweringRegistry;
pub use view::{
    BodyItem, ExplainLine, InvocationView, LoweringError, SceneDraft, ValueView, scene_duration,
};

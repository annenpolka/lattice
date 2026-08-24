use lattice_core::OverlayStyle;

use crate::builtins::{
    apply_commentary, lower_callout, lower_caption, lower_fade, lower_freeze, lower_gain,
    lower_speech, lower_title,
};
use crate::overlay_registry::{OverlayPresetRegistry, OverlayPresetSource};
use crate::view::{InvocationView, LoweringError, SceneDraft};

type Builtin = fn(&InvocationView, &mut SceneDraft) -> Result<(), LoweringError>;

/// Command → lowering function. Parser does not consult this map.
pub struct LoweringRegistry {
    builtins: Vec<(&'static str, Builtin)>,
    wasm: Option<crate::host::WasmStdlib>,
    presets: OverlayPresetRegistry,
}

impl LoweringRegistry {
    pub fn stdlib() -> Self {
        let wasm = crate::host::WasmStdlib::load().ok();
        let mut presets = OverlayPresetRegistry::builtin();
        if let Some(wasm) = &wasm
            && let Ok(entries) = wasm.overlay_presets()
        {
            for (name, style) in entries {
                let _ = presets.register(name, style, OverlayPresetSource::Wasm);
            }
        }
        Self {
            builtins: vec![
                ("freeze", lower_freeze),
                ("title", lower_title),
                ("caption", lower_caption),
                ("callout", lower_callout),
                ("fade", lower_fade),
                ("gain", lower_gain),
                ("speech", lower_speech),
            ],
            wasm,
            presets,
        }
    }

    pub fn uses_wasm(&self) -> bool {
        self.wasm.is_some()
    }

    #[must_use]
    pub fn overlay_presets(&self) -> OverlayPresetRegistry {
        self.presets.clone()
    }

    pub fn register_overlay_preset(
        &mut self,
        name: impl Into<String>,
        style: OverlayStyle,
        source: OverlayPresetSource,
    ) -> Result<(), String> {
        self.presets.register(name, style, source)
    }

    pub fn lower(&self, inv: &InvocationView, draft: &mut SceneDraft) -> Result<(), LoweringError> {
        if matches!(inv.command.as_str(), "freeze" | "title" | "caption")
            && let Some(wasm) = &self.wasm
        {
            return wasm.lower(inv, draft);
        }
        if let Some((_, func)) = self.builtins.iter().find(|(name, _)| *name == inv.command) {
            return func(inv, draft);
        }
        draft.diagnostics.push(lattice_core::Diagnostic::error(
            "LAT-DSL-001",
            format!(
                "unknown invocation `{}` (not in the host builtin registry)",
                inv.command
            ),
            Some(inv.span),
        ));
        Ok(())
    }

    pub fn apply_convention(&self, name: Option<&str>, draft: &mut SceneDraft) {
        match name {
            Some("commentary") => apply_commentary(draft),
            Some(other) => draft.diagnostics.push(lattice_core::Diagnostic::warning(
                "LAT-CONV-001",
                format!("unknown convention `{other}`"),
                None,
            )),
            None => {}
        }
    }
}

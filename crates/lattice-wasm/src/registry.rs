use lattice_core::{Diagnostic, OverlayStyle};

use crate::builtins::{
    apply_commentary, lower_callout, lower_caption, lower_fade, lower_freeze, lower_gain,
    lower_speech, lower_title,
};
use crate::overlay_preset::{ingest_wasm_presets, register_dsl_preset};
use crate::overlay_registry::OverlayPresetRegistry;
use crate::view::{ExplainLine, InvocationView, LoweringError, SceneDraft};

type Builtin = fn(&InvocationView, &mut SceneDraft) -> Result<(), LoweringError>;

/// Command → lowering function. Parser does not consult this map.
pub struct LoweringRegistry {
    builtins: Vec<(&'static str, Builtin)>,
    wasm: Option<crate::host::WasmStdlib>,
    presets: OverlayPresetRegistry,
    preset_diagnostics: Vec<Diagnostic>,
}

impl LoweringRegistry {
    pub fn stdlib() -> Self {
        let wasm = crate::host::WasmStdlib::load().ok();
        let mut presets = OverlayPresetRegistry::builtin();
        let preset_diagnostics = match &wasm {
            Some(wasm) => ingest_wasm_presets(&mut presets, wasm.overlay_presets()),
            None => Vec::new(),
        };
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
            preset_diagnostics,
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
        source: crate::overlay_registry::OverlayPresetSource,
    ) -> Result<(), String> {
        self.presets.register(name, style, source)
    }

    #[must_use]
    pub fn preset_diagnostics(&self) -> &[Diagnostic] {
        &self.preset_diagnostics
    }

    /// Document-scope words. Engine does not match command names.
    pub fn lower_document(
        &self,
        inv: &InvocationView,
        presets: &mut OverlayPresetRegistry,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<ExplainLine> {
        match inv.command.as_str() {
            "overlay-preset" => register_dsl_preset(inv, presets, diagnostics),
            _ => None,
        }
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

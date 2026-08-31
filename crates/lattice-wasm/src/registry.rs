use lattice_core::{Diagnostic, OverlayStyle, Time};

use crate::builtins::{
    apply_commentary, lower_callout, lower_caption, lower_fade, lower_freeze, lower_gain,
    lower_speech, lower_title,
};
use crate::overlay_preset::{ingest_wasm_presets, register_dsl_preset};
use crate::overlay_registry::OverlayPresetRegistry;
use crate::view::{ExplainLine, InvocationView, LoweringError, SceneDraft};

type Builtin = fn(&InvocationView, &mut SceneDraft) -> Result<(), LoweringError>;

const WASM_SCENE_COMMANDS: &[&str] = &[
    "freeze", "title", "caption", "callout", "fade", "gain", "speech",
];
const WASM_SEQUENCE_COMMANDS: &[&str] = &["gap"];

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

    /// Whether a generic VEL invocation name is supplied by this registry.
    ///
    /// Editor projections use this to distinguish stdlib words without
    /// teaching the VEL parser their meaning.
    #[must_use]
    pub fn handles_invocation(&self, command: &str) -> bool {
        self.builtins.iter().any(|(name, _)| *name == command)
            || WASM_SEQUENCE_COMMANDS.contains(&command)
            || self.handles_document(command)
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
    #[must_use]
    pub fn handles_document(&self, command: &str) -> bool {
        matches!(command, "overlay-preset")
    }

    /// Unknown top-level invocation (not a document-scope word).
    #[must_use]
    pub fn unknown_document_invocation(&self, name: &str, span: lattice_core::Span) -> Diagnostic {
        Diagnostic::error(
            "LAT-DSL-001",
            format!("unknown invocation `{name}` (not a document-scope word)"),
            Some(span),
        )
    }

    /// Document-scope word whose arguments could not be converted.
    #[must_use]
    pub fn invalid_document_args(&self, command: &str, span: lattice_core::Span) -> Diagnostic {
        match command {
            "overlay-preset" => Diagnostic::error(
                crate::overlay_preset::INVALID_PRESET,
                "unsupported argument in `overlay-preset`",
                Some(span),
            ),
            other => Diagnostic::error(
                "LAT-DSL-001",
                format!("unsupported argument in `{other}`"),
                Some(span),
            ),
        }
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
        if WASM_SCENE_COMMANDS.contains(&inv.command.as_str())
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

    /// Lower sequence-scope `gap` to an additional offset before the next scene.
    /// Other names remain Engine-owned scene references.
    pub fn lower_sequence_gap(&self, inv: &InvocationView) -> Result<Option<Time>, LoweringError> {
        if inv.command != "gap" {
            return Ok(None);
        }
        if !inv.modifiers.is_empty() || !inv.body.is_empty() {
            return Err(LoweringError::Message(
                "`gap` accepts one inline duration and no body or modifiers".into(),
            ));
        }
        let [duration] = inv.args.as_slice() else {
            return Err(LoweringError::Message(
                "`gap` needs exactly one duration".into(),
            ));
        };
        let duration = duration
            .as_time()
            .ok_or_else(|| LoweringError::Message("`gap` needs a time duration".into()))?;
        if let Some(wasm) = &self.wasm {
            return wasm.sequence_gap(duration).map(Some);
        }
        if duration < Time::ZERO {
            return Err(LoweringError::Message(
                "gap duration must not be negative".into(),
            ));
        }
        Ok(Some(duration))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_exposes_vocabulary_without_involving_the_parser() {
        let registry = LoweringRegistry::stdlib();
        assert!(registry.handles_invocation("title"));
        assert!(registry.handles_invocation("overlay-preset"));
        assert!(!registry.handles_invocation("scene-name"));
    }

    #[test]
    fn every_runtime_builtin_prefers_the_wasm_component() {
        let registry = LoweringRegistry::stdlib();
        let builtin_names = registry
            .builtins
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>();
        assert_eq!(builtin_names, WASM_SCENE_COMMANDS);
        assert!(WASM_SEQUENCE_COMMANDS.contains(&"gap"));
    }
}

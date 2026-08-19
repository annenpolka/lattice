use crate::builtins::{apply_commentary, lower_freeze, lower_title};
use crate::view::{InvocationView, LoweringError, SceneDraft};

type Builtin = fn(&InvocationView, &mut SceneDraft) -> Result<(), LoweringError>;

/// Command → lowering function. Parser does not consult this map.
pub struct LoweringRegistry {
    builtins: Vec<(&'static str, Builtin)>,
}

impl LoweringRegistry {
    pub fn stdlib() -> Self {
        Self {
            builtins: vec![("freeze", lower_freeze), ("title", lower_title)],
        }
    }

    pub fn lower(&self, inv: &InvocationView, draft: &mut SceneDraft) -> Result<(), LoweringError> {
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

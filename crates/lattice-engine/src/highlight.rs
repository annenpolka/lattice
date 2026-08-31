use lattice_core::Span;

use crate::Engine;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VelHighlightClass {
    Declaration,
    Keyword,
    Builtin,
    Identifier,
    String,
    Number,
    Comment,
    Punctuation,
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VelHighlight {
    pub class: VelHighlightClass,
    pub text: String,
    pub span: Span,
}

impl Engine {
    /// Highlight a possibly incomplete VEL draft.
    ///
    /// The VEL crate owns syntax classification; only registry vocabulary is
    /// added here so Studio does not maintain a second builtin list.
    #[must_use]
    pub fn highlight_vel(&self, source: &str) -> Vec<VelHighlight> {
        lattice_vel::highlight(source)
            .into_iter()
            .map(|token| {
                let class = match token.class {
                    lattice_vel::SyntaxClass::Declaration => VelHighlightClass::Declaration,
                    lattice_vel::SyntaxClass::Keyword => VelHighlightClass::Keyword,
                    lattice_vel::SyntaxClass::Invocation
                        if self.registry.handles_invocation(&token.text) =>
                    {
                        VelHighlightClass::Builtin
                    }
                    lattice_vel::SyntaxClass::Invocation | lattice_vel::SyntaxClass::Identifier => {
                        VelHighlightClass::Identifier
                    }
                    lattice_vel::SyntaxClass::String => VelHighlightClass::String,
                    lattice_vel::SyntaxClass::Number => VelHighlightClass::Number,
                    lattice_vel::SyntaxClass::Comment => VelHighlightClass::Comment,
                    lattice_vel::SyntaxClass::Punctuation => VelHighlightClass::Punctuation,
                    lattice_vel::SyntaxClass::Invalid => VelHighlightClass::Invalid,
                };
                VelHighlight {
                    class,
                    text: token.text,
                    span: token.span,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stdlib_registry_distinguishes_builtins_from_generic_invocations() {
        let tokens = Engine::default().highlight_vel(
            "scene intro {\n  title \"Hello\" { at 1s for 2s }\n  custom thing\n}\n",
        );
        let class_of = |text: &str| {
            tokens
                .iter()
                .find(|token| token.text == text)
                .map(|token| token.class)
        };

        assert_eq!(class_of("scene"), Some(VelHighlightClass::Declaration));
        assert_eq!(class_of("title"), Some(VelHighlightClass::Builtin));
        assert_eq!(
            Engine::default()
                .highlight_vel("sequence main { a\n gap 500ms\n b }")
                .into_iter()
                .find(|token| token.text == "gap")
                .map(|token| token.class),
            Some(VelHighlightClass::Builtin)
        );
        assert_eq!(class_of("custom"), Some(VelHighlightClass::Identifier));
        assert_eq!(class_of("at"), Some(VelHighlightClass::Keyword));
    }
}

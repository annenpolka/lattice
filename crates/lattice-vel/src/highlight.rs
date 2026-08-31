use lattice_core::Span;

use crate::lexer::{TokenKind, lex_lossy};
use crate::parse::{DECLARATIONS, MODIFIERS};

/// Parser-owned lexical and syntactic roles used by editor projections.
///
/// `Invocation` is intentionally generic. The VEL crate does not decide
/// whether an invocation is a stdlib builtin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyntaxClass {
    Declaration,
    Keyword,
    Invocation,
    Identifier,
    String,
    Number,
    Comment,
    Punctuation,
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxToken {
    pub class: SyntaxClass,
    pub text: String,
    pub span: Span,
}

/// Produce byte-based highlighting spans without requiring a valid document.
#[must_use]
pub fn highlight(source: &str) -> Vec<SyntaxToken> {
    let mut at_item_start = true;
    let mut highlighted = Vec::new();

    for token in lex_lossy(source) {
        let class = match token.kind {
            TokenKind::Eof => break,
            TokenKind::Newline => {
                at_item_start = true;
                continue;
            }
            TokenKind::Comment => SyntaxClass::Comment,
            TokenKind::String => {
                at_item_start = false;
                SyntaxClass::String
            }
            TokenKind::Number | TokenKind::Size | TokenKind::Unit => {
                at_item_start = false;
                SyntaxClass::Number
            }
            TokenKind::Ident => {
                let text = token.text.as_str();
                let class = if at_item_start && DECLARATIONS.contains(&text) {
                    SyntaxClass::Declaration
                } else if MODIFIERS.contains(&text) {
                    SyntaxClass::Keyword
                } else if at_item_start {
                    SyntaxClass::Invocation
                } else {
                    SyntaxClass::Identifier
                };
                at_item_start = false;
                class
            }
            TokenKind::LBrace => {
                at_item_start = true;
                SyntaxClass::Punctuation
            }
            TokenKind::RBrace
            | TokenKind::LBracket
            | TokenKind::RBracket
            | TokenKind::LParen
            | TokenKind::RParen
            | TokenKind::Comma
            | TokenKind::Eq
            | TokenKind::Dot
            | TokenKind::DotDot
            | TokenKind::Minus
            | TokenKind::Plus => SyntaxClass::Punctuation,
            TokenKind::Invalid => SyntaxClass::Invalid,
        };
        highlighted.push(SyntaxToken {
            class,
            text: token.text,
            span: token.span,
        });
    }

    highlighted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_fixture_roles_and_continues_after_an_error() {
        let source = concat!(
            "project \"demo\"\n",
            "scene intro {\n",
            "  title \"Hello\" { at 1s for 2s } // overlay\n",
            "  @\n",
            "  callout \"Recovered\"\n",
            "}\n",
        );
        let tokens = highlight(source);
        let class_of = |text: &str| {
            tokens
                .iter()
                .find(|token| token.text == text)
                .map(|token| token.class)
        };

        assert_eq!(class_of("project"), Some(SyntaxClass::Declaration));
        assert_eq!(class_of("scene"), Some(SyntaxClass::Declaration));
        assert_eq!(class_of("title"), Some(SyntaxClass::Invocation));
        assert_eq!(class_of("at"), Some(SyntaxClass::Keyword));
        assert_eq!(class_of("1s"), Some(SyntaxClass::Number));
        assert_eq!(class_of("// overlay"), Some(SyntaxClass::Comment));
        assert_eq!(class_of("@"), Some(SyntaxClass::Invalid));
        assert_eq!(class_of("callout"), Some(SyntaxClass::Invocation));
    }
}

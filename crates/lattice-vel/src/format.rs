use crate::error::ParseError;
use crate::lexer::{Token, TokenKind, lex_with_comments};

/// Formats valid VEL using only generic syntax tokens.
///
/// Invocation names remain opaque, string lexemes and comments are preserved,
/// and invalid input is rejected before any formatted text is returned.
pub fn format(source: &str) -> Result<String, ParseError> {
    crate::parse(source)?;
    let tokens = lex_with_comments(source)?;
    Ok(Formatter::new().format(&tokens))
}

struct Formatter {
    output: String,
    indent: usize,
    at_line_start: bool,
    pending_breaks: usize,
    previous: Option<Token>,
}

impl Formatter {
    fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
            at_line_start: true,
            pending_breaks: 0,
            previous: None,
        }
    }

    fn format(mut self, tokens: &[Token]) -> String {
        for token in tokens {
            match token.kind {
                TokenKind::Eof => break,
                TokenKind::Newline => self.request_breaks(line_breaks(&token.text)),
                TokenKind::LBrace => self.open_block(token),
                TokenKind::RBrace => self.close_block(token),
                TokenKind::Comment => self.comment(token),
                TokenKind::Invalid => unreachable!("strict parse rejects invalid tokens"),
                _ => self.token(token),
            }
        }
        while self.output.ends_with(char::is_whitespace) {
            self.output.pop();
        }
        if !self.output.is_empty() {
            self.output.push('\n');
        }
        self.output
    }

    fn open_block(&mut self, token: &Token) {
        self.flush_breaks();
        let needs_space = self.needs_space(token);
        self.indent_line();
        if needs_space {
            self.output.push(' ');
        }
        self.output.push('{');
        self.indent += 1;
        self.request_breaks(1);
        self.previous = Some(token.clone());
    }

    fn close_block(&mut self, token: &Token) {
        self.indent = self.indent.saturating_sub(1);
        self.pending_breaks = 1;
        self.flush_breaks();
        self.indent_line();
        self.output.push('}');
        self.request_breaks(1);
        self.previous = Some(token.clone());
    }

    fn comment(&mut self, token: &Token) {
        self.flush_breaks();
        if self.at_line_start {
            self.indent_line();
        } else {
            self.output.push_str("  ");
        }
        self.output.push_str(token.text.trim_end());
        self.request_breaks(1);
        self.previous = Some(token.clone());
    }

    fn token(&mut self, token: &Token) {
        self.flush_breaks();
        let needs_space = self.needs_space(token);
        self.indent_line();
        if needs_space {
            self.output.push(' ');
        }
        self.output.push_str(&token.text);
        self.previous = Some(token.clone());
    }

    fn request_breaks(&mut self, count: usize) {
        if !self.output.is_empty() {
            self.pending_breaks = self.pending_breaks.max(count.clamp(1, 2));
        }
    }

    fn flush_breaks(&mut self) {
        if self.pending_breaks == 0 {
            return;
        }
        self.output
            .extend(std::iter::repeat_n('\n', self.pending_breaks));
        self.pending_breaks = 0;
        self.at_line_start = true;
    }

    fn indent_line(&mut self) {
        if self.at_line_start {
            self.output.push_str(&"  ".repeat(self.indent));
            self.at_line_start = false;
        }
    }

    fn needs_space(&self, token: &Token) -> bool {
        if self.at_line_start {
            return false;
        }
        let Some(previous) = self.previous.as_ref() else {
            return false;
        };
        match token.kind {
            TokenKind::RBracket
            | TokenKind::RParen
            | TokenKind::Comma
            | TokenKind::Dot
            | TokenKind::DotDot
            | TokenKind::LBracket => false,
            TokenKind::LBrace | TokenKind::LParen | TokenKind::Eq => true,
            TokenKind::Minus | TokenKind::Plus => expression_ends(previous.kind),
            _ if previous.kind == TokenKind::Number
                && token.kind == TokenKind::Number
                && previous.text.ends_with('m')
                && token.text.ends_with('s') =>
            {
                false
            }
            _ => !matches!(
                previous.kind,
                TokenKind::LParen
                    | TokenKind::LBracket
                    | TokenKind::Dot
                    | TokenKind::DotDot
                    | TokenKind::Minus
                    | TokenKind::Plus
            ),
        }
    }
}

fn expression_ends(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Ident
            | TokenKind::String
            | TokenKind::Number
            | TokenKind::Unit
            | TokenKind::Size
            | TokenKind::RBracket
            | TokenKind::RParen
    )
}

fn line_breaks(text: &str) -> usize {
    let line_feeds = text.bytes().filter(|byte| *byte == b'\n').count();
    if line_feeds == 0 {
        1
    } else {
        line_feeds.min(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_generic_invocations_and_is_idempotent() {
        let source = "project   \"demo\"\nscene demo{game [ 26m14s .. 26m22s]as clip\ntitle  \"Hello\"{at 2s for 3s\nopacity 90}}";
        let expected = r#"project "demo"
scene demo {
  game[26m14s..26m22s] as clip
  title "Hello" {
    at 2s for 3s
    opacity 90
  }
}
"#;
        let formatted = format(source).unwrap();
        assert_eq!(formatted, expected);
        assert_eq!(format(&formatted).unwrap(), formatted);
    }

    #[test]
    fn preserves_comments_and_string_lexemes() {
        let source = "// header\nscene x { title \"a\\\\b\\n\\\"c\" // inline\n}\n";
        let formatted = format(source).unwrap();
        assert!(formatted.starts_with("// header\nscene x {"));
        assert!(formatted.contains(r#""a\\b\n\"c""#));
        assert!(formatted.contains("  // inline"));
        assert_eq!(format(&formatted).unwrap(), formatted);
    }

    #[test]
    fn rejects_invalid_vel() {
        assert!(format("scene x { title @ }").is_err());
    }
}

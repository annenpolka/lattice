use lattice_core::Span;

use crate::error::ParseError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Ident,
    String,
    Number,
    Unit,
    Size,
    Newline,
    DotDot,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Comma,
    Eq,
    Dot,
    Minus,
    Plus,
    Eof,
}

pub fn lex(source: &str) -> Result<Vec<Token>, ParseError> {
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token()?;
        let eof = token.kind == TokenKind::Eof;
        tokens.push(token);
        if eof {
            break;
        }
    }
    Ok(tokens)
}

struct Lexer<'src> {
    source: &'src str,
    bytes: &'src [u8],
    pos: usize,
    line: u32,
    column: u32,
}

impl<'src> Lexer<'src> {
    fn new(source: &'src str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    fn next_token(&mut self) -> Result<Token, ParseError> {
        self.skip_spaces_and_comments();
        let start = self.mark();
        if self.eof() {
            return Ok(self.token(TokenKind::Eof, start, self.pos));
        }
        let b = self.bytes[self.pos];
        match b {
            b'\n' => {
                self.bump();
                while self.peek() == Some(b'\n') || self.peek() == Some(b'\r') {
                    if self.peek() == Some(b'\r') {
                        self.bump();
                        continue;
                    }
                    self.bump();
                }
                Ok(self.token(TokenKind::Newline, start, self.pos))
            }
            b'\r' => {
                self.bump();
                if self.peek() == Some(b'\n') {
                    self.bump();
                }
                Ok(self.token(TokenKind::Newline, start, self.pos))
            }
            b'{' => Ok(self.simple(TokenKind::LBrace, start)),
            b'}' => Ok(self.simple(TokenKind::RBrace, start)),
            b'[' => Ok(self.simple(TokenKind::LBracket, start)),
            b']' => Ok(self.simple(TokenKind::RBracket, start)),
            b'(' => Ok(self.simple(TokenKind::LParen, start)),
            b')' => Ok(self.simple(TokenKind::RParen, start)),
            b',' => Ok(self.simple(TokenKind::Comma, start)),
            b'=' => Ok(self.simple(TokenKind::Eq, start)),
            b'+' => Ok(self.simple(TokenKind::Plus, start)),
            b'-' => Ok(self.simple(TokenKind::Minus, start)),
            b'.' => {
                self.bump();
                if self.peek() == Some(b'.') {
                    self.bump();
                    Ok(self.token(TokenKind::DotDot, start, self.pos))
                } else {
                    Ok(self.token(TokenKind::Dot, start, self.pos))
                }
            }
            b'"' => self.string(start),
            b'0'..=b'9' => Ok(self.number_or_size(start)),
            b'%' => {
                self.bump();
                Ok(self.token(TokenKind::Unit, start, self.pos))
            }
            _ if is_ident_start(b) => {
                while self.peek().is_some_and(is_ident_continue) {
                    self.bump();
                }
                Ok(self.token(TokenKind::Ident, start, self.pos))
            }
            _ => Err(ParseError::new(
                format!(
                    "unexpected character {:?}",
                    self.source[self.pos..].chars().next()
                ),
                self.span_at(start),
            )),
        }
    }

    fn skip_spaces_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(b' ' | b'\t') => {
                    self.bump();
                }
                Some(b'/') if self.peek_at(1) == Some(b'/') => {
                    while let Some(b) = self.peek() {
                        if b == b'\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                _ => break,
            }
        }
    }

    fn string(&mut self, start: Mark) -> Result<Token, ParseError> {
        let triple = self.remaining().starts_with("\"\"\"");
        if triple {
            self.bump();
            self.bump();
            self.bump();
            while !self.eof() && !self.remaining().starts_with("\"\"\"") {
                self.bump();
            }
            if self.remaining().starts_with("\"\"\"") {
                self.bump();
                self.bump();
                self.bump();
                return Ok(self.token(TokenKind::String, start, self.pos));
            }
            return Err(ParseError::new("unterminated string", self.span_at(start)));
        }
        self.bump();
        while let Some(b) = self.peek() {
            if b == b'"' {
                self.bump();
                return Ok(self.token(TokenKind::String, start, self.pos));
            }
            if b == b'\n' {
                break;
            }
            if b == b'\\' {
                self.bump();
            }
            self.bump();
        }
        Err(ParseError::new("unterminated string", self.span_at(start)))
    }

    fn number_or_size(&mut self, start: Mark) -> Token {
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.bump();
        }
        if self.peek() == Some(b'.') && self.peek_at(1).is_some_and(|b| b.is_ascii_digit()) {
            self.bump();
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.bump();
            }
        }
        if self.peek() == Some(b'x') && self.peek_at(1).is_some_and(|b| b.is_ascii_digit()) {
            self.bump();
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.bump();
            }
            return self.token(TokenKind::Size, start, self.pos);
        }
        if self.peek() == Some(b'%') {
            self.bump();
            return self.token(TokenKind::Number, start, self.pos);
        }
        if self.peek().is_some_and(|b| b.is_ascii_alphabetic()) {
            while self.peek().is_some_and(|b| b.is_ascii_alphabetic()) {
                self.bump();
            }
            return self.token(TokenKind::Number, start, self.pos);
        }
        self.token(TokenKind::Number, start, self.pos)
    }

    fn simple(&mut self, kind: TokenKind, start: Mark) -> Token {
        self.bump();
        self.token(kind, start, self.pos)
    }

    fn token(&self, kind: TokenKind, start: Mark, end: usize) -> Token {
        Token {
            kind,
            text: self.source[start.pos..end].to_string(),
            span: Span::new(
                byte_offset(start.pos),
                byte_offset(end),
                start.line,
                start.column,
            ),
        }
    }

    fn span_at(&self, start: Mark) -> Span {
        Span::new(
            byte_offset(start.pos),
            byte_offset(self.pos),
            start.line,
            start.column,
        )
    }

    fn mark(&self) -> Mark {
        Mark {
            pos: self.pos,
            line: self.line,
            column: self.column,
        }
    }

    fn eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }

    fn remaining(&self) -> &str {
        &self.source[self.pos..]
    }

    fn bump(&mut self) {
        if self.eof() {
            return;
        }
        let ch = self.source[self.pos..].chars().next().unwrap_or('\0');
        let len = ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        self.pos += len;
    }
}

#[derive(Clone, Copy)]
struct Mark {
    pos: usize,
    line: u32,
    column: u32,
}

fn byte_offset(pos: usize) -> u32 {
    u32::try_from(pos).unwrap_or(u32::MAX)
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_compound_time_and_string() {
        let tokens = lex(r#"game[26m14s..26m22s] "こんにちは""#).unwrap();
        let texts: Vec<_> = tokens.iter().map(|t| t.text.as_str()).collect();
        assert!(texts.contains(&"26m"));
        assert!(texts.contains(&"14s"));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::DotDot));
        assert!(
            tokens
                .iter()
                .any(|t| t.kind == TokenKind::String && t.text.contains("こんにちは"))
        );
    }
}

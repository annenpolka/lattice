use lattice_core::Span;

use crate::ast::{
    Block, Document, Expr, Invocation, Item, MediaKind, Modifier, Quantity, TimeLiteral,
};
use crate::error::ParseError;
use crate::lexer::{Token, TokenKind, lex};

const DECLARATIONS: &[&str] = &[
    "project",
    "convention",
    "theme",
    "media",
    "image",
    "music",
    "sequence",
    "scene",
    "narration",
];

const MODIFIERS: &[&str] = &["at", "for", "over", "using", "as", "by", "from", "to"];

pub fn parse(source: &str) -> Result<Document, ParseError> {
    parse_file(source, "<input>")
}

pub fn parse_file(source: &str, _origin: &str) -> Result<Document, ParseError> {
    let tokens = lex(source)?;
    let mut parser = Parser { tokens, index: 0 };
    parser.parse_document()
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    fn parse_document(&mut self) -> Result<Document, ParseError> {
        let start = self.peek().span;
        let mut items = Vec::new();
        self.skip_newlines();
        while !self.at(TokenKind::Eof) {
            items.push(self.parse_item()?);
            self.skip_newlines();
        }
        let span = if let Some(last) = items.last() {
            start.merge(item_span(last))
        } else {
            start
        };
        Ok(Document { items, span })
    }

    fn parse_item(&mut self) -> Result<Item, ParseError> {
        if let TokenKind::Ident = self.peek().kind
            && DECLARATIONS.contains(&self.peek().text.as_str())
        {
            return self.parse_declaration();
        }
        if self.at_ident_named_modifier() {
            return self.parse_modifier_line();
        }
        self.parse_statement()
    }

    fn parse_declaration(&mut self) -> Result<Item, ParseError> {
        let keyword = self.expect_ident()?;
        match keyword.text.as_str() {
            "project" => {
                let name = self.expect_string()?;
                let body = self.optional_block()?;
                let span = merge_opt(keyword.span, body.as_ref().map(|b| b.span));
                Ok(Item::Project { name, body, span })
            }
            "convention" => {
                let name = self.expect_ident()?;
                Ok(Item::Convention {
                    name: name.text,
                    span: keyword.span.merge(name.span),
                })
            }
            "theme" => {
                let name = self.expect_string()?;
                let span = keyword.span.merge(self.previous_span());
                Ok(Item::Theme { name, span })
            }
            "media" | "image" | "music" => {
                let kind = match keyword.text.as_str() {
                    "image" => MediaKind::Image,
                    "music" => MediaKind::Music,
                    _ => MediaKind::Media,
                };
                let name = self.expect_ident()?;
                let path = self.expect_string()?;
                Ok(Item::Media {
                    kind,
                    name: name.text,
                    path,
                    span: keyword.span.merge(self.previous_span()),
                })
            }
            "sequence" => {
                let name = self.expect_ident()?;
                let body = self.expect_block()?;
                Ok(Item::Sequence {
                    name: name.text,
                    span: keyword.span.merge(body.span),
                    body,
                })
            }
            "scene" => {
                let name = self.expect_ident()?;
                let over = if self.at_ident("over") {
                    self.bump();
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                let body = self.expect_block()?;
                Ok(Item::Scene {
                    name: name.text,
                    over,
                    span: keyword.span.merge(body.span),
                    body,
                })
            }
            "narration" => {
                let body = self.expect_block()?;
                Ok(Item::Narration {
                    span: keyword.span.merge(body.span),
                    body,
                })
            }
            other => Err(ParseError::new(
                format!("unknown declaration `{other}`"),
                keyword.span,
            )),
        }
    }

    fn parse_modifier_line(&mut self) -> Result<Item, ParseError> {
        let mut modifiers = Vec::new();
        while self.at_ident_named_modifier() && !self.at(TokenKind::Newline) {
            modifiers.push(self.parse_modifier()?);
        }
        let span = modifiers
            .iter()
            .map(|m| m.span)
            .reduce(Span::merge)
            .unwrap_or_else(|| self.peek().span);
        Ok(Item::Modifiers { modifiers, span })
    }

    fn parse_statement(&mut self) -> Result<Item, ParseError> {
        let expr = self.parse_expr()?;
        if self.at_ident("as") {
            self.bump();
            let name = self.expect_ident()?;
            return Ok(Item::Binding {
                span: expr.span().merge(name.span),
                expr,
                name: name.text,
            });
        }
        if let Expr::Ident { name, span } = expr {
            return self.finish_invocation(name, span);
        }
        Err(ParseError::new(
            "expected invocation, binding, or declaration",
            expr.span(),
        ))
    }

    fn finish_invocation(&mut self, name: String, start: Span) -> Result<Item, ParseError> {
        let mut args = Vec::new();
        let mut modifiers = Vec::new();
        while !self.at_stmt_end() && !self.at(TokenKind::LBrace) {
            if self.at_ident_named_modifier() {
                modifiers.push(self.parse_modifier()?);
            } else {
                args.push(self.parse_expr()?);
            }
        }
        let body = self.optional_block()?;
        let end = body
            .as_ref()
            .map_or_else(|| self.previous_span(), |block| block.span);
        Ok(Item::Invocation(Invocation {
            name,
            args,
            modifiers,
            body,
            span: start.merge(end),
        }))
    }

    fn parse_modifier(&mut self) -> Result<Modifier, ParseError> {
        let name = self.expect_ident()?;
        let value = self.parse_expr()?;
        Ok(Modifier {
            name: name.text,
            span: name.span.merge(value.span()),
            value,
        })
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.at(TokenKind::LBracket) {
                let open = self.bump();
                let index = self.parse_range_or_expr()?;
                let close = self.expect(TokenKind::RBracket, "expected `]`")?;
                expr = Expr::Index {
                    span: open.span.merge(close.span),
                    target: Box::new(expr),
                    index: Box::new(index),
                };
            } else if self.at(TokenKind::Dot) {
                self.bump();
                let part = self.expect_ident()?;
                match expr {
                    Expr::Ident { name, span } => {
                        expr = Expr::Path {
                            parts: vec![name, part.text],
                            span: span.merge(part.span),
                        };
                    }
                    Expr::Path { mut parts, span } => {
                        parts.push(part.text);
                        expr = Expr::Path {
                            parts,
                            span: span.merge(part.span),
                        };
                    }
                    other => {
                        return Err(ParseError::new(
                            "`.` can only follow an identifier or path",
                            other.span(),
                        ));
                    }
                }
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_range_or_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.parse_timeish()?;
        if self.at(TokenKind::DotDot) {
            self.bump();
            let end = self.parse_timeish()?;
            return Ok(Expr::Range {
                span: start.span().merge(end.span()),
                start: Box::new(start),
                end: Box::new(end),
            });
        }
        Ok(start)
    }

    fn parse_timeish(&mut self) -> Result<Expr, ParseError> {
        if self.at_ident("end") {
            let tok = self.bump();
            return Ok(Expr::End { span: tok.span });
        }
        if self.at(TokenKind::Minus) {
            let minus = self.bump();
            let mut expr = self.parse_timeish()?;
            negate_time(&mut expr);
            expand_span_left(&mut expr, minus.span);
            return Ok(expr);
        }
        self.parse_expr()
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        if self.at(TokenKind::Minus) {
            let minus = self.bump();
            let mut expr = self.parse_primary()?;
            negate_time(&mut expr);
            expand_span_left(&mut expr, minus.span);
            return Ok(expr);
        }
        match self.peek().kind {
            TokenKind::String => {
                let tok = self.bump();
                Ok(Expr::String {
                    value: unquote(&tok.text),
                    span: tok.span,
                })
            }
            TokenKind::Ident => {
                let tok = self.bump();
                if tok.text == "end" {
                    Ok(Expr::End { span: tok.span })
                } else {
                    Ok(Expr::Ident {
                        name: tok.text,
                        span: tok.span,
                    })
                }
            }
            TokenKind::Number => self.parse_quantity_or_time(),
            TokenKind::LParen => {
                let open = self.bump();
                let mut items = Vec::new();
                if !self.at(TokenKind::RParen) {
                    loop {
                        items.push(self.parse_expr()?);
                        if self.at(TokenKind::Comma) {
                            self.bump();
                            continue;
                        }
                        break;
                    }
                }
                let close = self.expect(TokenKind::RParen, "expected `)`")?;
                Ok(Expr::Tuple {
                    items,
                    span: open.span.merge(close.span),
                })
            }
            _ => Err(ParseError::new(
                format!("expected expression, found {}", token_label(self.peek())),
                self.peek().span,
            )),
        }
    }

    fn parse_quantity_or_time(&mut self) -> Result<Expr, ParseError> {
        let first = self.parse_one_quantity()?;
        if first.is_time_unit()
            && first.unit.as_deref() == Some("m")
            && let Ok(second) = self.try_peek_seconds_quantity()
        {
            let span = first.span.merge(second.span);
            let seconds = quantity_to_seconds(&second);
            return Ok(Expr::Time(TimeLiteral::MinutesSeconds {
                minutes: signed_digits(&first),
                seconds: Box::new(seconds),
                span,
            }));
        }
        if first.is_time_unit() {
            return Ok(Expr::Time(quantity_to_seconds(&first)));
        }
        Ok(Expr::Quantity(first))
    }

    fn try_peek_seconds_quantity(&mut self) -> Result<Quantity, ParseError> {
        if !self.at(TokenKind::Number) {
            return Err(ParseError::new("not a quantity", self.peek().span));
        }
        let saved = self.index;
        let q = self.parse_one_quantity()?;
        if q.unit.as_deref() == Some("s") {
            Ok(q)
        } else {
            self.index = saved;
            Err(ParseError::new("not seconds", q.span))
        }
    }

    fn parse_one_quantity(&mut self) -> Result<Quantity, ParseError> {
        let tok = self.expect(TokenKind::Number, "expected number")?;
        let (digits, scale, unit) = split_number_lexeme(&tok.text);
        Ok(Quantity {
            negative: false,
            digits,
            scale,
            unit,
            span: tok.span,
        })
    }

    fn optional_block(&mut self) -> Result<Option<Block>, ParseError> {
        let saved = self.index;
        self.skip_newlines();
        if self.at(TokenKind::LBrace) {
            Ok(Some(self.parse_block()?))
        } else {
            self.index = saved;
            Ok(None)
        }
    }

    fn expect_block(&mut self) -> Result<Block, ParseError> {
        self.skip_newlines();
        self.parse_block()
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let open = self.expect(TokenKind::LBrace, "expected `{`")?;
        let mut items = Vec::new();
        self.skip_newlines();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            items.push(self.parse_item()?);
            self.skip_newlines();
        }
        let close = self.expect(TokenKind::RBrace, "expected `}`")?;
        Ok(Block {
            items,
            span: open.span.merge(close.span),
        })
    }

    fn skip_newlines(&mut self) {
        while self.at(TokenKind::Newline) {
            self.bump();
        }
    }

    fn at_stmt_end(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Newline | TokenKind::RBrace | TokenKind::Eof
        )
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }

    fn at_ident(&self, name: &str) -> bool {
        self.peek().kind == TokenKind::Ident && self.peek().text == name
    }

    fn at_ident_named_modifier(&self) -> bool {
        self.peek().kind == TokenKind::Ident && MODIFIERS.contains(&self.peek().text.as_str())
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.index.min(self.tokens.len() - 1)]
    }

    fn bump(&mut self) -> Token {
        let tok = self.peek().clone();
        if self.index < self.tokens.len() - 1 {
            self.index += 1;
        }
        tok
    }

    fn expect(&mut self, kind: TokenKind, message: &str) -> Result<Token, ParseError> {
        if self.peek().kind == kind {
            Ok(self.bump())
        } else {
            Err(ParseError::new(message, self.peek().span))
        }
    }

    fn expect_ident(&mut self) -> Result<Token, ParseError> {
        self.expect(TokenKind::Ident, "expected identifier")
    }

    fn expect_string(&mut self) -> Result<String, ParseError> {
        let tok = self.expect(TokenKind::String, "expected string")?;
        Ok(unquote(&tok.text))
    }

    fn previous_span(&self) -> Span {
        if self.index == 0 {
            self.peek().span
        } else {
            self.tokens[self.index - 1].span
        }
    }
}

fn item_span(item: &Item) -> Span {
    match item {
        Item::Project { span, .. }
        | Item::Convention { span, .. }
        | Item::Theme { span, .. }
        | Item::Media { span, .. }
        | Item::Sequence { span, .. }
        | Item::Scene { span, .. }
        | Item::Narration { span, .. }
        | Item::Binding { span, .. }
        | Item::Modifiers { span, .. } => *span,
        Item::Invocation(inv) => inv.span,
    }
}

fn merge_opt(start: Span, extra: Option<Span>) -> Span {
    extra.map_or(start, |span| start.merge(span))
}

fn token_label(token: &Token) -> String {
    if token.text.is_empty() {
        format!("{:?}", token.kind)
    } else {
        format!("`{}`", token.text)
    }
}

fn unquote(lexeme: &str) -> String {
    if let Some(inner) = lexeme
        .strip_prefix("\"\"\"")
        .and_then(|s| s.strip_suffix("\"\"\""))
    {
        return inner.to_string();
    }
    if let Some(inner) = lexeme.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return inner.replace("\\\"", "\"").replace("\\n", "\n");
    }
    lexeme.to_string()
}

fn split_number_lexeme(text: &str) -> (i64, u32, Option<String>) {
    let unit_at = text
        .char_indices()
        .find(|&(_, ch)| ch.is_ascii_alphabetic() || ch == '%')
        .map(|(i, _)| i);
    let (number, unit) = match unit_at {
        Some(i) => (&text[..i], Some(text[i..].to_string())),
        None => (text, None),
    };
    if let Some((whole, frac)) = number.split_once('.') {
        let digits = format!("{whole}{frac}");
        let value = digits.parse::<i64>().unwrap_or(0);
        (value, u32::try_from(frac.len()).unwrap_or(0), unit)
    } else {
        let value = number.parse::<i64>().unwrap_or(0);
        (value, 0, unit)
    }
}

fn signed_digits(q: &Quantity) -> i64 {
    if q.negative { -q.digits } else { q.digits }
}

fn quantity_to_seconds(q: &Quantity) -> TimeLiteral {
    match q.unit.as_deref() {
        Some("ms") => TimeLiteral::Milliseconds {
            value: if q.negative { -q.digits } else { q.digits },
            span: q.span,
        },
        Some("f") => TimeLiteral::Frames {
            frames: if q.negative { -q.digits } else { q.digits },
            span: q.span,
        },
        _ => TimeLiteral::Seconds {
            negative: q.negative,
            digits: q.digits,
            scale: q.scale,
            span: q.span,
        },
    }
}

fn negate_time(expr: &mut Expr) {
    match expr {
        Expr::Quantity(q) => q.negative = !q.negative,
        Expr::Time(TimeLiteral::Seconds { negative, .. }) => *negative = !*negative,
        Expr::Time(TimeLiteral::Milliseconds { value, .. }) => *value = -*value,
        Expr::Time(TimeLiteral::Frames { frames, .. }) => *frames = -*frames,
        Expr::Time(TimeLiteral::MinutesSeconds { minutes, .. }) => *minutes = -*minutes,
        _ => {}
    }
}

fn expand_span_left(expr: &mut Expr, left: Span) {
    match expr {
        Expr::Quantity(q) => q.span = left.merge(q.span),
        Expr::Time(
            TimeLiteral::Seconds { span, .. }
            | TimeLiteral::Milliseconds { span, .. }
            | TimeLiteral::MinutesSeconds { span, .. }
            | TimeLiteral::Frames { span, .. },
        )
        | Expr::String { span, .. }
        | Expr::Ident { span, .. }
        | Expr::Path { span, .. }
        | Expr::Range { span, .. }
        | Expr::Index { span, .. }
        | Expr::Tuple { span, .. }
        | Expr::End { span } => *span = left.merge(*span),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEMO: &str = r#"
project "demo"

convention commentary

media game "capture.mp4"

sequence main {
  demo
}

scene demo {
  game[10s..20s] as fight

  freeze fight at 5.2s for 1.5s

  title "Hello" {
    at 2s for 3s
  }
}
"#;

    #[test]
    fn parses_walking_skeleton() {
        let doc = parse(DEMO).unwrap();
        assert!(
            doc.items
                .iter()
                .any(|item| matches!(item, Item::Project { name, .. } if name == "demo"))
        );
        let scene = doc
            .items
            .iter()
            .find_map(|item| match item {
                Item::Scene { name, body, .. } if name == "demo" => Some(body),
                _ => None,
            })
            .unwrap();
        assert!(
            scene
                .items
                .iter()
                .any(|item| matches!(item, Item::Binding { name, .. } if name == "fight"))
        );
        assert!(scene.items.iter().any(|item| matches!(
            item,
            Item::Invocation(inv) if inv.name == "freeze"
        )));
        assert!(scene.items.iter().any(|item| matches!(
            item,
            Item::Invocation(inv) if inv.name == "title"
        )));
    }

    #[test]
    fn parses_compound_media_range() {
        let doc = parse("scene hook { game[26m14s..26m22s] as clip }").unwrap();
        let Item::Scene { body, .. } = &doc.items[0] else {
            panic!("expected scene");
        };
        let Item::Binding { expr, .. } = &body.items[0] else {
            panic!("expected binding");
        };
        let Expr::Index { index, .. } = expr else {
            panic!("expected index");
        };
        let Expr::Range { start, end, .. } = index.as_ref() else {
            panic!("expected range");
        };
        assert!(matches!(
            start.as_ref(),
            Expr::Time(TimeLiteral::MinutesSeconds { minutes: 26, .. })
        ));
        assert!(matches!(
            end.as_ref(),
            Expr::Time(TimeLiteral::MinutesSeconds { minutes: 26, .. })
        ));
    }

    #[test]
    fn freeze_and_title_are_generic_invocations_not_ast_nodes() {
        let doc = parse(DEMO).unwrap();
        let mut freeze = None;
        let mut title = None;
        for item in &doc.items {
            let Item::Scene { body, .. } = item else {
                continue;
            };
            for inner in &body.items {
                if let Item::Invocation(inv) = inner {
                    if inv.name == "freeze" {
                        freeze = Some(inv);
                    }
                    if inv.name == "title" {
                        title = Some(inv);
                    }
                }
            }
        }
        let freeze = freeze.expect("freeze is a generic invocation");
        assert!(
            freeze.modifiers.iter().any(|m| m.name == "at"),
            "parser keeps `at` as a modifier, not freeze semantics"
        );
        let title = title.expect("title is a generic invocation");
        assert!(matches!(title.args.first(), Some(Expr::String { value, .. }) if value == "Hello"));
    }

    #[test]
    fn caption_is_a_generic_invocation_not_an_ast_node() {
        let doc = parse(r#"scene x { caption "cue" at 1s for 2s }"#).unwrap();
        let Item::Scene { body, .. } = &doc.items[0] else {
            panic!("expected scene");
        };
        let Item::Invocation(inv) = &body.items[0] else {
            panic!("expected generic invocation");
        };
        assert_eq!(inv.name, "caption");
        assert!(matches!(inv.args.first(), Some(Expr::String { value, .. }) if value == "cue"));
        assert!(inv.modifiers.iter().any(|m| m.name == "at"));
        assert!(inv.modifiers.iter().any(|m| m.name == "for"));
        assert!(inv.body.is_none(), "body-less cue keeps at/for inline");
    }

    #[test]
    fn parses_negative_quantity_as_generic_expr() {
        let doc = parse(r"scene x { gain fight by -3 }").unwrap();
        let Item::Scene { body, .. } = &doc.items[0] else {
            panic!("expected scene");
        };
        let Item::Invocation(inv) = &body.items[0] else {
            panic!("expected invocation");
        };
        assert_eq!(inv.name, "gain");
        let by = inv
            .modifiers
            .iter()
            .find(|m| m.name == "by")
            .expect("parser keeps `by` as a modifier");
        assert!(
            matches!(
                &by.value,
                Expr::Quantity(q) if q.negative && q.digits == 3
            ),
            "unary minus is syntax, not gain semantics: {:?}",
            by.value
        );
        let Expr::Quantity(q) = &by.value else {
            panic!("expected quantity");
        };
        let lexeme = &r"scene x { gain fight by -3 }"[q.span.start as usize..q.span.end as usize];
        assert_eq!(
            lexeme, "-3",
            "quantity span must include the unary minus so splices cannot write `--3`"
        );
    }

    #[test]
    fn parses_japanese_string() {
        let doc = parse(r#"scene x { title "この数字" }"#).unwrap();
        let Item::Scene { body, .. } = &doc.items[0] else {
            panic!("expected scene");
        };
        let Item::Invocation(inv) = &body.items[0] else {
            panic!("expected title");
        };
        assert!(matches!(
            &inv.args[0],
            Expr::String { value, .. } if value == "この数字"
        ));
    }
}

//! VEL surface parser.
//!
//! Parses a generic invocation DSL. Command meaning (`freeze`, `title`, …)
//! lives in `lattice-wasm`, not here.

mod ast;
mod error;
mod format;
mod highlight;
mod lexer;
mod parse;

pub use ast::{
    Block, Document, Expr, Invocation, Item, MediaKind, Modifier, Quantity, TimeLiteral,
};
pub use error::ParseError;
pub use format::format;
pub use highlight::{SyntaxClass, SyntaxToken, highlight};
pub use parse::parse;
pub use parse::parse_file;

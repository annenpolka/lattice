use serde::{Deserialize, Serialize};

use crate::span::Span;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Origin {
    Source,
    Invocation { command: String },
    Convention { name: String },
    Builtin { name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub span: Option<Span>,
    pub origin: Origin,
}

impl Provenance {
    pub fn source(span: Span) -> Self {
        Self {
            span: Some(span),
            origin: Origin::Source,
        }
    }

    pub fn invocation(command: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            span,
            origin: Origin::Invocation {
                command: command.into(),
            },
        }
    }

    pub fn convention(name: impl Into<String>) -> Self {
        Self {
            span: None,
            origin: Origin::Convention { name: name.into() },
        }
    }
}

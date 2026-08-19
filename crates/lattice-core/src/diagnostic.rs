use serde::{Deserialize, Serialize};

use crate::span::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub span: Option<Span>,
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Error,
            message: message.into(),
            span,
        }
    }

    pub fn warning(
        code: impl Into<String>,
        message: impl Into<String>,
        span: Option<Span>,
    ) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Warning,
            message: message.into(),
            span,
        }
    }
}

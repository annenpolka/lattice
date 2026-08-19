use serde::{Deserialize, Serialize};

/// Byte range in a source file. Core does not interpret VEL; it only stores the range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    pub start: u32,
    pub end: u32,
    pub line: u32,
    pub column: u32,
}

impl Span {
    pub const fn new(start: u32, end: u32, line: u32, column: u32) -> Self {
        Self {
            start,
            end,
            line,
            column,
        }
    }

    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            line: self.line.min(other.line),
            column: if self.start <= other.start {
                self.column
            } else {
                other.column
            },
        }
    }
}

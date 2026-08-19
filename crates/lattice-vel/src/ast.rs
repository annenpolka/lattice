use lattice_core::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    Project {
        name: String,
        body: Option<Block>,
        span: Span,
    },
    Convention {
        name: String,
        span: Span,
    },
    Theme {
        name: String,
        span: Span,
    },
    Media {
        kind: MediaKind,
        name: String,
        path: String,
        span: Span,
    },
    Sequence {
        name: String,
        body: Block,
        span: Span,
    },
    Scene {
        name: String,
        over: Option<Expr>,
        body: Block,
        span: Span,
    },
    Narration {
        body: Block,
        span: Span,
    },
    Binding {
        expr: Expr,
        name: String,
        span: Span,
    },
    Invocation(Invocation),
    Modifiers {
        modifiers: Vec<Modifier>,
        span: Span,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKind {
    Media,
    Image,
    Music,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invocation {
    pub name: String,
    pub args: Vec<Expr>,
    pub modifiers: Vec<Modifier>,
    pub body: Option<Block>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Modifier {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    String {
        value: String,
        span: Span,
    },
    Ident {
        name: String,
        span: Span,
    },
    Path {
        parts: Vec<String>,
        span: Span,
    },
    Quantity(Quantity),
    Time(TimeLiteral),
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        span: Span,
    },
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    Tuple {
        items: Vec<Expr>,
        span: Span,
    },
    End {
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Self::String { span, .. }
            | Self::Ident { span, .. }
            | Self::Path { span, .. }
            | Self::Range { span, .. }
            | Self::Index { span, .. }
            | Self::Tuple { span, .. }
            | Self::End { span } => *span,
            Self::Quantity(q) => q.span,
            Self::Time(t) => t.span(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quantity {
    pub negative: bool,
    pub digits: i64,
    pub scale: u32,
    pub unit: Option<String>,
    pub span: Span,
}

impl Quantity {
    pub fn is_time_unit(&self) -> bool {
        matches!(self.unit.as_deref(), Some("s" | "ms" | "m" | "f"))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimeLiteral {
    Seconds {
        negative: bool,
        digits: i64,
        scale: u32,
        span: Span,
    },
    Milliseconds {
        value: i64,
        span: Span,
    },
    MinutesSeconds {
        minutes: i64,
        seconds: Box<TimeLiteral>,
        span: Span,
    },
    Frames {
        frames: i64,
        span: Span,
    },
}

impl TimeLiteral {
    pub fn span(&self) -> Span {
        match self {
            Self::Seconds { span, .. }
            | Self::Milliseconds { span, .. }
            | Self::MinutesSeconds { span, .. }
            | Self::Frames { span, .. } => *span,
        }
    }
}

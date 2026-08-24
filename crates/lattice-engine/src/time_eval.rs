use lattice_core::{Time, TimeError};
use lattice_vel::{Expr, Quantity, TimeLiteral};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TimeEvalError {
    #[error(transparent)]
    Time(#[from] TimeError),
    #[error("{0}")]
    Message(String),
}

pub fn expr_time(expr: &Expr) -> Result<Time, TimeEvalError> {
    match expr {
        Expr::Time(literal) => literal_time(literal),
        Expr::Quantity(q) if q.is_time_unit() => quantity_time(q),
        other => Err(TimeEvalError::Message(format!(
            "expected a time, got {}",
            expr_kind(other)
        ))),
    }
}

pub fn literal_time(literal: &TimeLiteral) -> Result<Time, TimeEvalError> {
    match literal {
        TimeLiteral::Seconds {
            negative,
            digits,
            scale,
            ..
        } => {
            let whole_scale = 10i64.checked_pow(*scale).ok_or(TimeError::Overflow)?;
            let whole = digits / whole_scale;
            let frac = digits % whole_scale;
            let mut t = Time::from_decimal_seconds(whole, frac, *scale)?;
            if *negative {
                t = Time::ZERO.checked_sub(t)?;
            }
            Ok(t)
        }
        TimeLiteral::Milliseconds { value, .. } => Ok(Time::milliseconds(*value)),
        TimeLiteral::MinutesSeconds {
            minutes, seconds, ..
        } => Time::from_minutes_seconds(*minutes, literal_time(seconds)?).map_err(Into::into),
        TimeLiteral::Frames { frames, .. } => {
            // Sequence fps is not a Core primitive yet; v0 assumes 60.
            Time::from_frames(*frames, 60, 1).map_err(Into::into)
        }
    }
}

fn quantity_time(q: &Quantity) -> Result<Time, TimeEvalError> {
    literal_time(&match q.unit.as_deref() {
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
    })
}

pub fn expr_kind(expr: &Expr) -> &'static str {
    match expr {
        Expr::String { .. } => "string",
        Expr::Ident { .. } => "ident",
        Expr::Path { .. } => "path",
        Expr::Quantity(_) => "quantity",
        Expr::Time(_) => "time",
        Expr::Range { .. } => "range",
        Expr::Index { .. } => "index",
        Expr::Tuple { .. } => "tuple",
        Expr::End { .. } => "end",
    }
}

pub fn expr_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident { name, .. } => Some(name.clone()),
        Expr::Path { parts, .. } => Some(parts.join(".")),
        _ => None,
    }
}

pub fn range_times(expr: &Expr) -> Result<(Time, Time), TimeEvalError> {
    match expr {
        Expr::Range { start, end, .. } => Ok((expr_time(start)?, expr_time(end)?)),
        _ => Err(TimeEvalError::Message("expected a time range".into())),
    }
}

use lattice_vel::{Block, Expr, Item};
use lattice_wasm::{BodyItem, InvocationView, ValueView};

use crate::time_eval::{TimeEvalError, expr_time};

pub fn invocation_view(inv: &lattice_vel::Invocation) -> Result<InvocationView, TimeEvalError> {
    Ok(InvocationView {
        command: inv.name.clone(),
        args: inv
            .args
            .iter()
            .map(value_view)
            .collect::<Result<Vec<_>, _>>()?,
        modifiers: inv
            .modifiers
            .iter()
            .map(|m| Ok::<_, TimeEvalError>((m.name.clone(), value_view(&m.value)?)))
            .collect::<Result<Vec<_>, _>>()?,
        body: match &inv.body {
            Some(block) => body_items(block)?,
            None => Vec::new(),
        },
        span: inv.span,
    })
}

fn body_items(block: &Block) -> Result<Vec<BodyItem>, TimeEvalError> {
    let mut out = Vec::new();
    for item in &block.items {
        match item {
            Item::Modifiers { modifiers, .. } => {
                for modifier in modifiers {
                    out.push(BodyItem::Modifier {
                        name: modifier.name.clone(),
                        value: value_view(&modifier.value)?,
                    });
                }
            }
            Item::Invocation(inv) => out.push(BodyItem::Invocation(invocation_view(inv)?)),
            _ => {}
        }
    }
    Ok(out)
}

fn value_view(expr: &Expr) -> Result<ValueView, TimeEvalError> {
    match expr {
        Expr::String { value, .. } => Ok(ValueView::String(value.clone())),
        Expr::Ident { name, .. } => Ok(ValueView::Name(name.clone())),
        Expr::Path { parts, .. } => Ok(ValueView::Path(parts.clone())),
        Expr::Time(_) => Ok(ValueView::Time(expr_time(expr)?)),
        Expr::Quantity(q) if q.is_time_unit() => Ok(ValueView::Time(expr_time(expr)?)),
        Expr::Quantity(q) => Ok(ValueView::Quantity {
            negative: q.negative,
            digits: q.digits,
            scale: q.scale,
            unit: q.unit.clone(),
        }),
        Expr::End { .. } => Err(TimeEvalError::Message(
            "`end` is only valid inside a range".into(),
        )),
        other => Err(TimeEvalError::Message(format!(
            "unsupported argument {other:?}"
        ))),
    }
}

pub fn over_path(expr: &Expr) -> String {
    match expr {
        Expr::Ident { name, .. } => name.clone(),
        Expr::Path { parts, .. } => parts.join("."),
        _ => String::new(),
    }
}

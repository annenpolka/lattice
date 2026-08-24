use lattice_core::{
    Diagnostic, Media, NormalizedPosition, NormalizedScale, Origin, Placement, Scene, Source, Span,
    Time,
};
use thiserror::Error;

use crate::overlay_registry::OverlayPresetRegistry;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LoweringError {
    #[error("{0}")]
    Message(String),
}

#[derive(Clone, Debug)]
pub struct InvocationView {
    pub command: String,
    pub args: Vec<ValueView>,
    pub modifiers: Vec<(String, ValueView)>,
    pub body: Vec<BodyItem>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum BodyItem {
    Modifier { name: String, value: ValueView },
    Invocation(InvocationView),
}

#[derive(Clone, Debug)]
pub enum ValueView {
    Name(String),
    String(String),
    Time(Time),
    Path(Vec<String>),
    Quantity {
        negative: bool,
        digits: i64,
        scale: u32,
        unit: Option<String>,
    },
    Tuple(Vec<ValueView>),
}

impl ValueView {
    pub fn as_name(&self) -> Option<&str> {
        match self {
            Self::Name(name) => Some(name),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_time(&self) -> Option<Time> {
        match self {
            Self::Time(t) => Some(*t),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Quantity {
                negative,
                digits,
                scale,
                ..
            } if *scale == 0 => Some(if *negative { -*digits } else { *digits }),
            _ => None,
        }
    }

    /// Read a generic `(x%, y%)` tuple as normalized Canvas Space.
    /// Vocabulary meaning remains in lowering; the VEL parser only supplies a tuple.
    pub fn as_normalized_position(&self) -> Option<NormalizedPosition> {
        let Self::Tuple(items) = self else {
            return None;
        };
        let [x, y] = items.as_slice() else {
            return None;
        };
        NormalizedPosition::new(percent_basis_points(x)?, percent_basis_points(y)?)
    }

    /// Read a generic percent as an aspect-preserving overlay scale.
    pub fn as_normalized_scale(&self) -> Option<NormalizedScale> {
        let basis_points = percent_basis_points(self)?;
        let milli = basis_points.saturating_add(5) / 10;
        NormalizedScale::new(milli)
    }
}

fn percent_basis_points(value: &ValueView) -> Option<u16> {
    let ValueView::Quantity {
        negative,
        digits,
        scale,
        unit: Some(unit),
    } = value
    else {
        return None;
    };
    if *negative || unit != "%" || *digits < 0 {
        return None;
    }
    let divisor = 10_i128.checked_pow(*scale)?;
    let numerator = i128::from(*digits).checked_mul(100)?;
    let rounded = numerator.checked_add(divisor / 2)?.checked_div(divisor)?;
    u16::try_from(rounded).ok()
}

impl InvocationView {
    pub fn modifier(&self, name: &str) -> Option<&ValueView> {
        self.modifiers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
            .or_else(|| {
                self.body.iter().find_map(|item| match item {
                    BodyItem::Modifier {
                        name: item_name,
                        value,
                    } if item_name == name => Some(value),
                    _ => None,
                })
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplainLine {
    pub origin: Origin,
    pub message: String,
}

#[derive(Clone, Debug, Default)]
pub struct SceneDraft {
    pub name: String,
    pub over: Option<String>,
    pub sources: Vec<Source>,
    pub placements: Vec<Placement>,
    pub media: Vec<Media>,
    pub source_fade_in: Vec<(String, Time)>,
    pub source_gain_db: Vec<(String, i32)>,
    pub explain: Vec<ExplainLine>,
    pub diagnostics: Vec<Diagnostic>,
    /// Document-scoped title presets. Core never sees the IDENT.
    pub overlay_presets: OverlayPresetRegistry,
}

impl SceneDraft {
    pub fn source_mut(&mut self, name: &str) -> Result<&mut Source, LoweringError> {
        self.sources
            .iter_mut()
            .find(|source| source.name == name)
            .ok_or_else(|| LoweringError::Message(format!("unknown source `{name}`")))
    }

    pub fn finish(self, id: String) -> Scene {
        let duration = scene_duration(&self);
        Scene {
            id,
            name: self.name,
            over: self.over,
            duration,
            sources: self.sources,
            placements: self.placements,
        }
    }
}

pub fn scene_duration(draft: &SceneDraft) -> Time {
    let from_sources = draft
        .sources
        .iter()
        .map(|source| source.time_map.duration)
        .max()
        .unwrap_or(Time::ZERO);
    let from_placements = draft
        .placements
        .iter()
        .map(|placement| placement.span.end())
        .max()
        .unwrap_or(Time::ZERO);
    from_sources.max(from_placements)
}

impl SceneDraft {
    pub fn explain(&mut self, origin: Origin, message: impl Into<String>) {
        self.explain.push(ExplainLine {
            origin,
            message: message.into(),
        });
    }

    pub fn next_placement_id(&self, prefix: &str) -> String {
        format!("{}:{}:{}", self.name, prefix, self.placements.len() + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn percent(digits: i64, scale: u32) -> ValueView {
        ValueView::Quantity {
            negative: false,
            digits,
            scale,
            unit: Some("%".into()),
        }
    }

    #[test]
    fn generic_percent_tuple_projects_to_canvas_basis_points() {
        let tuple = ValueView::Tuple(vec![percent(125, 1), percent(875, 1)]);
        assert_eq!(
            tuple.as_normalized_position(),
            NormalizedPosition::new(1_250, 8_750)
        );
        let outside = ValueView::Tuple(vec![percent(101, 0), percent(0, 0)]);
        assert!(outside.as_normalized_position().is_none());
        assert_eq!(
            percent(1255, 1).as_normalized_scale(),
            NormalizedScale::new(1_255)
        );
    }
}

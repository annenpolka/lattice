use std::path::Path;

use lattice_core::{
    Diagnostic, Media, MediaLocator, Origin, Project, Provenance, Sequence, Source, TimeMap,
    TimeSpan, Timeline, TimelineError, flatten_project,
};
use lattice_media::{ExportError, ExportReport, PreviewOptions, export_preview};
use lattice_vel::{Document, Expr, Item, ParseError};
use lattice_wasm::{ExplainLine, LoweringRegistry, SceneDraft};
use serde::Serialize;
use thiserror::Error;

use crate::lower::{invocation_view, over_path};
use crate::time_eval::{TimeEvalError, expr_name, range_times};

#[derive(Debug, Error)]
pub enum EngineError {
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Time(#[from] TimeEvalError),
    #[error(transparent)]
    Timeline(#[from] TimelineError),
    #[error(transparent)]
    Export(#[from] ExportError),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExplainEvent {
    pub origin: Origin,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Compilation {
    pub project: Project,
    pub diagnostics: Vec<Diagnostic>,
    pub explain: Vec<ExplainEvent>,
}

impl Compilation {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == lattice_core::Severity::Error)
    }
}

pub struct Engine {
    registry: LoweringRegistry,
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            registry: LoweringRegistry::stdlib(),
        }
    }
}

impl Engine {
    pub fn compile(&self, source: &str) -> Result<Compilation, EngineError> {
        let document = lattice_vel::parse(source)?;
        self.compile_document(&document)
    }

    pub fn timeline(project: &Project) -> Result<Timeline, EngineError> {
        Ok(flatten_project(project)?)
    }

    pub fn render(
        &self,
        project: &Project,
        output: &Path,
        media_root: &Path,
    ) -> Result<ExportReport, EngineError> {
        let timeline = flatten_project(project)?;
        Ok(export_preview(
            &timeline,
            &PreviewOptions {
                output: output.to_path_buf(),
                media_root: media_root.to_path_buf(),
            },
        )?)
    }

    fn compile_document(&self, document: &Document) -> Result<Compilation, EngineError> {
        let mut project = Project::new("untitled");
        let mut diagnostics = Vec::new();
        let mut explain = Vec::new();
        collect_header(document, &mut project, &mut diagnostics);

        for item in document.items.iter().chain(nested_items(document)) {
            if let Item::Sequence { name, body, .. } = item {
                let scene_ids = body
                    .items
                    .iter()
                    .filter_map(|inner| match inner {
                        Item::Invocation(inv) if inv.args.is_empty() => {
                            Some(format!("scene:{name}", name = inv.name))
                        }
                        Item::Binding { name, .. } => Some(format!("scene:{name}")),
                        _ => None,
                    })
                    .collect();
                project.sequences.push(Sequence {
                    id: format!("sequence:{name}"),
                    name: name.clone(),
                    scene_ids,
                });
                explain.push(ExplainEvent {
                    origin: Origin::Builtin {
                        name: "flow".into(),
                    },
                    message: format!("sequence `{name}` is a flow of scenes"),
                });
            }
        }

        for item in &document.items {
            let Item::Scene {
                name,
                over,
                body,
                span,
            } = item
            else {
                continue;
            };
            let mut draft = SceneDraft {
                name: name.clone(),
                over: over.as_ref().map(over_path),
                sources: Vec::new(),
                placements: Vec::new(),
                explain: Vec::new(),
                diagnostics: Vec::new(),
            };
            for inner in &body.items {
                match inner {
                    Item::Binding { expr, name, span } => {
                        draft.sources.push(binding_source(name, expr, *span)?);
                    }
                    Item::Invocation(inv) => {
                        let view = invocation_view(inv)?;
                        self.registry
                            .lower(&view, &mut draft)
                            .map_err(|err| TimeEvalError::Message(err.to_string()))?;
                    }
                    _ => {}
                }
            }
            self.registry
                .apply_convention(project.convention.as_deref(), &mut draft);
            explain.extend(draft.explain.iter().cloned().map(to_event));
            diagnostics.append(&mut draft.diagnostics);
            validate_scene(&draft, *span, &mut diagnostics);
            project.scenes.push(draft.finish(format!("scene:{name}")));
        }

        if project.name == "untitled" {
            diagnostics.push(Diagnostic::warning(
                "LAT-PROJ-001",
                "no `project` declaration; using name \"untitled\"",
                None,
            ));
        }
        if project.scenes.is_empty() {
            diagnostics.push(Diagnostic::error(
                "LAT-PROJ-002",
                "project has no scenes",
                None,
            ));
        }

        Ok(Compilation {
            project,
            diagnostics,
            explain,
        })
    }
}

fn nested_items(document: &Document) -> impl Iterator<Item = &Item> {
    document.items.iter().flat_map(|item| {
        let items: &[Item] = match item {
            Item::Project {
                body: Some(body), ..
            } => body.items.as_slice(),
            _ => &[],
        };
        items.iter()
    })
}

fn collect_header(document: &Document, project: &mut Project, diagnostics: &mut Vec<Diagnostic>) {
    for item in document.items.iter().chain(nested_items(document)) {
        match item {
            Item::Project { name, .. } => project.name.clone_from(name),
            Item::Convention { name, .. } => project.convention = Some(name.clone()),
            Item::Media {
                name, path, span, ..
            } => {
                if path.contains('\\') {
                    diagnostics.push(Diagnostic::warning(
                        "LAT-MEDIA-001",
                        "media path uses a backslash; Core stores the spelling, media backends interpret OS paths",
                        Some(*span),
                    ));
                }
                project.media.push(Media {
                    id: format!("media:{name}"),
                    name: name.clone(),
                    locator: MediaLocator::File { path: path.clone() },
                });
            }
            _ => {}
        }
    }
}

fn binding_source(
    name: &str,
    expr: &Expr,
    span: lattice_core::Span,
) -> Result<Source, EngineError> {
    let Expr::Index { target, index, .. } = expr else {
        return Err(
            TimeEvalError::Message(format!("binding `{name}` expected media[start..end]")).into(),
        );
    };
    let media_name = expr_name(target)
        .ok_or_else(|| TimeEvalError::Message(format!("binding `{name}` needs a media name")))?;
    let (start, end) = range_times(index)?;
    let duration = end.checked_sub(start).map_err(TimeEvalError::from)?;
    Ok(Source {
        id: format!("source:{name}"),
        name: name.to_string(),
        media_name,
        source_range: TimeSpan::new(start, duration),
        time_map: TimeMap::identity(start, duration),
        provenance: Provenance::source(span),
    })
}

fn validate_scene(draft: &SceneDraft, span: lattice_core::Span, diagnostics: &mut Vec<Diagnostic>) {
    let duration = lattice_wasm::scene_duration(draft);
    for placement in &draft.placements {
        if placement.span.end() > duration {
            diagnostics.push(Diagnostic::warning(
                "LAT-TIME-103",
                format!(
                    "{} extends past scene boundary ({})",
                    placement.id,
                    placement.span.end() - duration
                ),
                Some(span),
            ));
        }
    }
}

fn to_event(line: ExplainLine) -> ExplainEvent {
    ExplainEvent {
        origin: line.origin,
        message: line.message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::Time;

    const DEMO: &str = include_str!("../../../examples/gameplay-commentary/main.vel");

    #[test]
    fn compiles_demo() {
        let compilation = Engine::default().compile(DEMO).unwrap();
        assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
        assert_eq!(compilation.project.name, "demo");
        assert_eq!(compilation.project.scenes.len(), 1);
        let scene = &compilation.project.scenes[0];
        assert_eq!(
            scene.duration,
            Time::from_decimal_seconds(11, 5, 1).unwrap()
        );
        assert!(
            scene.sources[0]
                .time_map
                .segments
                .iter()
                .any(|seg| seg.rate == Time::ZERO)
        );
        assert!(
            compilation
                .explain
                .iter()
                .any(|event| event.message.contains("canvas-fill"))
        );
    }
}

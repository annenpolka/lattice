use std::path::Path;

use lattice_core::{
    Diagnostic, EditProposal, Locus, LocusId, LocusProjection, Media, MediaLocator, Origin,
    Project, Provenance, SemanticEdit, Sequence, Source, Time, TimeMap, TimeSpan, Timeline,
    TimelineError, flatten_project,
};
use lattice_media::{
    AudioMixError, ExportError, ExportReport, MediaInfo, MixSpec, PreparedAudio,
    PreviewFrameRequest, PreviewOptions, RendererRequest, export_preview, mix_timeline_audio,
    probe_media,
};
use lattice_vel::{Document, Expr, Item, ParseError};
use lattice_wasm::{
    ExplainLine, LoweringRegistry, OverlayPresetRegistry, SceneDraft, register_dsl_preset,
};
use serde::Serialize;
use thiserror::Error;

use crate::lower::{invocation_view, over_path};
use crate::resolve::{GeneratedMediaProvider, Resolution, ResolveError, ResolveOptions};
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
    #[error(transparent)]
    Audio(#[from] AudioMixError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("edit: {0}")]
    Edit(String),
    #[error("stale proposal (base revision {expected}, current {found})")]
    StaleProposal { expected: String, found: String },
    #[error(transparent)]
    Resolve(#[from] ResolveError),
}

impl EngineError {
    pub fn stale_revisions(&self) -> Option<(&str, &str)> {
        match self {
            Self::StaleProposal { expected, found } => Some((expected, found)),
            _ => None,
        }
    }
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
    #[serde(skip)]
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
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
    #[must_use]
    pub fn with_registry(registry: LoweringRegistry) -> Self {
        Self { registry }
    }

    pub fn compile(&self, source: &str) -> Result<Compilation, EngineError> {
        self.compile_origin(source, None)
    }

    pub fn compile_origin(
        &self,
        source: &str,
        origin: Option<String>,
    ) -> Result<Compilation, EngineError> {
        let document = lattice_vel::parse(source)?;
        let mut compilation = self.compile_document(&document)?;
        compilation.source = source.to_string();
        compilation.origin = origin;
        Ok(compilation)
    }

    pub fn compile_path(&self, path: &Path) -> Result<Compilation, EngineError> {
        let source = std::fs::read_to_string(path)?;
        self.compile_origin(&source, Some(path.display().to_string()))
    }

    pub fn uses_wasm_stdlib(&self) -> bool {
        self.registry.uses_wasm()
    }

    pub fn timeline(project: &Project) -> Result<Timeline, EngineError> {
        Ok(flatten_project(project)?)
    }

    pub fn loci(&self, compilation: &Compilation) -> Result<Vec<Locus>, EngineError> {
        let timeline = flatten_project(&compilation.project)?;
        Ok(crate::locus::loci_from_project(
            &compilation.project,
            &timeline,
            compilation.origin.as_deref(),
        ))
    }

    pub fn inspect(
        &self,
        compilation: &Compilation,
        id: &LocusId,
    ) -> Result<LocusProjection, EngineError> {
        let loci = self.loci(compilation)?;
        let locus = crate::locus::locus_by_id(&loci, id)
            .ok_or_else(|| EngineError::Edit(format!("unknown locus {}", id.as_str())))?;
        Ok(crate::locus::project_locus(locus))
    }

    pub fn locus_at_source(
        &self,
        compilation: &Compilation,
        offset: u32,
    ) -> Result<Option<Locus>, EngineError> {
        let loci = self.loci(compilation)?;
        Ok(crate::locus::locus_at_source(&loci, offset).cloned())
    }

    pub fn locus_for_node(
        &self,
        compilation: &Compilation,
        node_id: &str,
    ) -> Result<Option<Locus>, EngineError> {
        let loci = self.loci(compilation)?;
        Ok(crate::locus::locus_for_node(&loci, node_id).cloned())
    }

    pub fn locus_at_timeline(
        &self,
        compilation: &Compilation,
        time: Time,
    ) -> Result<Option<Locus>, EngineError> {
        let loci = self.loci(compilation)?;
        Ok(crate::locus::locus_at_timeline(&loci, time).cloned())
    }

    /// All loci covering a timeline time, including Scene and Source via clip membership.
    /// Does not rank or collapse. Studio uses this for overlap pointing.
    pub fn loci_covering_timeline(
        &self,
        compilation: &Compilation,
        time: Time,
    ) -> Result<Vec<Locus>, EngineError> {
        let timeline = flatten_project(&compilation.project)?;
        let loci = crate::locus::loci_from_project(
            &compilation.project,
            &timeline,
            compilation.origin.as_deref(),
        );
        Ok(crate::locus::loci_covering_timeline(
            &compilation.project,
            &timeline,
            &loci,
            time,
        ))
    }

    /// Engine-named legal edits for a committed locus. Surfaces do not invent this set.
    #[must_use]
    pub fn legal_edits(&self, locus: &Locus) -> Vec<crate::legal::LegalEdit> {
        crate::legal::legal_edits_for(locus)
    }

    pub fn propose(
        &self,
        compilation: &Compilation,
        locus: &Locus,
        edit: SemanticEdit,
    ) -> Result<EditProposal, EngineError> {
        crate::edit::propose_edit(&compilation.source, locus, edit)
    }

    pub fn apply_proposal(
        &self,
        source: &str,
        proposal: &EditProposal,
    ) -> Result<String, EngineError> {
        crate::edit::apply_proposal(source, proposal)
    }

    pub fn write_source_atomic(&self, path: &Path, contents: &str) -> Result<(), EngineError> {
        Ok(crate::atomic::write_source_atomic(path, contents)?)
    }

    pub fn import_media(
        &self,
        media: &Path,
        out_dir: Option<&Path>,
    ) -> Result<crate::import::ImportResult, EngineError> {
        crate::import::import_media(media, out_dir)
    }

    /// Probe the first on-disk file media referenced by the project.
    pub fn project_media_info(&self, project: &Project, media_root: &Path) -> Option<MediaInfo> {
        for media in &project.media {
            let MediaLocator::File { path } = &media.locator else {
                continue;
            };
            let candidate = media_root.join(path);
            if candidate.is_file() {
                return probe_media(candidate).ok();
            }
        }
        None
    }

    /// Load `lattice.lock.json` next to the project if present.
    pub fn load_lock(media_root: &Path) -> Option<lattice_core::ResolveLock> {
        let path = media_root.join("lattice.lock.json");
        let text = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&text).ok()
    }

    fn preview_spec(request: &PreviewFrameRequest) -> lattice_media::OutputSpec {
        lattice_media::OutputSpec {
            width: request.width,
            height: request.height,
            fps_num: request.fps_num,
            fps_den: request.fps_den,
            sample_rate: 44_100,
            channels: 2,
        }
    }

    fn preview_options(
        media_root: &Path,
        output: &Path,
        lock: Option<&lattice_core::ResolveLock>,
        renderer: RendererRequest,
    ) -> PreviewOptions {
        PreviewOptions {
            output: output.to_path_buf(),
            media_root: media_root.to_path_buf(),
            lock: lock.cloned().or_else(|| Self::load_lock(media_root)),
            spec: lattice_media::OutputSpec::preview(),
            renderer,
            allow_fixtures: false,
            font: None,
        }
    }

    pub fn preview_frame(
        &self,
        project: &Project,
        request: &PreviewFrameRequest,
        media_root: &Path,
        output: &Path,
    ) -> Result<std::path::PathBuf, EngineError> {
        self.preview_frame_with_lock(project, request, media_root, output, None)
    }

    pub fn preview_frame_with_lock(
        &self,
        project: &Project,
        request: &PreviewFrameRequest,
        media_root: &Path,
        output: &Path,
        lock: Option<&lattice_core::ResolveLock>,
    ) -> Result<std::path::PathBuf, EngineError> {
        let timeline = flatten_project(project)?;
        let options = Self::preview_options(media_root, output, lock, RendererRequest::RequireCpu);
        Ok(lattice_media::render_still(
            &timeline,
            request.timeline_time,
            Self::preview_spec(request),
            &options,
            output,
        )?)
    }

    pub fn sample_frame(
        &self,
        project: &Project,
        request: &PreviewFrameRequest,
        media_root: &Path,
        lock: Option<&lattice_core::ResolveLock>,
    ) -> Result<(lattice_core::RenderScene, lattice_media::RawFrame), EngineError> {
        let timeline = flatten_project(project)?;
        let options = Self::preview_options(
            media_root,
            &media_root.join("_lattice_sample"),
            lock,
            RendererRequest::RequireCpu,
        );
        Ok(lattice_media::sample_frame(
            &timeline,
            request.timeline_time,
            Self::preview_spec(request),
            &options,
        )?)
    }

    /// Warm sample-at-t session for Studio stills. Not a realtime decoder.
    pub fn preview_sampler(
        &self,
        project: &Project,
        request: &PreviewFrameRequest,
        media_root: &Path,
        output: &Path,
        lock: Option<&lattice_core::ResolveLock>,
    ) -> Result<lattice_media::PreviewSampler, EngineError> {
        self.sample_session(
            project,
            request,
            media_root,
            output,
            lock,
            RendererRequest::RequireCpu,
        )
    }

    /// Create a reusable sample-at-t session with a required renderer.
    pub fn sample_session(
        &self,
        project: &Project,
        request: &PreviewFrameRequest,
        media_root: &Path,
        output: &Path,
        lock: Option<&lattice_core::ResolveLock>,
        renderer: RendererRequest,
    ) -> Result<lattice_media::SampleSession, EngineError> {
        let timeline = flatten_project(project)?;
        let options = Self::preview_options(media_root, output, lock, renderer);
        Ok(lattice_media::SampleSession::open(
            timeline,
            Self::preview_spec(request),
            &options,
        )?)
    }

    /// Prepare the full timeline mix for Studio monitoring.
    ///
    /// The media backend is shared with export. `None` means the timeline has
    /// no audio windows; missing referenced or generated media is an error.
    pub fn prepare_audio(
        &self,
        project: &Project,
        media_root: &Path,
        lock: Option<&lattice_core::ResolveLock>,
        spec: MixSpec,
    ) -> Result<Option<PreparedAudio>, EngineError> {
        let timeline = flatten_project(project)?;
        let options = Self::preview_options(
            media_root,
            &media_root.join(".lattice-audio-monitor"),
            lock,
            RendererRequest::RequireCpu,
        );
        Ok(mix_timeline_audio(&timeline, &options, spec)?)
    }

    pub fn reject_proposal(&self, source: &str, _proposal: &EditProposal) -> String {
        source.to_string()
    }

    pub fn resolve(
        &self,
        project: &Project,
        options: &ResolveOptions<'_>,
        provider: &mut dyn GeneratedMediaProvider,
    ) -> Result<Resolution, EngineError> {
        Ok(crate::resolve::resolve_project(project, options, provider)?)
    }

    pub fn render(
        &self,
        project: &Project,
        output: &Path,
        media_root: &Path,
    ) -> Result<ExportReport, EngineError> {
        self.render_with_lock(project, output, media_root, None)
    }

    pub fn render_with_lock(
        &self,
        project: &Project,
        output: &Path,
        media_root: &Path,
        lock: Option<&lattice_core::ResolveLock>,
    ) -> Result<ExportReport, EngineError> {
        self.render_with_options(
            project,
            &PreviewOptions {
                output: output.to_path_buf(),
                media_root: media_root.to_path_buf(),
                lock: lock.cloned().or_else(|| Self::load_lock(media_root)),
                spec: lattice_media::OutputSpec::preview(),
                renderer: RendererRequest::RequireCpu,
                allow_fixtures: false,
                font: None,
            },
        )
    }

    pub fn render_with_options(
        &self,
        project: &Project,
        options: &PreviewOptions,
    ) -> Result<ExportReport, EngineError> {
        let timeline = flatten_project(project)?;
        Ok(export_preview(&timeline, options)?)
    }

    fn compile_document(&self, document: &Document) -> Result<Compilation, EngineError> {
        let mut project = Project::new("untitled");
        let mut diagnostics = Vec::new();
        let mut explain = Vec::new();
        collect_header(document, &mut project, &mut diagnostics);
        let mut overlay_presets = self.registry.overlay_presets();
        collect_overlay_presets(
            document,
            &mut overlay_presets,
            &mut diagnostics,
            &mut explain,
        )?;

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

        compile_scenes(
            document,
            &self.registry,
            &overlay_presets,
            &mut project,
            &mut diagnostics,
            &mut explain,
        )?;

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
            source: String::new(),
            origin: None,
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

fn compile_scenes(
    document: &Document,
    registry: &LoweringRegistry,
    overlay_presets: &OverlayPresetRegistry,
    project: &mut Project,
    diagnostics: &mut Vec<Diagnostic>,
    explain: &mut Vec<ExplainEvent>,
) -> Result<(), EngineError> {
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
            media: Vec::new(),
            source_fade_in: Vec::new(),
            source_gain_db: Vec::new(),
            explain: Vec::new(),
            diagnostics: Vec::new(),
            overlay_presets: overlay_presets.clone(),
        };
        for inner in &body.items {
            match inner {
                Item::Binding { expr, name, span } => {
                    draft.sources.push(binding_source(name, expr, *span)?);
                }
                Item::Invocation(inv) => {
                    let view = invocation_view(inv)?;
                    registry
                        .lower(&view, &mut draft)
                        .map_err(|err| TimeEvalError::Message(err.to_string()))?;
                }
                _ => {}
            }
        }
        registry.apply_convention(project.convention.as_deref(), &mut draft);
        explain.extend(draft.explain.iter().cloned().map(to_event));
        diagnostics.append(&mut draft.diagnostics);
        validate_scene(&draft, *span, diagnostics);
        project.media.append(&mut draft.media);
        project.scenes.push(draft.finish(format!("scene:{name}")));
    }
    Ok(())
}

fn collect_overlay_presets(
    document: &Document,
    presets: &mut OverlayPresetRegistry,
    diagnostics: &mut Vec<Diagnostic>,
    explain: &mut Vec<ExplainEvent>,
) -> Result<(), EngineError> {
    for item in document.items.iter().chain(nested_items(document)) {
        let Item::Invocation(inv) = item else {
            continue;
        };
        if inv.name != "overlay-preset" {
            continue;
        }
        let view = invocation_view(inv)?;
        if let Some(line) = register_dsl_preset(&view, presets, diagnostics) {
            explain.push(to_event(line));
        }
    }
    Ok(())
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
        generated: false,
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

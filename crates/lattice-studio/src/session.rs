//! GPUI-free Studio session. The window is a client of this type.

use std::path::{Path, PathBuf};

use lattice_engine::{
    Compilation, Diagnostic, EditProposal, Engine, EngineError, Locus, LocusId, Provenance,
    RenderPlan, SemanticEdit, Span, Time, plan_from_timeline,
};

use crate::layout::{self, StudioLayout};

/// Engine-backed Studio state. No GPUI types.
pub struct StudioSession {
    engine: Engine,
    path: PathBuf,
    compilation: Compilation,
    current: Option<LocusId>,
    review: Option<EditProposal>,
}

impl StudioSession {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, EngineError> {
        let path = path.as_ref().to_path_buf();
        let engine = Engine::default();
        let compilation = engine.compile_path(&path)?;
        let mut session = Self {
            engine,
            path,
            compilation,
            current: None,
            review: None,
        };
        if let Some(title) = session
            .loci()?
            .into_iter()
            .find(|locus| locus.kind == lattice_engine::LocusKind::Title)
        {
            session.current = Some(title.id);
        }
        Ok(session)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn source(&self) -> &str {
        &self.compilation.source
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.compilation.diagnostics
    }

    pub fn compilation(&self) -> &Compilation {
        &self.compilation
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn review_proposal(&self) -> Option<&EditProposal> {
        self.review.as_ref()
    }

    pub fn loci(&self) -> Result<Vec<Locus>, EngineError> {
        self.engine.loci(&self.compilation)
    }

    pub fn layout(&self) -> Result<StudioLayout, EngineError> {
        layout::from_session(self)
    }

    pub fn point_at(&mut self, id: LocusId) {
        self.current = Some(id);
    }

    pub fn point_at_title(&mut self) -> Result<Option<Locus>, EngineError> {
        let title = self
            .loci()?
            .into_iter()
            .find(|locus| locus.kind == lattice_engine::LocusKind::Title);
        if let Some(locus) = &title {
            self.current = Some(locus.id.clone());
        }
        Ok(title)
    }

    /// Point from a canvas overlay (locus id of that overlay).
    pub fn point_from_canvas_overlay(
        &mut self,
        locus_id: &str,
    ) -> Result<Option<Locus>, EngineError> {
        let found = self
            .loci()?
            .into_iter()
            .find(|locus| locus.id.as_str() == locus_id);
        if let Some(locus) = &found {
            self.current = Some(locus.id.clone());
        }
        Ok(found)
    }

    /// Point from a byte offset in the VEL source.
    pub fn point_from_source_offset(&mut self, offset: u32) -> Result<Option<Locus>, EngineError> {
        let found = self.engine.locus_at_source(&self.compilation, offset)?;
        if let Some(locus) = &found {
            self.current = Some(locus.id.clone());
        }
        Ok(found)
    }

    /// Point from a time on the flattened timeline.
    pub fn point_from_timeline_time(&mut self, time: Time) -> Result<Option<Locus>, EngineError> {
        let found = self.engine.locus_at_timeline(&self.compilation, time)?;
        if let Some(locus) = &found {
            self.current = Some(locus.id.clone());
        }
        Ok(found)
    }

    pub fn current_locus(&self) -> Result<Option<Locus>, EngineError> {
        let Some(id) = &self.current else {
            return Ok(None);
        };
        Ok(self
            .engine
            .inspect(&self.compilation, id)
            .ok()
            .map(|projection| projection.locus))
    }

    pub fn current_provenance(&self) -> Result<Option<Provenance>, EngineError> {
        Ok(self.current_locus()?.map(|locus| locus.provenance))
    }

    /// Optional Navigate: the current locus's source span.
    pub fn go_to_definition(&self) -> Result<Option<Span>, EngineError> {
        Ok(self.current_locus()?.and_then(|locus| locus.source_span))
    }

    pub fn preview_plan(&self) -> Result<RenderPlan, EngineError> {
        let timeline = Engine::timeline(&self.compilation.project)?;
        Ok(plan_from_timeline(&timeline)?)
    }

    pub fn propose_title_text(
        &mut self,
        text: impl Into<String>,
    ) -> Result<EditProposal, EngineError> {
        let locus = self
            .current_locus()?
            .ok_or_else(|| EngineError::Edit("no current locus".into()))?;
        let proposal = self.engine.propose(
            &self.compilation,
            &locus,
            SemanticEdit::Title {
                text: Some(text.into()),
                at: None,
                duration: None,
                opacity: None,
            },
        )?;
        self.review = Some(proposal.clone());
        Ok(proposal)
    }

    pub fn apply_review(&mut self, proposal: &EditProposal) -> Result<(), EngineError> {
        let new_source = self
            .engine
            .apply_proposal(&self.compilation.source, proposal);
        std::fs::write(&self.path, &new_source)?;
        self.compilation = self.engine.compile_path(&self.path)?;
        self.review = None;
        if let Some(title) = self
            .loci()?
            .into_iter()
            .find(|locus| locus.kind == lattice_engine::LocusKind::Title)
        {
            self.current = Some(title.id);
        }
        Ok(())
    }

    pub fn reject_review(&mut self, proposal: &EditProposal) -> String {
        let original = self
            .engine
            .reject_proposal(&self.compilation.source, proposal);
        self.review = None;
        original
    }

    /// Everyday Manipulate: rewrite title text through Engine, then recompile.
    pub fn apply_title_text(&mut self, text: &str) -> Result<(), EngineError> {
        let proposal = self.propose_title_text(text)?;
        self.apply_review(&proposal)
    }

    /// Flattened preview via Engine render (`FFmpeg`). Writes beside the open VEL.
    pub fn render_preview(&self) -> Result<std::path::PathBuf, EngineError> {
        let dir = self.path.parent().unwrap_or_else(|| Path::new("."));
        let output = dir.join("studio-preview.mp4");
        self.engine
            .render(&self.compilation.project, &output, dir)?;
        Ok(output)
    }

    pub fn uses_engine_not_own_compiler(&self) -> bool {
        true
    }
}

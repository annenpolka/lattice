//! Engine-named legal edits for a committed locus.
//!
//! Legality is a property of `here`, not of the surface that was touched.
//! Studio routing (what a gesture can commit) lives in the Studio client.

use lattice_core::{Locus, LocusId, LocusKind};
use serde::{Deserialize, Serialize};

/// One Engine-named legal `SemanticEdit` for a locus.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegalEdit {
    pub verb: String,
    pub target: LocusId,
    pub scope: String,
    pub effect: String,
}

/// Typed absence / routing reasons shared by Studio, CLI `--json`, and agents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AbsenceReason {
    UnresolvedPointing,
    NeedsSourceBinding,
    NeedsScene,
    NeedsTitle,
    NeedsCallout,
    NeedsOverlay,
    RoutedElsewhere,
    StructurallyAbsent,
}

impl AbsenceReason {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnresolvedPointing => "unresolved-pointing",
            Self::NeedsSourceBinding => "needs-source-binding",
            Self::NeedsScene => "needs-scene",
            Self::NeedsTitle => "needs-title",
            Self::NeedsCallout => "needs-callout",
            Self::NeedsOverlay => "needs-overlay",
            Self::RoutedElsewhere => "routed-elsewhere",
            Self::StructurallyAbsent => "structurally-absent",
        }
    }
}

/// Legal `SemanticEdit` kinds the Engine can name for this locus.
///
/// A verb is present only when the Engine can name the exact target and scope.
/// Scene-level insert of a new title is not a Title-shaped property of a Scene.
#[must_use]
pub fn legal_edits_for(locus: &Locus) -> Vec<LegalEdit> {
    let target = locus.id.clone();
    match locus.kind {
        LocusKind::Title => vec![
            legal(
                "title",
                &target,
                "definition",
                "rewrite title text, time, or opacity",
            ),
            legal(
                "set-position",
                &target,
                "placement",
                "move the title in normalized Canvas Space",
            ),
            legal(
                "resize-overlay",
                &target,
                "placement",
                "resize the title while keeping the opposite corner",
            ),
        ],
        LocusKind::Callout => vec![
            legal("callout", &target, "definition", "rewrite callout time"),
            legal(
                "set-position",
                &target,
                "placement",
                "move the callout in normalized Canvas Space",
            ),
            legal(
                "resize-overlay",
                &target,
                "placement",
                "resize the callout while keeping the opposite corner",
            ),
        ],
        LocusKind::Source => vec![
            legal(
                "trim",
                &target,
                "source-range",
                "set in/out on this source binding",
            ),
            legal(
                "set-gain",
                &target,
                "source-binding",
                "set gain on this source",
            ),
            legal(
                "set-fade",
                &target,
                "source-binding",
                "set fade on this source",
            ),
        ],
        LocusKind::Scene => vec![
            legal(
                "split",
                &target,
                "scene",
                "split this scene at a source time",
            ),
            legal("delete", &target, "scene", "delete this scene"),
            legal(
                "reorder-scene",
                &target,
                "sequence",
                "reorder this scene in the sequence",
            ),
        ],
        LocusKind::Sequence | LocusKind::Media | LocusKind::Speech | LocusKind::Placement => {
            Vec::new()
        }
    }
}

fn legal(verb: &str, target: &LocusId, scope: &str, effect: &str) -> LegalEdit {
    LegalEdit {
        verb: verb.into(),
        target: target.clone(),
        scope: scope.into(),
        effect: effect.into(),
    }
}

/// Whether `verb` is in the Engine legal set for this locus.
#[must_use]
pub fn is_legal_verb(locus: &Locus, verb: &str) -> bool {
    legal_edits_for(locus).iter().any(|edit| edit.verb == verb)
}

#[cfg(test)]
mod tests {
    use lattice_core::{Locus, LocusId, LocusKind, Origin, Provenance};

    use super::{is_legal_verb, legal_edits_for};

    fn locus(kind: LocusKind, id: &str) -> Locus {
        Locus {
            id: LocusId::new(id),
            kind,
            node_id: id.into(),
            label: "demo".into(),
            origin: None,
            source_span: None,
            scene_id: Some("scene:demo".into()),
            sequence_id: Some("sequence:main".into()),
            timeline_span: None,
            provenance: Provenance {
                span: None,
                origin: Origin::Source,
            },
            derived_from: None,
            visual: None,
        }
    }

    #[test]
    fn source_legal_set_is_trim_gain_fade() {
        let edits = legal_edits_for(&locus(LocusKind::Source, "source:fight"));
        let verbs: Vec<_> = edits.iter().map(|edit| edit.verb.as_str()).collect();
        assert_eq!(verbs, ["trim", "set-gain", "set-fade"]);
        assert!(
            edits
                .iter()
                .all(|edit| edit.target.as_str() == "source:fight")
        );
        assert!(!is_legal_verb(
            &locus(LocusKind::Source, "source:fight"),
            "split"
        ));
    }

    #[test]
    fn scene_legal_set_does_not_invent_a_source() {
        let edits = legal_edits_for(&locus(LocusKind::Scene, "scene:demo"));
        let verbs: Vec<_> = edits.iter().map(|edit| edit.verb.as_str()).collect();
        assert_eq!(verbs, ["split", "delete", "reorder-scene"]);
        assert!(!is_legal_verb(
            &locus(LocusKind::Scene, "scene:demo"),
            "set-gain"
        ));
        assert!(!is_legal_verb(
            &locus(LocusKind::Scene, "scene:demo"),
            "title"
        ));
    }

    #[test]
    fn title_legal_set_includes_canvas_geometry() {
        let edits = legal_edits_for(&locus(LocusKind::Title, "title:hello"));
        let verbs: Vec<_> = edits.iter().map(|edit| edit.verb.as_str()).collect();
        assert_eq!(verbs, ["title", "set-position", "resize-overlay"]);
    }
}

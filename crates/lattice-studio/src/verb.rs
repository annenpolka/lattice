//! Integrated verb-license spine.
//!
//! One locus, one legal set, one utterance. The Engine names what is legal for
//! here. A gesture on a projection says what it commits. When those sets differ,
//! the difference is spoken — never implied, never silently retargeted.
//!
//! The touched projection is routing, not a second selection and not a second
//! legal set.

use lattice_engine::{
    AbsenceReason, LegalEdit, Locus, LocusKind, SemanticEdit, Time, legal_edits_for,
};

/// Projection that was touched. Routing only — not a per-view selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Projection {
    #[default]
    Timeline,
    Canvas,
    Source,
    Inspector,
    Review,
    Tree,
    Toolbar,
}

impl Projection {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Timeline => "Timeline",
            Self::Canvas => "Canvas",
            Self::Source => "VEL",
            Self::Inspector => "Inspector",
            Self::Review => "Review",
            Self::Tree => "Sequence",
            Self::Toolbar => "Toolbar",
        }
    }
}

/// A failed coordinate point held open on the projection that was touched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnresolvedPointing {
    pub projection: Projection,
    pub time: Option<Time>,
    pub candidates: Vec<Locus>,
}

/// One overlap card: identity, scope, and what *this* projection would commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PointCandidate {
    pub locus_id: String,
    pub kind: String,
    pub label: String,
    pub scope: String,
    pub routed_verbs: Vec<String>,
}

/// A spoken clause: present, routed elsewhere, or absent with a typed reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpokenClause {
    pub verb: String,
    pub status: String,
    pub reason: Option<String>,
    pub text: String,
}

/// One utterance for here: legal set, routed set, and the spoken gap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Utterance {
    pub here: Option<String>,
    pub here_kind: Option<String>,
    pub pointing: String,
    pub legal: Vec<LegalEdit>,
    pub routed: Vec<String>,
    pub spoken: Vec<SpokenClause>,
    pub projection: Projection,
}

impl Utterance {
    #[must_use]
    pub fn spoken_text(&self) -> String {
        self.spoken
            .iter()
            .map(|clause| clause.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[must_use]
    pub fn speaks_gap(&self) -> bool {
        self.spoken.iter().any(|clause| clause.status != "present")
    }
}

/// Verbs a gesture on `projection` can commit for this locus kind.
///
/// This is routing, not legality. A missing route is not an absent edit.
#[must_use]
pub fn routed_verbs(projection: Projection, kind: LocusKind) -> Vec<&'static str> {
    match (projection, kind) {
        (Projection::Timeline, LocusKind::Source) => vec!["trim"],
        (Projection::Timeline | Projection::Inspector | Projection::Toolbar, LocusKind::Title) => {
            vec!["title"]
        }
        (Projection::Timeline, LocusKind::Callout) => vec!["callout"],
        (Projection::Timeline, LocusKind::Scene) => vec!["reorder-scene", "split", "delete"],
        (Projection::Canvas, LocusKind::Title | LocusKind::Callout) => {
            vec!["set-position", "resize-overlay"]
        }
        (Projection::Toolbar, LocusKind::Source) => vec!["trim", "set-gain", "set-fade"],
        (Projection::Toolbar, LocusKind::Scene) => vec!["split", "delete", "reorder-scene"],
        _ => vec![],
    }
}

/// Projection that actually routes `verb` for this kind.
///
/// Derived from [`routed_verbs`] so a spoken "committed on" clause cannot name
/// a surface that does not commit the verb. Timeline is searched first, then
/// Toolbar, then the remaining projections.
#[must_use]
pub fn commit_projection(verb: &str, kind: LocusKind) -> Option<Projection> {
    const SURFACES: [Projection; 7] = [
        Projection::Timeline,
        Projection::Toolbar,
        Projection::Canvas,
        Projection::Inspector,
        Projection::Source,
        Projection::Review,
        Projection::Tree,
    ];
    SURFACES
        .into_iter()
        .find(|&surface| routed_verbs(surface, kind).contains(&verb))
}

#[must_use]
pub fn verb_for_edit(edit: &SemanticEdit) -> &'static str {
    match edit {
        SemanticEdit::Title { .. } => "title",
        SemanticEdit::Trim { .. } => "trim",
        SemanticEdit::Split { .. } => "split",
        SemanticEdit::Delete => "delete",
        SemanticEdit::SetGain { .. } => "set-gain",
        SemanticEdit::SetFade { .. } => "set-fade",
        SemanticEdit::ReorderScene { .. } => "reorder-scene",
        SemanticEdit::Callout { .. } => "callout",
        SemanticEdit::SetPosition { .. } => "set-position",
        SemanticEdit::ResizeOverlay { .. } => "resize-overlay",
    }
}

#[must_use]
pub fn kind_label(kind: LocusKind) -> &'static str {
    match kind {
        LocusKind::Title => "title",
        LocusKind::Callout => "callout",
        LocusKind::Source => "source",
        LocusKind::Placement => "placement",
        LocusKind::Scene => "scene",
        LocusKind::Sequence => "sequence",
        LocusKind::Media => "media",
        LocusKind::Speech => "speech",
    }
}

/// Build the one utterance for a committed or unresolved here.
#[must_use]
pub fn utterance(
    here: Option<&Locus>,
    unresolved: Option<&UnresolvedPointing>,
    projection: Projection,
) -> Utterance {
    if let Some(point) = unresolved {
        let spoken = vec![SpokenClause {
            verb: String::new(),
            status: AbsenceReason::UnresolvedPointing.as_str().into(),
            reason: Some(AbsenceReason::UnresolvedPointing.as_str().into()),
            text: format!(
                "This {} point named {} loci. Pick one card. Here is unset.",
                point.projection.as_str(),
                point.candidates.len()
            ),
        }];
        return Utterance {
            here: None,
            here_kind: None,
            pointing: AbsenceReason::UnresolvedPointing.as_str().into(),
            legal: Vec::new(),
            routed: Vec::new(),
            spoken,
            projection: point.projection,
        };
    }
    let Some(locus) = here else {
        return Utterance {
            here: None,
            here_kind: None,
            pointing: "none".into(),
            legal: Vec::new(),
            routed: Vec::new(),
            spoken: vec![SpokenClause {
                verb: String::new(),
                status: "none".into(),
                reason: None,
                text: "No here. Point a locus.".into(),
            }],
            projection,
        };
    };
    let legal = legal_edits_for(locus);
    let routed: Vec<String> = routed_verbs(projection, locus.kind)
        .into_iter()
        .map(str::to_string)
        .collect();
    let spoken = speak_legal_vs_routed(locus, &legal, &routed, projection);
    Utterance {
        here: Some(format!("{} \"{}\"", kind_label(locus.kind), locus.label)),
        here_kind: Some(kind_label(locus.kind).into()),
        pointing: "committed".into(),
        legal,
        routed,
        spoken,
        projection,
    }
}

fn speak_legal_vs_routed(
    locus: &Locus,
    legal: &[LegalEdit],
    routed: &[String],
    projection: Projection,
) -> Vec<SpokenClause> {
    let mut spoken = Vec::new();
    for edit in legal {
        if routed.iter().any(|verb| verb == &edit.verb) {
            spoken.push(SpokenClause {
                verb: edit.verb.clone(),
                status: "present".into(),
                reason: None,
                text: format!(
                    "{} is legal for this {} and this {} gesture commits it.",
                    edit.verb,
                    kind_label(locus.kind),
                    projection.as_str()
                ),
            });
            continue;
        }
        let route = commit_projection(&edit.verb, locus.kind).unwrap_or(Projection::Timeline);
        spoken.push(SpokenClause {
            verb: edit.verb.clone(),
            status: "routed".into(),
            reason: Some(AbsenceReason::RoutedElsewhere.as_str().into()),
            text: format!(
                "{} is legal for this {}, committed on {} — not implied absent here.",
                edit.verb,
                kind_label(locus.kind),
                route.as_str()
            ),
        });
    }
    spoken
}

/// Refuse a toolbar/session edit whose target the Engine cannot name for here.
#[must_use]
pub fn refuse_edit(here: Option<&Locus>, edit: &SemanticEdit) -> String {
    let verb = verb_for_edit(edit);
    let Some(locus) = here else {
        return format!("{verb} needs a committed locus. Here is unset.");
    };
    if legal_edits_for(locus)
        .iter()
        .any(|legal| legal.verb == verb)
    {
        return format!(
            "{verb} is legal for this {} and targets {}.",
            kind_label(locus.kind),
            locus.id.as_str()
        );
    }
    let reason = match verb {
        "trim" | "set-gain" | "set-fade" => AbsenceReason::NeedsSourceBinding,
        "split" | "delete" | "reorder-scene" => AbsenceReason::NeedsScene,
        "title" => AbsenceReason::NeedsTitle,
        "callout" => AbsenceReason::NeedsCallout,
        "set-position" | "resize-overlay" => AbsenceReason::NeedsOverlay,
        _ => AbsenceReason::StructurallyAbsent,
    };
    let relation = match (verb, locus.kind) {
        ("split" | "delete" | "reorder-scene", LocusKind::Source) => locus
            .scene_id
            .as_deref()
            .map(|scene| format!(" {verb} is legal for {scene} — Navigate, do not retarget."))
            .unwrap_or_default(),
        ("trim" | "set-gain" | "set-fade", LocusKind::Scene) => {
            format!(" {verb} needs a source binding. Point the video clip.")
        }
        _ => String::new(),
    };
    format!(
        "{verb} is not legal for {} \"{}\" ({reason}).{relation}",
        kind_label(locus.kind),
        locus.label,
        reason = reason.as_str()
    )
}

/// Cards for an unresolved point on one projection.
#[must_use]
pub fn candidate_cards(unresolved: &UnresolvedPointing) -> Vec<PointCandidate> {
    unresolved
        .candidates
        .iter()
        .map(|locus| {
            let routed = routed_verbs(unresolved.projection, locus.kind)
                .into_iter()
                .map(str::to_string)
                .collect();
            let scope = locus.timeline_span.map_or_else(
                || {
                    locus.source_span.map_or_else(
                        || kind_label(locus.kind).into(),
                        |span| format!("main.vel:{}", span.line),
                    )
                },
                |span| format!("{}–{}", span.start, span.end()),
            );
            PointCandidate {
                locus_id: locus.id.as_str().to_string(),
                kind: kind_label(locus.kind).into(),
                label: locus.label.clone(),
                scope,
                routed_verbs: routed,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use lattice_engine::{Locus, LocusId, LocusKind, Origin, Provenance, SemanticEdit};

    use super::{Projection, commit_projection, refuse_edit, routed_verbs, utterance};

    fn locus(kind: LocusKind, id: &str, label: &str) -> Locus {
        Locus {
            id: LocusId::new(id),
            kind,
            node_id: id.into(),
            label: label.into(),
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
    fn timeline_title_speaks_canvas_route_for_position() {
        let title = locus(LocusKind::Title, "title:hello", "Hello");
        let spoken = utterance(Some(&title), None, Projection::Timeline);
        assert!(spoken.speaks_gap());
        assert!(
            spoken
                .spoken
                .iter()
                .any(|clause| clause.verb == "set-position" && clause.status == "routed")
        );
        assert!(spoken.spoken_text().contains("committed on Canvas"));
        assert!(!spoken.spoken_text().contains("is absent"));
    }

    #[test]
    fn toolbar_split_on_source_is_spoken_not_retargeted() {
        let source = locus(LocusKind::Source, "source:fight", "fight");
        let text = refuse_edit(
            Some(&source),
            &SemanticEdit::Split {
                at: lattice_engine::Time::ZERO,
            },
        );
        assert!(text.contains("needs-scene"), "{text}");
        assert!(text.contains("scene:demo"), "{text}");
        assert!(text.contains("do not retarget"), "{text}");
    }

    #[test]
    fn timeline_does_not_route_set_position_for_title() {
        assert!(!routed_verbs(Projection::Timeline, LocusKind::Title).contains(&"set-position"));
        assert!(routed_verbs(Projection::Canvas, LocusKind::Title).contains(&"set-position"));
    }

    #[test]
    fn source_gain_and_fade_commit_on_toolbar_not_timeline() {
        assert_eq!(
            commit_projection("set-gain", LocusKind::Source),
            Some(Projection::Toolbar)
        );
        assert_eq!(
            commit_projection("set-fade", LocusKind::Source),
            Some(Projection::Toolbar)
        );
        assert_eq!(
            commit_projection("trim", LocusKind::Source),
            Some(Projection::Timeline)
        );
        let source = locus(LocusKind::Source, "source:fight", "fight");
        let spoken = utterance(Some(&source), None, Projection::Timeline);
        assert!(spoken.speaks_gap());
        for verb in ["set-gain", "set-fade"] {
            assert!(
                spoken
                    .spoken
                    .iter()
                    .any(|clause| clause.verb == verb && clause.status == "routed"),
                "{verb} {}",
                spoken.spoken_text()
            );
        }
        assert!(
            spoken.spoken_text().contains("committed on Toolbar"),
            "{}",
            spoken.spoken_text()
        );
        assert!(
            !spoken.spoken_text().contains("committed on Timeline"),
            "gain/fade must not claim Timeline: {}",
            spoken.spoken_text()
        );
    }
}

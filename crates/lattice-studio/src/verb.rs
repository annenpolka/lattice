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

/// A spoken clause: present, routed elsewhere, relation, or absent with a typed reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpokenClause {
    pub verb: String,
    pub status: String,
    pub reason: Option<String>,
    pub target: Option<String>,
    pub scope: Option<String>,
    pub effect: Option<String>,
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

/// Verbs a gesture on `projection` can actually commit for this locus kind.
///
/// This is routing, not legality. A missing route is not an absent edit.
/// Only real commit paths belong here: Timeline trim / overlay time / scene
/// reorder, Canvas geometry, Toolbar trim/gain/fade/split/delete, Inspector
/// title text. Do not list a verb the UI cannot commit on that surface.
#[must_use]
pub fn routed_verbs(projection: Projection, kind: LocusKind) -> Vec<&'static str> {
    match (projection, kind) {
        (Projection::Timeline, LocusKind::Source) => vec!["trim"],
        (Projection::Timeline, LocusKind::Title) => vec!["title"],
        (Projection::Timeline, LocusKind::Callout) => vec!["callout"],
        (Projection::Timeline, LocusKind::Scene) => vec!["reorder-scene"],
        (Projection::Canvas, LocusKind::Title | LocusKind::Callout) => {
            vec!["set-position", "resize-overlay"]
        }
        (Projection::Toolbar, LocusKind::Source) => vec!["trim", "set-gain", "set-fade"],
        (Projection::Toolbar, LocusKind::Scene) => vec!["split", "delete"],
        (Projection::Inspector, LocusKind::Title) => vec!["title"],
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
    loci: &[Locus],
) -> Utterance {
    if let Some(point) = unresolved {
        let spoken = vec![SpokenClause {
            verb: String::new(),
            status: AbsenceReason::UnresolvedPointing.as_str().into(),
            reason: Some(AbsenceReason::UnresolvedPointing.as_str().into()),
            target: None,
            scope: None,
            effect: None,
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
                target: None,
                scope: None,
                effect: None,
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
    let mut spoken = speak_legal_vs_routed(locus, &legal, &routed, projection);
    spoken.extend(speak_relations(locus, loci));
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
        let disclosure = format!(
            "{} → {} ({}: {})",
            edit.verb,
            edit.target.as_str(),
            edit.scope,
            edit.effect
        );
        if routed.iter().any(|verb| verb == &edit.verb) {
            spoken.push(SpokenClause {
                verb: edit.verb.clone(),
                status: "present".into(),
                reason: None,
                target: Some(edit.target.as_str().into()),
                scope: Some(edit.scope.clone()),
                effect: Some(edit.effect.clone()),
                text: format!(
                    "{disclosure} is legal for this {} and this {} gesture commits it.",
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
            target: Some(edit.target.as_str().into()),
            scope: Some(edit.scope.clone()),
            effect: Some(edit.effect.clone()),
            text: format!(
                "{disclosure} is legal for this {}, committed on {} — not implied absent here.",
                kind_label(locus.kind),
                route.as_str()
            ),
        });
    }
    spoken
}

/// Related Scene for a pointed source. Identity-versus-relation only —
/// displaying this never adopts it as here.
#[must_use]
pub fn related_scene<'a>(locus: &Locus, loci: &'a [Locus]) -> Option<&'a Locus> {
    let id = locus.scene_id.as_deref()?;
    loci.iter().find(|candidate| {
        candidate.kind == LocusKind::Scene
            && (candidate.id.as_str() == id || candidate.node_id == id)
    })
}

/// Identity-versus-relation: a displayed container or binding is never adopted as here.
/// Legality, target, scope, and effect come from Engine `legal_edits_for` on the
/// resolved related locus — Studio does not invent a second legal set.
fn speak_relations(locus: &Locus, loci: &[Locus]) -> Vec<SpokenClause> {
    match locus.kind {
        LocusKind::Source => {
            let Some(scene) = related_scene(locus, loci) else {
                return locus.scene_id.as_deref().map_or_else(Vec::new, |id| {
                    vec![SpokenClause {
                        verb: String::new(),
                        status: "relation".into(),
                        reason: None,
                        target: Some(id.into()),
                        scope: Some("relation".into()),
                        effect: Some("Navigate; do not retarget here".into()),
                        text: format!("{id} is a relation, not here. Navigate, do not retarget."),
                    }]
                });
            };
            let legal = legal_edits_for(scene);
            let details = if legal.is_empty() {
                "Engine names no legal edits there".into()
            } else {
                legal
                    .iter()
                    .map(|edit| {
                        format!(
                            "{} → {} ({}: {})",
                            edit.verb,
                            edit.target.as_str(),
                            edit.scope,
                            edit.effect
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            };
            vec![SpokenClause {
                verb: String::new(),
                status: "relation".into(),
                reason: None,
                target: Some(scene.id.as_str().into()),
                scope: Some("relation".into()),
                effect: Some("Navigate to the scene; do not retarget here".into()),
                text: format!(
                    "{} \"{}\" is a relation, not here. {details} — Navigate, do not retarget.",
                    kind_label(scene.kind),
                    scene.label
                ),
            }]
        }
        LocusKind::Scene => vec![SpokenClause {
            verb: String::new(),
            status: "relation".into(),
            reason: Some(AbsenceReason::NeedsSourceBinding.as_str().into()),
            target: None,
            scope: Some("relation".into()),
            effect: Some("Point the video clip; do not retarget here".into()),
            text: "trim, set-gain, set-fade need a source binding. Point the video clip — do not retarget.".into(),
        }],
        _ => Vec::new(),
    }
}

/// Refuse a toolbar/session edit whose target the Engine cannot name for here.
#[must_use]
pub fn refuse_edit(here: Option<&Locus>, edit: &SemanticEdit, loci: &[Locus]) -> String {
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
        ("split" | "delete" | "reorder-scene", LocusKind::Source) => {
            if let Some(scene) = related_scene(locus, loci) {
                legal_edits_for(scene)
                    .iter()
                    .find(|legal| legal.verb == verb)
                    .map(|legal| {
                        format!(
                            " {verb} → {} ({}: {}) — Navigate, do not retarget.",
                            legal.target.as_str(),
                            legal.scope,
                            legal.effect
                        )
                    })
                    .unwrap_or_else(|| " Navigate, do not retarget.".into())
            } else if locus.scene_id.is_some() {
                " Navigate, do not retarget.".into()
            } else {
                String::new()
            }
        }
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

    fn scene() -> Locus {
        locus(LocusKind::Scene, "scene:demo", "demo")
    }

    #[test]
    fn timeline_title_speaks_canvas_route_for_position() {
        let title = locus(LocusKind::Title, "title:hello", "Hello");
        let spoken = utterance(Some(&title), None, Projection::Timeline, &[]);
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
            &[scene()],
        );
        assert!(text.contains("needs-scene"), "{text}");
        assert!(text.contains("split → scene:demo"), "{text}");
        assert!(text.contains("scene:"), "{text}");
        assert!(text.contains("do not retarget"), "{text}");
        assert!(!text.contains("is legal for"), "{text}");
    }

    #[test]
    fn refuse_edit_does_not_claim_scene_legality_without_related_scene() {
        let source = locus(LocusKind::Source, "source:fight", "fight");
        let text = refuse_edit(
            Some(&source),
            &SemanticEdit::Split {
                at: lattice_engine::Time::ZERO,
            },
            &[],
        );
        assert!(text.contains("needs-scene"), "{text}");
        assert!(text.contains("do not retarget"), "{text}");
        assert!(!text.contains("is legal for"), "{text}");
        assert!(!text.contains("split →"), "{text}");
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
        let spoken = utterance(Some(&source), None, Projection::Timeline, &[]);
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

    #[test]
    fn source_utterance_discloses_scene_as_relation_not_absence() {
        let source = locus(LocusKind::Source, "source:fight", "fight");
        let spoken = utterance(Some(&source), None, Projection::Timeline, &[scene()]);
        assert!(
            spoken
                .spoken
                .iter()
                .any(|clause| clause.status == "relation" && clause.text.contains("scene:demo")),
            "{}",
            spoken.spoken_text()
        );
        let text = spoken.spoken_text();
        assert!(text.contains("split → scene:demo"), "{text}");
        assert!(text.contains("delete → scene:demo"), "{text}");
        assert!(text.contains("reorder-scene → scene:demo"), "{text}");
        assert!(text.contains("do not retarget"), "{text}");
        assert!(!text.contains("legal there"), "{text}");
        assert!(!text.contains("split is not legal"), "{text}");
        assert!(spoken.spoken.iter().any(
            |clause| clause.verb == "trim" && clause.target.as_deref() == Some("source:fight")
        ));
    }

    #[test]
    fn unresolved_scene_id_does_not_invent_engine_legality() {
        let source = locus(LocusKind::Source, "source:fight", "fight");
        let spoken = utterance(Some(&source), None, Projection::Timeline, &[]);
        let text = spoken.spoken_text();
        assert!(
            spoken
                .spoken
                .iter()
                .any(|clause| clause.status == "relation" && clause.text.contains("scene:demo")),
            "{text}"
        );
        assert!(text.contains("do not retarget"), "{text}");
        assert!(!text.contains("split →"), "{text}");
        assert!(!text.contains("legal there"), "{text}");
        assert!(!text.contains("are legal"), "{text}");
    }

    #[test]
    fn gesture_routes_match_real_commit_paths() {
        assert_eq!(
            routed_verbs(Projection::Timeline, LocusKind::Scene),
            ["reorder-scene"]
        );
        assert_eq!(
            routed_verbs(Projection::Toolbar, LocusKind::Scene),
            ["split", "delete"]
        );
        assert!(routed_verbs(Projection::Toolbar, LocusKind::Title).is_empty());
        assert_eq!(
            routed_verbs(Projection::Inspector, LocusKind::Title),
            ["title"]
        );
        assert_eq!(
            commit_projection("split", LocusKind::Scene),
            Some(Projection::Toolbar)
        );
        assert_eq!(
            commit_projection("reorder-scene", LocusKind::Scene),
            Some(Projection::Timeline)
        );
        assert_eq!(
            commit_projection("title", LocusKind::Title),
            Some(Projection::Timeline)
        );
    }

    #[test]
    fn timeline_scene_utterance_routes_split_to_toolbar() {
        let spoken = utterance(Some(&scene()), None, Projection::Timeline, &[]);
        assert!(spoken.routed.iter().eq(["reorder-scene"]));
        assert!(
            spoken
                .spoken
                .iter()
                .any(|clause| clause.verb == "split" && clause.status == "routed"),
            "{}",
            spoken.spoken_text()
        );
        assert!(
            spoken
                .spoken
                .iter()
                .any(|clause| clause.verb == "delete" && clause.status == "routed"),
            "{}",
            spoken.spoken_text()
        );
        assert!(
            spoken
                .spoken
                .iter()
                .any(|clause| clause.verb == "reorder-scene" && clause.status == "present"),
            "{}",
            spoken.spoken_text()
        );
        assert!(
            spoken.spoken_text().contains("committed on Toolbar"),
            "{}",
            spoken.spoken_text()
        );
        assert!(
            !spoken
                .spoken
                .iter()
                .any(|clause| clause.verb == "split" && clause.status == "present"),
            "Timeline must not claim it commits split: {}",
            spoken.spoken_text()
        );
    }

    #[test]
    fn scene_utterance_discloses_source_binding_affordance() {
        let spoken = utterance(Some(&scene()), None, Projection::Timeline, &[]);
        assert!(
            spoken.spoken_text().contains("Point the video clip"),
            "{}",
            spoken.spoken_text()
        );
        assert!(
            spoken
                .spoken
                .iter()
                .any(|clause| clause.status == "relation"
                    && clause.reason.as_deref() == Some("needs-source-binding"))
        );
    }
}

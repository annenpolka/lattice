use lattice_core::{
    Locus, LocusId, LocusKind, LocusProjection, PlacementKind, Project, Provenance, Time, Timeline,
    VisualProjection,
};

#[allow(clippy::too_many_lines)]
pub fn loci_from_project(
    project: &Project,
    timeline: &Timeline,
    origin: Option<&str>,
) -> Vec<Locus> {
    let mut loci = Vec::new();
    for media in &project.media {
        loci.push(Locus {
            id: LocusId::new(&media.id),
            kind: LocusKind::Media,
            node_id: media.id.clone(),
            label: media.name.clone(),
            origin: origin.map(ToOwned::to_owned),
            source_span: None,
            scene_id: None,
            sequence_id: None,
            timeline_span: None,
            provenance: Provenance {
                span: None,
                origin: lattice_core::Origin::Source,
            },
            derived_from: None,
            visual: None,
        });
    }
    for sequence in &project.sequences {
        loci.push(Locus {
            id: LocusId::new(&sequence.id),
            kind: LocusKind::Sequence,
            node_id: sequence.id.clone(),
            label: sequence.name.clone(),
            origin: origin.map(ToOwned::to_owned),
            source_span: None,
            scene_id: None,
            sequence_id: Some(sequence.id.clone()),
            timeline_span: None,
            provenance: Provenance {
                span: None,
                origin: lattice_core::Origin::Builtin {
                    name: "flow".into(),
                },
            },
            derived_from: None,
            visual: None,
        });
    }
    for scene in &project.scenes {
        let sequence_id = project
            .sequences
            .iter()
            .find(|sequence| sequence.scene_ids.iter().any(|id| id == &scene.id))
            .map(|sequence| sequence.id.clone());
        loci.push(Locus {
            id: LocusId::new(&scene.id),
            kind: LocusKind::Scene,
            node_id: scene.id.clone(),
            label: scene.name.clone(),
            origin: origin.map(ToOwned::to_owned),
            source_span: None,
            scene_id: Some(scene.id.clone()),
            sequence_id: sequence_id.clone(),
            timeline_span: None,
            provenance: Provenance {
                span: None,
                origin: lattice_core::Origin::Source,
            },
            derived_from: None,
            visual: None,
        });
        for source in &scene.sources {
            loci.push(Locus {
                id: LocusId::new(&source.id),
                kind: LocusKind::Source,
                node_id: source.id.clone(),
                label: source.name.clone(),
                origin: origin.map(ToOwned::to_owned),
                source_span: source.provenance.span,
                scene_id: Some(scene.id.clone()),
                sequence_id: sequence_id.clone(),
                timeline_span: None,
                provenance: source.provenance.clone(),
                derived_from: None,
                visual: None,
            });
        }
        for placement in &scene.placements {
            let clip = timeline.clips.iter().find(|clip| clip.id == placement.id);
            let kind = match placement.kind {
                PlacementKind::Title => LocusKind::Title,
                PlacementKind::Callout => LocusKind::Callout,
                PlacementKind::Audio
                    if matches!(
                        placement.provenance.origin,
                        lattice_core::Origin::Invocation { ref command } if command == "speech"
                    ) =>
                {
                    LocusKind::Speech
                }
                _ => LocusKind::Placement,
            };
            let label = placement
                .visual
                .as_ref()
                .and_then(|visual| visual.text.clone())
                .or_else(|| placement.source_id.clone())
                .unwrap_or_else(|| placement.id.clone());
            loci.push(Locus {
                id: LocusId::new(&placement.id),
                kind,
                node_id: placement.id.clone(),
                label,
                origin: origin.map(ToOwned::to_owned),
                source_span: placement.provenance.span,
                scene_id: Some(scene.id.clone()),
                sequence_id: sequence_id.clone(),
                timeline_span: clip.map(|clip| clip.span),
                provenance: placement.provenance.clone(),
                derived_from: placement
                    .source_id
                    .as_ref()
                    .map(|id| LocusId::new(id.clone())),
                visual: placement.visual.as_ref().map(|visual| VisualProjection {
                    text: visual.text.clone(),
                    fit: visual.fit.clone(),
                    opacity: visual.opacity,
                    position: visual.position,
                    scale: visual.scale,
                }),
            });
        }
    }
    loci
}

pub fn project_locus(locus: &Locus) -> LocusProjection {
    locus.project()
}

pub fn locus_by_id<'a>(loci: &'a [Locus], id: &LocusId) -> Option<&'a Locus> {
    loci.iter().find(|locus| locus.id == *id)
}

pub fn locus_for_node<'a>(loci: &'a [Locus], node_id: &str) -> Option<&'a Locus> {
    loci.iter().find(|locus| locus.node_id == node_id)
}

pub fn locus_at_source(loci: &[Locus], offset: u32) -> Option<&Locus> {
    loci.iter()
        .filter(|locus| locus.contains_source_offset(offset))
        .max_by_key(|locus| {
            let span = locus
                .source_span
                .map_or(u32::MAX, |span| span.end.saturating_sub(span.start).max(1));
            (locus.specificity(), u32::MAX - span)
        })
}

pub fn locus_at_timeline(loci: &[Locus], time: Time) -> Option<&Locus> {
    loci.iter()
        .filter(|locus| locus.contains_timeline_time(time))
        .max_by_key(|locus| {
            let span = locus
                .timeline_span
                .map_or(i64::MAX, |span| span.duration.num().saturating_abs());
            (locus.specificity(), i64::MAX - span)
        })
}

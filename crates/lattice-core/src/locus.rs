use serde::{Deserialize, Serialize};

use crate::ir::TimeSpan;
use crate::provenance::Provenance;
use crate::span::Span;

/// Identity of a semantic "here" shared across VEL, Core, timeline, and agents.
///
/// Guarantees:
/// - Stable within one compilation: the same Core node keeps the same id for
///   the life of that compiled project.
/// - Consistent across source / Core / timeline projections of that compilation.
///
/// After a supported one-property edit (for example changing title text) and a
/// recompile, the edited target is found again by the same `LocusId` when the
/// underlying placement identity is unchanged. Arbitrary source rewrites may
/// mint new ids; that is not guaranteed and must not be assumed.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocusId(pub String);

impl LocusId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for LocusId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for LocusId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

/// What kind of semantic target a locus names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LocusKind {
    Title,
    Callout,
    Source,
    Placement,
    Scene,
    Sequence,
    Media,
    Speech,
}

/// The shared editing target. GPUI types do not belong here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Locus {
    pub id: LocusId,
    pub kind: LocusKind,
    /// Core node this locus names (placement id, source id, scene id, …).
    pub node_id: String,
    pub label: String,
    /// Origin path when compiled from a file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    pub source_span: Option<Span>,
    pub scene_id: Option<String>,
    pub sequence_id: Option<String>,
    pub timeline_span: Option<TimeSpan>,
    pub provenance: Provenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<LocusId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visual: Option<VisualProjection>,
}

/// Visual projection of a locus when the Core node has one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualProjection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<u8>,
}

/// One locus seen from source, Core, and timeline together.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocusProjection {
    pub locus: Locus,
    pub source: Option<SourceProjection>,
    pub core: CoreProjection,
    pub timeline: Option<TimelineProjection>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceProjection {
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreProjection {
    pub node_id: String,
    pub kind: LocusKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineProjection {
    pub clip_id: String,
    pub span: TimeSpan,
}

impl Locus {
    pub fn project(&self) -> LocusProjection {
        LocusProjection {
            source: self.source_span.map(|span| SourceProjection { span }),
            core: CoreProjection {
                node_id: self.node_id.clone(),
                kind: self.kind,
            },
            timeline: self.timeline_span.map(|span| TimelineProjection {
                clip_id: self.node_id.clone(),
                span,
            }),
            locus: self.clone(),
        }
    }

    pub fn contains_source_offset(&self, offset: u32) -> bool {
        self.source_span
            .is_some_and(|span| offset >= span.start && offset < span.end.max(span.start + 1))
    }

    pub fn contains_timeline_time(&self, time: crate::time::Time) -> bool {
        self.timeline_span.is_some_and(|span| span.contains(time))
    }

    pub fn specificity(&self) -> u8 {
        match self.kind {
            LocusKind::Title | LocusKind::Callout | LocusKind::Speech => 4,
            LocusKind::Source => 3,
            LocusKind::Placement => 2,
            LocusKind::Scene => 1,
            LocusKind::Sequence | LocusKind::Media => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::{Origin, Provenance};
    use crate::span::Span;
    use crate::time::Time;

    #[test]
    fn locus_is_json_serializable() {
        let locus = Locus {
            id: LocusId::new("demo:title:1"),
            kind: LocusKind::Title,
            node_id: "demo:title:1".into(),
            label: "Hello".into(),
            origin: Some("main.vel".into()),
            source_span: Some(Span::new(166, 202, 16, 3)),
            scene_id: Some("scene:demo".into()),
            sequence_id: Some("sequence:main".into()),
            timeline_span: Some(TimeSpan::new(Time::seconds(2), Time::seconds(3))),
            provenance: Provenance {
                span: Some(Span::new(166, 202, 16, 3)),
                origin: Origin::Invocation {
                    command: "title".into(),
                },
            },
            derived_from: None,
            visual: Some(VisualProjection {
                text: Some("Hello".into()),
                fit: None,
                opacity: None,
            }),
        };
        let json = serde_json::to_value(&locus).unwrap();
        assert_eq!(json["id"], "demo:title:1");
        assert_eq!(json["kind"], "title");
        assert_eq!(json["node_id"], "demo:title:1");
        let round: Locus = serde_json::from_value(json).unwrap();
        assert_eq!(round.id, locus.id);
        let projection = locus.project();
        assert_eq!(
            projection.core.node_id,
            projection.timeline.unwrap().clip_id
        );
        assert_eq!(projection.source.unwrap().span, locus.source_span.unwrap());
    }
}

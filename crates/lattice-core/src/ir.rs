use serde::{Deserialize, Serialize};

use crate::locator::MediaLocator;
use crate::overlay::OverlayStyle;
use crate::provenance::Provenance;
use crate::space::{NormalizedPosition, NormalizedScale};
use crate::time::Time;
use crate::time_map::TimeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub schema_version: u32,
    pub name: String,
    pub convention: Option<String>,
    pub media: Vec<Media>,
    pub sequences: Vec<Sequence>,
    pub scenes: Vec<Scene>,
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            schema_version: 1,
            name: name.into(),
            convention: None,
            media: Vec::new(),
            sequences: Vec::new(),
            scenes: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Media {
    pub id: String,
    pub name: String,
    pub locator: MediaLocator,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sequence {
    pub id: String,
    pub name: String,
    /// Scene ids in flow order.
    pub scene_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub over: Option<String>,
    pub duration: Time,
    pub sources: Vec<Source>,
    pub placements: Vec<Placement>,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub media_name: String,
    pub source_range: TimeSpan,
    pub time_map: TimeMap,
    pub provenance: Provenance,
    /// Generated sources (speech, etc.) are not commentary A/V fill targets.
    #[serde(default, skip_serializing_if = "is_false")]
    pub generated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeSpan {
    pub start: Time,
    pub duration: Time,
}

impl TimeSpan {
    pub fn new(start: Time, duration: Time) -> Self {
        Self { start, duration }
    }

    pub fn end(&self) -> Time {
        self.start + self.duration
    }

    /// Half-open `[start, end)`, or `{start}` when the span is empty.
    #[must_use]
    pub fn contains(self, time: Time) -> bool {
        time >= self.start && (time < self.end() || (self.duration.is_zero() && time == self.start))
    }

    /// Split at an exclusive interior time. `at` must lie in `(start, end)`.
    pub fn split_at(self, at: Time) -> Option<(Self, Self)> {
        if at <= self.start || at >= self.end() {
            return None;
        }
        let left = at.checked_sub(self.start).ok()?;
        let right = self.end().checked_sub(at).ok()?;
        Some((Self::new(self.start, left), Self::new(at, right)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlacementKind {
    Video,
    Audio,
    Title,
    Callout,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    pub id: String,
    pub kind: PlacementKind,
    pub source_id: Option<String>,
    pub span: TimeSpan,
    pub visual: Option<Visual>,
    pub audio: Option<Audio>,
    pub provenance: Provenance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Visual {
    pub fit: Option<String>,
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_in: Option<Time>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_out: Option<Time>,
    /// Absolute top-left in normalized Canvas Space. `None` keeps the
    /// renderer's conventional title/callout placement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<NormalizedPosition>,
    /// Uniform aspect-preserving scale (`1000` = `100%`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<NormalizedScale>,
    /// Scale / placement pivot. `None` is today's top-left. Not `origin`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<crate::overlay::OverlayAnchor>,
    /// Explicit overlay color / size / weight / family / bar. Omitted style
    /// keeps today's title/callout convention at evaluate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<OverlayStyle>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::Time;

    #[test]
    fn omitted_overlay_style_is_absent_from_visual_json() {
        let json = serde_json::to_string(&Visual::text_overlay("Hello")).expect("json");
        assert!(
            !json.contains("style"),
            "omitted overlay style must skip: {json}"
        );
        let round: Visual = serde_json::from_str(&json).expect("roundtrip");
        assert_eq!(round.style, None);
        assert_eq!(round.anchor, None);
        assert!(
            !json.contains("anchor"),
            "omitted overlay anchor must skip: {json}"
        );
        assert!(
            !json.contains("origin"),
            "Visual must not grow an origin field: {json}"
        );
    }
}

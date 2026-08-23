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
    fn split_at_preserves_coverage() {
        let span = TimeSpan::new(Time::seconds(0), Time::seconds(20));
        let (left, right) = span.split_at(Time::seconds(8)).expect("interior");
        assert_eq!(left, TimeSpan::new(Time::seconds(0), Time::seconds(8)));
        assert_eq!(right, TimeSpan::new(Time::seconds(8), Time::seconds(12)));
        assert_eq!(left.end(), right.start);
        assert_eq!(left.start, span.start);
        assert_eq!(right.end(), span.end());
    }

    #[test]
    fn split_at_rejects_boundaries() {
        let span = TimeSpan::new(Time::seconds(10), Time::seconds(10));
        assert!(span.split_at(Time::seconds(10)).is_none());
        assert!(span.split_at(Time::seconds(20)).is_none());
        assert!(span.split_at(Time::seconds(9)).is_none());
    }

    #[test]
    fn contains_is_half_open() {
        let span = TimeSpan::new(Time::seconds(2), Time::seconds(3));
        assert!(span.contains(Time::seconds(2)));
        assert!(span.contains(Time::seconds(4)));
        assert!(!span.contains(Time::seconds(5)));
        assert!(!span.contains(Time::ZERO));
        let empty = TimeSpan::new(Time::seconds(3), Time::ZERO);
        assert!(empty.contains(Time::seconds(3)));
        assert!(!empty.contains(Time::seconds(4)));
    }

    #[test]
    fn omitted_overlay_style_is_absent_from_visual_json() {
        let json = serde_json::to_string(&Visual::text_overlay("Hello")).expect("json");
        assert!(
            !json.contains("style"),
            "omitted overlay style must skip: {json}"
        );
        let round: Visual = serde_json::from_str(&json).expect("roundtrip");
        assert_eq!(round.style, None);
    }
}

#[cfg(test)]
mod properties {
    use super::*;
    use crate::time::Time;
    use proptest::prelude::*;

    fn arb_span() -> impl Strategy<Value = TimeSpan> {
        (0i64..1_000, 1i64..1_000).prop_map(|(start, duration)| {
            TimeSpan::new(Time::seconds(start), Time::seconds(duration))
        })
    }

    proptest! {
        #[test]
        fn split_preserves_source_coverage(span in arb_span(), offset in 1i64..999) {
            let at = span.start + Time::seconds(offset);
            let Some((left, right)) = span.split_at(at) else {
                prop_assert!(at <= span.start || at >= span.end());
                return Ok(());
            };
            prop_assert_eq!(left.start, span.start);
            prop_assert_eq!(right.end(), span.end());
            prop_assert_eq!(left.end(), right.start);
            prop_assert_eq!(left.duration + right.duration, span.duration);
            prop_assert!(left.duration > Time::ZERO);
            prop_assert!(right.duration > Time::ZERO);
        }

        #[test]
        fn trim_stays_inside_original(span in arb_span(), in_off in 0i64..500, out_off in 0i64..500) {
            let in_point = span.start + Time::seconds(in_off);
            let out_point = span.end().checked_sub(Time::seconds(out_off)).unwrap_or(span.start);
            if in_point >= out_point || in_point < span.start || out_point > span.end() {
                return Ok(());
            }
            let trimmed = TimeSpan::new(in_point, out_point - in_point);
            prop_assert!(trimmed.start >= span.start);
            prop_assert!(trimmed.end() <= span.end());
        }
    }
}

impl Visual {
    pub fn text_overlay(text: impl Into<String>) -> Self {
        Self {
            fit: None,
            text: Some(text.into()),
            opacity: None,
            fade_in: None,
            fade_out: None,
            position: None,
            scale: None,
            style: None,
        }
    }

    pub fn fit(fit: impl Into<String>) -> Self {
        Self {
            fit: Some(fit.into()),
            text: None,
            opacity: None,
            fade_in: None,
            fade_out: None,
            position: None,
            scale: None,
            style: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Audio {
    pub gain_db: Option<i32>,
}

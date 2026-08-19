use serde::{Deserialize, Serialize};

use crate::locator::MediaLocator;
use crate::provenance::Provenance;
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub media_name: String,
    pub source_range: TimeSpan,
    pub time_map: TimeMap,
    pub provenance: Provenance,
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlacementKind {
    Video,
    Audio,
    Title,
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
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Audio {
    pub gain_db: Option<i32>,
}

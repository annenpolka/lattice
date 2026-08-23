use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ir::{PlacementKind, Project, TimeSpan};
use crate::locator::MediaLocator;
use crate::overlay::{OverlayAnchor, OverlayStyle};
use crate::time::{Time, TimeError};
use crate::time_map::TimeMap;
use crate::{NormalizedPosition, NormalizedScale};

/// Flattened editorial timeline. Pure function of compiled Core IR.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timeline {
    pub duration: Time,
    pub clips: Vec<TimelineClip>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineClip {
    pub id: String,
    pub kind: PlacementKind,
    pub span: TimeSpan,
    pub source: Option<TimelineSource>,
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_in: Option<crate::time::Time>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_out: Option<crate::time::Time>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<NormalizedPosition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<NormalizedScale>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<OverlayAnchor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<OverlayStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gain_db: Option<i32>,
}

use serde::{Deserialize, Serialize};

use crate::locus::LocusId;
use crate::time::Time;

/// A semantic change, named before any source rewrite.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SemanticEdit {
    Title {
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        at: Option<Time>,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration: Option<Time>,
        #[serde(skip_serializing_if = "Option::is_none")]
        opacity: Option<u8>,
    },
}

/// Inspectable proposal. Current VEL is unchanged until Apply writes `new_source`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditProposal {
    pub locus_id: LocusId,
    pub description: String,
    pub edit: SemanticEdit,
    pub vel_diff: String,
    pub new_source: String,
}

impl SemanticEdit {
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Title {
                text,
                at,
                duration,
                opacity,
            } => text.is_none() && at.is_none() && duration.is_none() && opacity.is_none(),
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Self::Title {
                text,
                at,
                duration,
                opacity,
            } => {
                let mut parts = Vec::new();
                if let Some(text) = text {
                    parts.push(format!("text {text:?}"));
                }
                if let Some(at) = at {
                    parts.push(format!("at {at}"));
                }
                if let Some(duration) = duration {
                    parts.push(format!("for {duration}"));
                }
                if let Some(opacity) = opacity {
                    parts.push(format!("opacity {opacity}"));
                }
                if parts.is_empty() {
                    "no title change".into()
                } else {
                    format!("set title {}", parts.join(", "))
                }
            }
        }
    }
}

use serde::{Deserialize, Serialize};

use crate::locus::LocusId;
use crate::time::Time;

/// Fingerprint of VEL source bytes a proposal was created against.
pub fn source_revision(source: &str) -> String {
    fnv_hex(source.as_bytes())
}

fn fnv_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    format!("{hash:016x}")
}

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
    Trim {
        #[serde(skip_serializing_if = "Option::is_none")]
        in_point: Option<Time>,
        #[serde(skip_serializing_if = "Option::is_none")]
        out_point: Option<Time>,
    },
    Split {
        at: Time,
    },
    Delete,
    SetGain {
        db: i32,
    },
    SetFade {
        #[serde(skip_serializing_if = "Option::is_none")]
        fade_in: Option<Time>,
    },
}

/// Inspectable proposal. Current VEL is unchanged until Apply writes `new_source`.
///
/// `base_revision` is a fingerprint of the source the proposal was built from.
/// Apply must reject the proposal when the current source no longer matches.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditProposal {
    pub locus_id: LocusId,
    pub description: String,
    pub edit: SemanticEdit,
    pub vel_diff: String,
    pub new_source: String,
    /// FNV-1a of the source bytes this proposal was created against.
    #[serde(default)]
    pub base_revision: String,
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
            Self::Trim {
                in_point,
                out_point,
            } => in_point.is_none() && out_point.is_none(),
            Self::SetFade { fade_in } => fade_in.is_none(),
            Self::Split { .. } | Self::Delete | Self::SetGain { .. } => false,
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
            Self::Trim {
                in_point,
                out_point,
            } => {
                let mut parts = Vec::new();
                if let Some(in_point) = in_point {
                    parts.push(format!("in {in_point}"));
                }
                if let Some(out_point) = out_point {
                    parts.push(format!("out {out_point}"));
                }
                if parts.is_empty() {
                    "no trim change".into()
                } else {
                    format!("trim {}", parts.join(", "))
                }
            }
            Self::Split { at } => format!("split at {at}"),
            Self::Delete => "delete clip".into(),
            Self::SetGain { db } => format!("set gain {db} dB"),
            Self::SetFade { fade_in } => match fade_in {
                Some(time) => format!("set fade in {time}"),
                None => "no fade change".into(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revision_changes_when_source_changes() {
        assert_ne!(source_revision("a"), source_revision("b"));
        assert_eq!(source_revision("same"), source_revision("same"));
    }
}

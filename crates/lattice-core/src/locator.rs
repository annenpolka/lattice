use serde::{Deserialize, Serialize};

/// Where media bytes come from. Core stores the VEL spelling; OS paths are a media concern.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum MediaLocator {
    File { path: String },
    Url { url: String },
    Generated { generator: String, key: String },
}

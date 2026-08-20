use serde::{Deserialize, Serialize};

use crate::locator::MediaLocator;
use crate::time::Time;

/// Stable identity of a resolved artifact (content hash, not a path).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetIdentity(pub String);

impl AssetIdentity {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One materialized input, locked for reproducible render.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedAsset {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
    pub key: String,
    pub path: String,
    pub identity: AssetIdentity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<Time>,
    /// Provider implementation identity used when this asset was generated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_version: Option<String>,
}

/// Git-friendly resolve lock. Render from this must not silently regenerate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveLock {
    pub schema_version: u32,
    pub assets: Vec<LockedAsset>,
}

impl ResolveLock {
    pub fn new() -> Self {
        Self {
            schema_version: 1,
            assets: Vec::new(),
        }
    }

    pub fn get(&self, generator: Option<&str>, key: &str) -> Option<&LockedAsset> {
        self.assets
            .iter()
            .find(|asset| asset.generator.as_deref() == generator && asset.key == key)
    }
}

impl Default for ResolveLock {
    fn default() -> Self {
        Self::new()
    }
}

/// How a media locator was materialized.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedAsset {
    pub id: String,
    pub locator: MediaLocator,
    pub path: String,
    pub identity: AssetIdentity,
    pub from_lock: bool,
}

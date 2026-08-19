use std::path::{Path, PathBuf};

use lattice_core::{
    AssetIdentity, Diagnostic, LockedAsset, MediaLocator, Project, ResolveLock, ResolvedAsset,
    Severity, Time,
};
use thiserror::Error;

use crate::time_eval::TimeEvalError;

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Time(#[from] TimeEvalError),
}

pub trait GeneratedMediaProvider {
    fn name(&self) -> &'static str;
    fn generate(&mut self, request: &GenerateRequest) -> Result<Vec<u8>, ResolveError>;
}

#[derive(Clone, Debug)]
pub struct GenerateRequest {
    pub generator: String,
    pub key: String,
    pub duration: Time,
}

#[derive(Clone, Debug)]
pub struct ResolveOptions<'a> {
    pub media_root: &'a Path,
    pub artifact_dir: &'a Path,
    pub lock: Option<&'a ResolveLock>,
}

#[derive(Clone, Debug)]
pub struct Resolution {
    pub lock: ResolveLock,
    pub assets: Vec<ResolvedAsset>,
    pub diagnostics: Vec<Diagnostic>,
    pub provider_calls: usize,
}

/// Local generated-media provider: deterministic PCM tone, not a remote TTS.
pub struct LocalToneProvider;

impl GeneratedMediaProvider for LocalToneProvider {
    fn name(&self) -> &'static str {
        "speech-local-tone"
    }

    fn generate(&mut self, request: &GenerateRequest) -> Result<Vec<u8>, ResolveError> {
        Ok(tone_wav(request.duration, 440))
    }
}

#[allow(clippy::too_many_lines)]
pub fn resolve_project(
    project: &Project,
    options: &ResolveOptions<'_>,
    provider: &mut dyn GeneratedMediaProvider,
) -> Result<Resolution, ResolveError> {
    let mut lock = ResolveLock::new();
    let mut assets = Vec::new();
    let mut diagnostics = Vec::new();
    let mut provider_calls = 0;
    std::fs::create_dir_all(options.artifact_dir)?;
    for media in &project.media {
        match &media.locator {
            MediaLocator::File { path } => {
                let resolved = resolve_file_path(path, options.media_root);
                if resolved.is_file() {
                    let bytes = std::fs::read(&resolved)?;
                    let identity = AssetIdentity::new(fnv_hex(&bytes));
                    let asset = LockedAsset {
                        id: media.id.clone(),
                        generator: None,
                        key: path.clone(),
                        path: resolved.display().to_string(),
                        identity: identity.clone(),
                        duration: None,
                    };
                    assets.push(ResolvedAsset {
                        id: media.id.clone(),
                        locator: media.locator.clone(),
                        path: asset.path.clone(),
                        identity,
                        from_lock: false,
                    });
                    lock.assets.push(asset);
                } else {
                    diagnostics.push(Diagnostic {
                        code: "LAT-RES-001".into(),
                        severity: Severity::Warning,
                        message: format!("media `{path}` is missing at {}", resolved.display()),
                        span: None,
                    });
                }
            }
            MediaLocator::Generated { generator, key } => {
                let duration = generated_duration(project, &media.name).unwrap_or(Time::seconds(2));
                let locked = options.lock.and_then(|lock| {
                    lock.get(Some(generator.as_str()), key)
                        .filter(|asset| Path::new(&asset.path).is_file())
                        .cloned()
                });
                if let Some(locked) = locked {
                    let bytes = std::fs::read(&locked.path)?;
                    let identity = AssetIdentity::new(fnv_hex(&bytes));
                    if identity == locked.identity {
                        assets.push(ResolvedAsset {
                            id: media.id.clone(),
                            locator: media.locator.clone(),
                            path: locked.path.clone(),
                            identity,
                            from_lock: true,
                        });
                        lock.assets.push(locked);
                        continue;
                    }
                    diagnostics.push(Diagnostic {
                        code: "LAT-RES-002".into(),
                        severity: Severity::Warning,
                        message: format!("lock for `{key}` is stale; regenerating"),
                        span: None,
                    });
                }
                let bytes = provider.generate(&GenerateRequest {
                    generator: generator.clone(),
                    key: key.clone(),
                    duration,
                })?;
                provider_calls += 1;
                let identity = AssetIdentity::new(fnv_hex(&bytes));
                let path = options
                    .artifact_dir
                    .join(format!("{}-{}.wav", generator, slug(key)));
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, &bytes)?;
                let asset = LockedAsset {
                    id: media.id.clone(),
                    generator: Some(generator.clone()),
                    key: key.clone(),
                    path: path.display().to_string(),
                    identity: identity.clone(),
                    duration: Some(duration),
                };
                assets.push(ResolvedAsset {
                    id: media.id.clone(),
                    locator: media.locator.clone(),
                    path: asset.path.clone(),
                    identity,
                    from_lock: false,
                });
                lock.assets.push(asset);
            }
            MediaLocator::Url { url } => {
                diagnostics.push(Diagnostic {
                    code: "LAT-RES-003".into(),
                    severity: Severity::Warning,
                    message: format!("url locator `{url}` is not resolved in this milestone"),
                    span: None,
                });
            }
        }
    }
    Ok(Resolution {
        lock,
        assets,
        diagnostics,
        provider_calls,
    })
}

fn resolve_file_path(path: &str, media_root: &Path) -> PathBuf {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        media_root.join(candidate)
    }
}

fn generated_duration(project: &Project, media_name: &str) -> Option<Time> {
    for scene in &project.scenes {
        if let Some(source) = scene
            .sources
            .iter()
            .find(|source| source.media_name == media_name)
        {
            return Some(source.time_map.duration);
        }
    }
    None
}

fn slug(text: &str) -> String {
    let mut slug = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
        if slug.len() >= 40 {
            break;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "generated".into()
    } else {
        slug
    }
}

fn fnv_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    format!("{hash:016x}")
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn tone_wav(duration: Time, freq_hz: u32) -> Vec<u8> {
    const SAMPLE_RATE: u32 = 8000;
    let seconds = if duration.den() == 0 {
        1.0
    } else {
        duration.num() as f64 / duration.den() as f64
    };
    let n = ((seconds.max(0.05) * f64::from(SAMPLE_RATE)).round() as usize).max(1);
    let mut data = Vec::with_capacity(n * 2);
    for i in 0..n {
        let t = f64::from(i as u32) / f64::from(SAMPLE_RATE);
        let sample = (2.0 * std::f64::consts::PI * f64::from(freq_hz) * t).sin();
        let amp = (sample * 0.25 * 32767.0) as i16;
        data.extend_from_slice(&amp.to_le_bytes());
    }
    let mut wav = Vec::new();
    let data_len = data.len() as u32;
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(&data);
    wav
}

/// Counting wrapper so tests can prove Compile never calls the provider
/// and a second Resolve against a lock does not regenerate.
pub struct CountingProvider<P> {
    pub inner: P,
    pub calls: usize,
}

impl<P: GeneratedMediaProvider> CountingProvider<P> {
    pub fn new(inner: P) -> Self {
        Self { inner, calls: 0 }
    }
}

impl<P: GeneratedMediaProvider> GeneratedMediaProvider for CountingProvider<P> {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn generate(&mut self, request: &GenerateRequest) -> Result<Vec<u8>, ResolveError> {
        self.calls += 1;
        self.inner.generate(request)
    }
}

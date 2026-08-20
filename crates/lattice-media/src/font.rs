//! Project-local font resolve + lock identity. Render does not rescan when a lock is valid.

use std::path::{Path, PathBuf};

use lattice_core::{AssetIdentity, FontIdentity, FontSource, FontSpec, LockedAsset, ResolveLock};

use crate::export::ExportError;
use crate::probe::find_font;

const FIXTURE_NAME: &str = "MPLUS1p-Regular.ttf";

#[derive(Clone, Debug)]
pub struct FontResolution {
    pub identity: FontIdentity,
    pub bytes: Vec<u8>,
    pub diagnostic: Option<String>,
}

pub fn fixture_font_path() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(env) = std::env::var("LATTICE_FONT") {
        candidates.push(PathBuf::from(env));
    }
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/fonts")
            .join(FIXTURE_NAME),
    );
    candidates.push(PathBuf::from("fixtures/fonts").join(FIXTURE_NAME));
    candidates.into_iter().find(|path| path.is_file())
}

pub fn font_bytes_identity(bytes: &[u8]) -> AssetIdentity {
    AssetIdentity::new(fnv_hex(bytes))
}

fn fnv_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn relative_to(path: &Path, root: &Path) -> Option<String> {
    path.strip_prefix(root).ok().map(|rel| {
        rel.components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    })
}

fn lock_load_path(stored: &str, media_root: &Path) -> PathBuf {
    let candidate = Path::new(stored);
    if candidate.is_absolute() {
        return candidate.to_path_buf();
    }
    media_root.join(candidate)
}

fn load_from_path(
    path: &Path,
    source: FontSource,
    stored: String,
) -> Result<FontResolution, ExportError> {
    let bytes = std::fs::read(path)?;
    Ok(FontResolution {
        identity: FontIdentity {
            path: stored,
            face_index: 0,
            identity: font_bytes_identity(&bytes),
            source,
        },
        bytes,
        diagnostic: None,
    })
}

fn font_assets(lock: &ResolveLock) -> impl Iterator<Item = &LockedAsset> {
    lock.assets
        .iter()
        .filter(|asset| asset.generator.as_deref() == Some("font"))
}

/// Render-time resolve. A valid font lock pins the face: no `fonts/` rescan, no fixture/system fallback.
pub fn resolve_font(
    spec: &FontSpec,
    media_root: &Path,
    lock: Option<&ResolveLock>,
    override_path: Option<&Path>,
) -> Result<FontResolution, ExportError> {
    if let Some(lock) = lock {
        let locked: Vec<_> = font_assets(lock).collect();
        if !locked.is_empty() {
            return load_locked_font(&locked, media_root);
        }
    }
    if let Some(path) = override_path {
        if path.is_file() {
            let stored =
                relative_to(path, media_root).unwrap_or_else(|| path.display().to_string());
            return load_from_path(path, FontSource::ProjectLocal, stored);
        }
        return Err(ExportError::MissingFont);
    }
    discover_font(spec, media_root)
}

fn load_locked_font(
    assets: &[&LockedAsset],
    media_root: &Path,
) -> Result<FontResolution, ExportError> {
    let asset = assets[0];
    let path = lock_load_path(&asset.path, media_root);
    if !path.is_file() {
        return Err(ExportError::MissingFont);
    }
    let resolved = load_from_path(&path, FontSource::Lock, asset.path.clone())?;
    if resolved.identity.identity != asset.identity {
        return Err(ExportError::StaleFont(asset.path.clone()));
    }
    Ok(resolved)
}

fn discover_font(spec: &FontSpec, media_root: &Path) -> Result<FontResolution, ExportError> {
    for dir in [media_root.join("fonts"), media_root.join("assets")] {
        if let Some(path) = first_ttf(&dir)
            && let Some(stored) = relative_to(&path, media_root)
        {
            return load_from_path(&path, FontSource::ProjectLocal, stored);
        }
    }
    if let Some(path) = fixture_font_path() {
        return load_from_path(&path, FontSource::Fixture, path.display().to_string());
    }
    if let Some(path) = find_font() {
        let mut resolved = load_from_path(
            &path,
            FontSource::System { portable: false },
            path.display().to_string(),
        )?;
        resolved.diagnostic = Some(format!(
            "font `{}` used a system face {}; render is not portable",
            spec.family,
            path.display()
        ));
        return Ok(resolved);
    }
    Err(ExportError::MissingFont)
}

fn first_ttf(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if path.is_file() && (ext.eq_ignore_ascii_case("ttf") || ext.eq_ignore_ascii_case("otf")) {
            return Some(path);
        }
    }
    None
}

fn copy_into_project_fonts(
    src: &Path,
    bytes: &[u8],
    media_root: &Path,
) -> Result<PathBuf, ExportError> {
    let dir = media_root.join("fonts");
    std::fs::create_dir_all(&dir)?;
    let name = src.file_name().map_or_else(
        || PathBuf::from(format!("{}.ttf", fnv_hex(bytes))),
        PathBuf::from,
    );
    let dest = dir.join(&name);
    if dest.is_file() {
        let existing = std::fs::read(&dest)?;
        if font_bytes_identity(&existing) == font_bytes_identity(bytes) {
            return Ok(dest);
        }
        let dest = dir.join(format!("{}.ttf", fnv_hex(bytes)));
        if !dest.is_file() {
            std::fs::write(&dest, bytes)?;
        }
        return Ok(dest);
    }
    std::fs::write(&dest, bytes)?;
    Ok(dest)
}

/// Resolve-time materialize: copy a discovered face under `media_root/fonts/` and
/// return a lockable relative path. A valid existing lock is reused as-is.
pub fn materialize_font_for_lock(
    spec: &FontSpec,
    media_root: &Path,
    lock: Option<&ResolveLock>,
) -> Result<FontResolution, ExportError> {
    if let Some(lock) = lock {
        let locked: Vec<_> = font_assets(lock).collect();
        if !locked.is_empty() {
            return load_locked_font(&locked, media_root);
        }
    }
    let discovered = discover_font(spec, media_root)?;
    if relative_to(Path::new(&discovered.identity.path), media_root).is_some()
        || Path::new(&discovered.identity.path).starts_with("fonts/")
    {
        return Ok(discovered);
    }
    let src = PathBuf::from(&discovered.identity.path);
    let dest = copy_into_project_fonts(&src, &discovered.bytes, media_root)?;
    let stored = relative_to(&dest, media_root).ok_or(ExportError::MissingFont)?;
    Ok(FontResolution {
        identity: FontIdentity {
            path: stored,
            face_index: discovered.identity.face_index,
            identity: discovered.identity.identity,
            source: FontSource::ProjectLocal,
        },
        bytes: discovered.bytes,
        diagnostic: discovered.diagnostic,
    })
}

pub fn locked_font_asset(identity: &FontIdentity, media_root: &Path) -> LockedAsset {
    let stored = if Path::new(&identity.path).is_absolute() {
        relative_to(Path::new(&identity.path), media_root).unwrap_or_else(|| identity.path.clone())
    } else {
        identity.path.replace('\\', "/")
    };
    LockedAsset {
        id: format!("font:{}", identity.identity.as_str()),
        generator: Some("font".into()),
        key: stored.clone(),
        path: stored,
        identity: identity.identity.clone(),
        duration: None,
        provider: None,
        provider_version: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::FontSpec;

    fn spec() -> FontSpec {
        FontSpec::preview_sans(16)
    }

    #[test]
    fn fixture_or_env_font_has_identity() {
        let root = std::env::temp_dir().join("lattice-font-none");
        let _ = std::fs::create_dir_all(&root);
        match resolve_font(&spec(), &root, None, None) {
            Ok(resolved) => {
                assert!(!resolved.bytes.is_empty());
                assert!(!resolved.identity.identity.as_str().is_empty());
            }
            Err(ExportError::MissingFont) => {}
            Err(err) => panic!("{err}"),
        }
    }

    #[test]
    fn stale_lock_hash_fails() {
        let dir = std::env::temp_dir().join("lattice-stale-font");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("fonts")).unwrap();
        let font = dir.join("fonts/face.ttf");
        std::fs::write(&font, b"not-a-real-font-but-hashed").unwrap();
        let lock = ResolveLock {
            schema_version: 1,
            assets: vec![LockedAsset {
                id: "font:x".into(),
                generator: Some("font".into()),
                key: "fonts/face.ttf".into(),
                path: "fonts/face.ttf".into(),
                identity: AssetIdentity::new("deadbeef"),
                duration: None,
                provider: None,
                provider_version: None,
            }],
        };
        let err = resolve_font(&spec(), &dir, Some(&lock), None).unwrap_err();
        assert!(matches!(err, ExportError::StaleFont(_)), "{err}");
    }

    #[test]
    fn valid_lock_pins_face_and_ignores_other_ttfs() {
        let fixture = fixture_font_path().expect("repo fixture font");
        let bytes = std::fs::read(&fixture).unwrap();
        let hash = font_bytes_identity(&bytes);
        let dir = std::env::temp_dir().join("lattice-font-lock-pin");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("fonts")).unwrap();
        std::fs::write(dir.join("fonts/locked.ttf"), &bytes).unwrap();
        std::fs::write(dir.join("fonts/aaa-other.ttf"), b"other-face-bytes").unwrap();
        let lock = ResolveLock {
            schema_version: 1,
            assets: vec![LockedAsset {
                id: "font:locked".into(),
                generator: Some("font".into()),
                key: "fonts/locked.ttf".into(),
                path: "fonts/locked.ttf".into(),
                identity: hash.clone(),
                duration: None,
                provider: None,
                provider_version: None,
            }],
        };
        let resolved = resolve_font(&spec(), &dir, Some(&lock), None).expect("lock font");
        assert_eq!(resolved.identity.identity, hash);
        assert_eq!(resolved.identity.path, "fonts/locked.ttf");
        assert_eq!(resolved.identity.source, FontSource::Lock);
    }

    #[test]
    fn materialize_writes_relative_project_font() {
        let dir = std::env::temp_dir().join("lattice-font-materialize");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let resolved = materialize_font_for_lock(&spec(), &dir, None).expect("fixture copy");
        assert!(
            !Path::new(&resolved.identity.path).is_absolute(),
            "{}",
            resolved.identity.path
        );
        assert!(
            resolved.identity.path.starts_with("fonts/"),
            "{}",
            resolved.identity.path
        );
        assert!(dir.join(&resolved.identity.path).is_file());
        let asset = locked_font_asset(&resolved.identity, &dir);
        assert!(!Path::new(&asset.path).is_absolute(), "{}", asset.path);
        assert_eq!(asset.path, resolved.identity.path);
    }

    #[test]
    fn missing_lock_file_does_not_fall_through_to_scan() {
        let dir = std::env::temp_dir().join("lattice-font-missing-lock");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("fonts")).unwrap();
        if let Some(fixture) = fixture_font_path() {
            std::fs::copy(&fixture, dir.join("fonts/visible.ttf")).unwrap();
        }
        let lock = ResolveLock {
            schema_version: 1,
            assets: vec![LockedAsset {
                id: "font:gone".into(),
                generator: Some("font".into()),
                key: "fonts/gone.ttf".into(),
                path: "fonts/gone.ttf".into(),
                identity: AssetIdentity::new("abc"),
                duration: None,
                provider: None,
                provider_version: None,
            }],
        };
        let err = resolve_font(&spec(), &dir, Some(&lock), None).unwrap_err();
        assert!(matches!(err, ExportError::MissingFont), "{err}");
    }
}

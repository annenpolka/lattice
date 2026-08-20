use std::path::Path;

/// Write `contents` beside `path` then replace. A failed write cannot truncate `path`.
pub fn write_source_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    atomic_replace(path, contents.as_bytes(), true)
}

/// Same as [`write_source_atomic`] but leaves `path` untouched after writing the temp file.
/// Used to prove a mid-write failure cannot truncate the live `.vel`.
pub fn write_source_atomic_no_commit(path: &Path, contents: &str) -> std::io::Result<()> {
    atomic_replace(path, contents.as_bytes(), false)
}

fn atomic_replace(path: &Path, data: &[u8], commit: bool) -> std::io::Result<()> {
    let dir = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    let dir = dir.unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("source.vel");
    let tmp = dir.join(format!(".{name}.part"));
    std::fs::write(&tmp, data)?;
    if !commit {
        let _ = std::fs::remove_file(&tmp);
        return Err(std::io::Error::other("simulated mid-write failure"));
    }
    replace_file(&tmp, path)
}

fn replace_file(tmp: &Path, dest: &Path) -> std::io::Result<()> {
    if dest.exists() {
        let backup = dest.with_extension("vel.bak");
        std::fs::rename(dest, &backup)?;
        match std::fs::rename(tmp, dest) {
            Ok(()) => {
                let _ = std::fs::remove_file(&backup);
                Ok(())
            }
            Err(err) => {
                let _ = std::fs::rename(&backup, dest);
                let _ = std::fs::remove_file(tmp);
                Err(err)
            }
        }
    } else {
        std::fs::rename(tmp, dest)
    }
}

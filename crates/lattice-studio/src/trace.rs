//! Durable Studio diagnostics.
//!
//! `eprintln!` panics when stderr is a closed Windows pipe (`0x800700e8`).
//! The process log is a file; stderr is best-effort and never a panic path.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, Once};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_LOG_BYTES: u64 = 8 * 1024 * 1024;

struct LogState {
    path: PathBuf,
    file: Option<File>,
}

static STATE: Mutex<Option<LogState>> = Mutex::new(None);
static HOOK: Once = Once::new();

/// Default log: `%LOCALAPPDATA%/lattice/studio.log`, else temp.
#[must_use]
pub fn default_log_path() -> PathBuf {
    if let Some(path) = std::env::var_os("LATTICE_STUDIO_LOG") {
        return PathBuf::from(path);
    }
    let dir = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("TEMP").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir)
        .join("lattice");
    dir.join("studio.log")
}

/// Install the file sink and panic hook. Safe to call more than once.
pub fn install() -> PathBuf {
    install_to(&default_log_path())
}

/// Install using an explicit path (tests and the debug launcher).
pub fn install_to(path: &Path) -> PathBuf {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    rotate_if_huge(path);
    let file = OpenOptions::new().create(true).append(true).open(path).ok();
    if let Ok(mut guard) = STATE.lock() {
        *guard = Some(LogState {
            path: path.to_path_buf(),
            file,
        });
    }
    HOOK.call_once(install_panic_hook);
    path.to_path_buf()
}

#[must_use]
pub fn log_path() -> Option<PathBuf> {
    STATE
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|state| state.path.clone()))
}

/// Append one line. Never panics on a closed stderr pipe.
pub fn log(message: impl AsRef<str>) {
    let line = format_line(message.as_ref());
    write_file(&line);
    let _ = writeln!(io::stderr(), "{line}");
}

fn format_line(message: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!(
        "{}.{:03} pid={} tid={:?} {message}",
        now.as_secs(),
        now.subsec_millis(),
        std::process::id(),
        std::thread::current().id()
    )
}

fn write_file(line: &str) {
    if let Ok(mut guard) = STATE.lock()
        && let Some(state) = guard.as_mut()
    {
        if let Some(file) = state.file.as_mut() {
            let _ = writeln!(file, "{line}");
            let _ = file.flush();
            return;
        }
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&state.path)
        {
            let _ = writeln!(file, "{line}");
            let _ = file.flush();
            state.file = Some(file);
            return;
        }
    }
    let path = log_path().unwrap_or_else(default_log_path);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
        let _ = file.flush();
    }
}

fn rotate_if_huge(path: &Path) {
    let Ok(meta) = fs::metadata(path) else {
        return;
    };
    if meta.len() < MAX_LOG_BYTES {
        return;
    }
    let prev = path.with_extension("log.prev");
    let _ = fs::remove_file(&prev);
    let _ = fs::rename(path, prev);
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info.location().map_or_else(
            || "unknown".into(),
            |loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()),
        );
        let payload = panic_text(info);
        log(format!("PANIC at {location}: {payload}"));
        if let Ok(trace) = std::env::var("RUST_BACKTRACE")
            && !trace.is_empty()
            && trace != "0"
        {
            log(format!(
                "backtrace:\n{}",
                std::backtrace::Backtrace::force_capture()
            ));
        }
        previous(info);
    }));
}

fn panic_text(info: &std::panic::PanicHookInfo<'_>) -> String {
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.clone()
    } else {
        "Box<dyn Any>".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_to_writes_and_survives_closed_style_log() {
        let dir = std::env::temp_dir().join(format!("lattice-studio-trace-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("studio.log");
        install_to(&path);
        log("hello-debug-sink");
        let text = fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("hello-debug-sink"),
            "log file must contain the message:\n{text}"
        );
        assert_eq!(log_path().as_deref(), Some(path.as_path()));
    }
}

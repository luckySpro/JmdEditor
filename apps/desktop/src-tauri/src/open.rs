//! Markdown files handed over by the system: the command line on Windows and
//! Linux, Apple events on macOS, and a second launch while JmdEditor is open.

use std::path::Path;
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};

/// Event the window listens to once it is ready.
pub const OPEN_FILE_EVENT: &str = "open-file";

/// A file that arrived before the window was ready to open it.
#[derive(Default)]
pub struct PendingOpen {
    file: Mutex<Option<String>>,
    ready: Mutex<bool>,
}

impl PendingOpen {
    /// Called once by the window at start: hands over the waiting file, and
    /// later files go straight to the window.
    pub fn take(&self) -> Option<String> {
        *self.ready.lock().unwrap() = true;
        self.file.lock().unwrap().take()
    }
}

/// The first argument that names an existing file, as an absolute path.
/// Relative paths are resolved against `cwd`.
pub fn file_arg<I, S>(args: I, cwd: &Path) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter().find_map(|a| {
        let a = a.as_ref();
        if a.starts_with('-') {
            return None;
        }
        let path = cwd.join(a);
        path.is_file()
            .then(|| std::fs::canonicalize(&path).ok())
            .flatten()
            .map(|p| p.to_string_lossy().into_owned())
    })
}

/// Opens `path` in the window, or keeps it until the window asks for it.
pub fn deliver(app: &AppHandle, path: String) {
    let pending = app.state::<PendingOpen>();
    let ready = *pending.ready.lock().unwrap();
    if !ready {
        *pending.file.lock().unwrap() = Some(path);
        return;
    }
    let _ = app.emit(OPEN_FILE_EVENT, path);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_first_existing_file_and_skips_flags() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "# a").unwrap();
        let found = file_arg(["--flag", "missing.md", "a.md"], dir.path()).unwrap();
        assert!(found.ends_with("a.md"));
        assert!(Path::new(&found).is_absolute());
        assert_eq!(file_arg(["--flag"], dir.path()), None);
    }

    #[test]
    fn a_file_waits_until_the_window_is_ready() {
        let pending = PendingOpen::default();
        *pending.file.lock().unwrap() = Some("/notes/a.md".into());
        assert_eq!(pending.take().as_deref(), Some("/notes/a.md"));
        assert_eq!(pending.take(), None);
        assert!(*pending.ready.lock().unwrap());
    }
}

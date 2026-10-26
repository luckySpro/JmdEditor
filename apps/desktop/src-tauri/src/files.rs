//! File access for the editor: reading, atomic saving and folder listing.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::AppError;

/// Notes larger than this are refused to keep the editor responsive.
pub const MAX_NOTE_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedFile {
    pub path: String,
    pub name: String,
    pub dir: String,
    pub content: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FolderEntry {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
}

pub fn absolute(path: &str) -> Result<PathBuf, AppError> {
    let p = PathBuf::from(path);
    if path.is_empty() || !p.is_absolute() {
        return Err(AppError::invalid("expected an absolute path"));
    }
    Ok(p)
}

fn display_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

pub fn read_note(path: &Path) -> Result<OpenedFile, AppError> {
    let meta = std::fs::metadata(path)?;
    if meta.is_dir() {
        return Err(AppError::invalid("this is a folder, not a file"));
    }
    if meta.len() > MAX_NOTE_BYTES {
        return Err(AppError::invalid("the file is larger than 20 MB"));
    }
    let bytes = std::fs::read(path)?;
    let content = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
    };
    // A UTF-8 byte order mark is dropped; saving writes plain UTF-8.
    let content = content.strip_prefix('\u{feff}').map(str::to_string).unwrap_or(content);
    Ok(OpenedFile {
        path: path.to_string_lossy().into_owned(),
        name: display_name(path),
        dir: path.parent().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default(),
        content,
    })
}

/// Writes through a temporary file in the same folder and renames it over the
/// target, so a crash never leaves a half-written note.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let dir = path.parent().ok_or_else(|| AppError::invalid("the path has no folder"))?;
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(".{}.jmd-tmp-{}", display_name(path), std::process::id()));
    let result = (|| {
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result.map_err(AppError::from)
}

fn is_note(p: &Path) -> bool {
    matches!(
        p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(),
        Some("md" | "markdown" | "mdown" | "mkd" | "txt")
    )
}

/// Markdown files and sub-folders of `dir`, folders first, hidden entries skipped.
pub fn list_notes(dir: &Path) -> Result<Vec<FolderEntry>, AppError> {
    let mut out: Vec<FolderEntry> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter_map(|e| {
            let path = e.path();
            let name = display_name(&path);
            if name.starts_with('.') || name == "node_modules" || name == "assets" {
                return None;
            }
            let is_dir = e.file_type().ok()?.is_dir();
            (is_dir || is_note(&path)).then(|| FolderEntry { path: path.to_string_lossy().into_owned(), name, is_dir })
        })
        .collect();
    out.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    out.truncate(2000);
    Ok(out)
}

/// Largest image accepted from paste or drop in the editor.
pub const MAX_ASSET_BYTES: usize = 20 * 1024 * 1024;

/// Keeps letters, digits, `-`, `_` and `.` so a pasted file name is safe on every OS.
pub fn safe_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '-' })
        .collect();
    let cleaned = cleaned.trim_matches(|c| c == '.' || c == '-').to_string();
    if cleaned.is_empty() { "image.png".to_string() } else { cleaned }
}

/// Saves an image under `<note_dir>/assets/` and returns the link to use in the note.
pub fn save_asset(note_dir: &Path, file_name: &str, bytes: &[u8]) -> Result<String, AppError> {
    if bytes.len() > MAX_ASSET_BYTES {
        return Err(AppError::invalid("the image is larger than 20 MB"));
    }
    let dir = note_dir.join("assets");
    let path = unique_path(&dir, &safe_file_name(file_name));
    write_atomic(&path, bytes)?;
    Ok(format!("assets/{}", display_name(&path)))
}

/// A file name that does not exist yet in `dir`: `name.md`, `name-2.md`, …
pub fn unique_path(dir: &Path, file_name: &str) -> PathBuf {
    let candidate = dir.join(file_name);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = match file_name.rsplit_once('.') {
        Some((s, e)) => (s.to_string(), format!(".{e}")),
        None => (file_name.to_string(), String::new()),
    };
    (2..10_000)
        .map(|n| dir.join(format!("{stem}-{n}{ext}")))
        .find(|p| !p.exists())
        .unwrap_or(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_atomically_and_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        write_atomic(&path, "# Hi\n".as_bytes()).unwrap();
        write_atomic(&path, "\u{feff}# Hello\n".as_bytes()).unwrap();
        let note = read_note(&path).unwrap();
        assert_eq!(note.content, "# Hello\n");
        assert_eq!(note.name, "note.md");
        let leftovers: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(leftovers.len(), 1, "no temporary files left behind");
    }

    #[test]
    fn lists_notes_and_folders() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b.md"), "").unwrap();
        std::fs::write(dir.path().join("A.markdown"), "").unwrap();
        std::fs::write(dir.path().join("pic.png"), "").unwrap();
        std::fs::write(dir.path().join(".hidden.md"), "").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::create_dir(dir.path().join("assets")).unwrap();
        let names: Vec<_> = list_notes(dir.path()).unwrap().into_iter().map(|e| e.name).collect();
        assert_eq!(names, ["sub", "A.markdown", "b.md"]);
    }

    #[test]
    fn unique_names() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(unique_path(dir.path(), "a.md"), dir.path().join("a.md"));
        std::fs::write(dir.path().join("a.md"), "").unwrap();
        assert_eq!(unique_path(dir.path(), "a.md"), dir.path().join("a-2.md"));
    }

    #[test]
    fn saves_assets_with_safe_unique_names() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(save_asset(dir.path(), "../evil name.png", b"x").unwrap(), "assets/evil-name.png");
        assert_eq!(save_asset(dir.path(), "../evil name.png", b"y").unwrap(), "assets/evil-name-2.png");
        assert!(dir.path().join("assets/evil-name.png").is_file());
        assert_eq!(safe_file_name("屏幕截图 1.png"), "屏幕截图-1.png");
    }

    #[test]
    fn rejects_relative_paths() {
        assert!(absolute("notes/a.md").is_err());
        assert!(absolute("").is_err());
    }
}

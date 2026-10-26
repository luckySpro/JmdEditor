//! User settings, edited in the Settings dialog and stored as JSON in the app
//! config folder. Writes are atomic and the previous file is kept as `.bak`.

use std::path::{Path, PathBuf};

use jmd_export::ExportOptions;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::files::write_atomic;

pub const MAX_RECENT: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// `system`, `en`, `zh-CN`, `zh-TW` or `ja`.
    pub language: String,
    /// `system`, `light` or `dark`.
    pub theme: String,
    pub accent: String,
    /// `rich`, `split`, `source` or `read`.
    pub editor_mode: String,
    pub editor_font_size: u8,
    pub workspace: Option<String>,
    pub recent_files: Vec<String>,
    pub recent_urls: Vec<String>,
    pub export: ExportOptions,
    pub export_dir: Option<String>,
    pub open_after_export: bool,
    pub import_save_images: bool,
    pub import_record_source: bool,
    pub autosave: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "system".into(),
            theme: "system".into(),
            accent: "#2F5BD3".into(),
            editor_mode: "split".into(),
            editor_font_size: 15,
            workspace: None,
            recent_files: Vec::new(),
            recent_urls: Vec::new(),
            export: ExportOptions::default(),
            export_dir: None,
            open_after_export: true,
            import_save_images: true,
            import_record_source: true,
            autosave: true,
        }
    }
}

impl Settings {
    /// Fixes values a hand-edited file may carry, so the UI never sees nonsense.
    pub fn normalized(mut self) -> Self {
        if !["system", "en", "zh-CN", "zh-TW", "ja"].contains(&self.language.as_str()) {
            self.language = "system".into();
        }
        if !["system", "light", "dark"].contains(&self.theme.as_str()) {
            self.theme = "system".into();
        }
        if !["rich", "split", "source", "read"].contains(&self.editor_mode.as_str()) {
            self.editor_mode = "split".into();
        }
        let hex = self.accent.strip_prefix('#').unwrap_or("");
        if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            self.accent = Settings::default().accent;
        }
        self.editor_font_size = self.editor_font_size.clamp(11, 24);
        self.recent_files.dedup();
        self.recent_files.truncate(MAX_RECENT);
        self.recent_urls.dedup();
        self.recent_urls.truncate(MAX_RECENT);
        self.export.toc_depth = self.export.toc_depth.clamp(1, 6);
        self
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedSettings {
    pub settings: Settings,
    /// Set when the stored file could not be read; the UI shows it and offers a reset.
    pub problem: Option<String>,
    pub path: String,
}

pub fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join("settings.json")
}

pub fn load(config_dir: &Path) -> LoadedSettings {
    let path = settings_path(config_dir);
    let display = path.to_string_lossy().into_owned();
    match std::fs::read_to_string(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            LoadedSettings { settings: Settings::default(), problem: None, path: display }
        }
        Err(e) => LoadedSettings { settings: Settings::default(), problem: Some(e.to_string()), path: display },
        Ok(text) => match serde_json::from_str::<Settings>(&text) {
            Ok(s) => LoadedSettings { settings: s.normalized(), problem: None, path: display },
            Err(e) => LoadedSettings {
                settings: Settings::default(),
                problem: Some(format!("settings.json could not be read ({e}); defaults are in use")),
                path: display,
            },
        },
    }
}

pub fn save(config_dir: &Path, settings: Settings) -> Result<Settings, AppError> {
    let settings = settings.normalized();
    let path = settings_path(config_dir);
    if path.exists() {
        let _ = std::fs::copy(&path, config_dir.join("settings.json.bak"));
    }
    write_atomic(&path, serde_json::to_string_pretty(&settings)?.as_bytes())?;
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_keeps_a_backup() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()).settings, Settings::default());
        let s = Settings { language: "zh-CN".into(), ..Default::default() };
        save(dir.path(), s.clone()).unwrap();
        save(dir.path(), Settings { theme: "dark".into(), ..s }).unwrap();
        let loaded = load(dir.path());
        assert_eq!(loaded.settings.language, "zh-CN");
        assert_eq!(loaded.settings.theme, "dark");
        assert!(dir.path().join("settings.json.bak").exists());
    }

    #[test]
    fn reports_broken_files_instead_of_failing_silently() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(settings_path(dir.path()), "{ not json").unwrap();
        let loaded = load(dir.path());
        assert!(loaded.problem.is_some());
        assert_eq!(loaded.settings, Settings::default());
    }

    #[test]
    fn normalizes_hand_edited_values() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            settings_path(dir.path()),
            r#"{"language":"klingon","accent":"red","editorFontSize":99,"export":{"tocDepth":9}}"#,
        )
        .unwrap();
        let s = load(dir.path()).settings;
        assert_eq!(s.language, "system");
        assert_eq!(s.accent, "#2F5BD3");
        assert_eq!(s.editor_font_size, 24);
        assert_eq!(s.export.toc_depth, 6);
    }
}

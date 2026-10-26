//! IPC commands called from the Vue UI. Every path is checked here; hiding a
//! button in the UI is not a security boundary.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use jmd_export::{ExportOptions, ImageSource, PdfEngine};
use jmd_import::{ImportOptions, Resolved};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::error::AppError;
use crate::files::{self, FolderEntry, OpenedFile};
use crate::settings::{self, LoadedSettings, Settings};
use crate::update::{self, UpdateStatus};

/// Shared state: the PDF engine is built in the background at start-up
/// because scanning system fonts takes a moment.
#[derive(Default)]
pub struct AppState {
    pub engine: Arc<OnceLock<PdfEngine>>,
    pub font_dirs: OnceLock<Vec<PathBuf>>,
}

impl AppState {
    pub fn engine(&self) -> &PdfEngine {
        self.engine.get_or_init(|| {
            PdfEngine::new(self.font_dirs.get().map(Vec::as_slice).unwrap_or_default())
        })
    }
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    app.path()
        .app_config_dir()
        .map_err(|e| AppError::new("settings", e.to_string()))
}

fn today() -> String {
    // Days since 1970-01-01 → civil date (Howard Hinnant's algorithm), no extra crate.
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[tauri::command]
pub fn app_info(app: AppHandle) -> serde_json::Value {
    serde_json::json!({
        "name": "JmdEditor",
        "version": app.package_info().version.to_string(),
        "author": "M&J Group",
        "email": "473276@qq.com",
        "repository": "https://github.com/luckySpro/JmdEditor",
        "today": today(),
        "platform": std::env::consts::OS,
    })
}

#[tauri::command]
pub fn initial_file(pending: State<'_, crate::open::PendingOpen>) -> Option<String> {
    pending.take()
}

#[tauri::command]
pub async fn open_file(path: String) -> Result<OpenedFile, AppError> {
    let path = files::absolute(&path)?;
    tauri::async_runtime::spawn_blocking(move || files::read_note(&path))
        .await
        .map_err(|e| AppError::new("io", e.to_string()))?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Saved {
    pub path: String,
    pub name: String,
    pub dir: String,
}

#[tauri::command]
pub async fn save_file(path: String, content: String) -> Result<Saved, AppError> {
    let p = files::absolute(&path)?;
    tauri::async_runtime::spawn_blocking(move || {
        files::write_atomic(&p, content.as_bytes())?;
        Ok(Saved {
            path: p.to_string_lossy().into_owned(),
            name: p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            dir: p
                .parent()
                .map(|d| d.to_string_lossy().into_owned())
                .unwrap_or_default(),
        })
    })
    .await
    .map_err(|e| AppError::new("io", e.to_string()))?
}

#[tauri::command]
pub async fn list_folder(path: String) -> Result<Vec<FolderEntry>, AppError> {
    let p = files::absolute(&path)?;
    tauri::async_runtime::spawn_blocking(move || files::list_notes(&p))
        .await
        .map_err(|e| AppError::new("io", e.to_string()))?
}

/// Stores a pasted or dropped image next to the note; returns the relative link.
#[tauri::command]
pub async fn save_asset(
    note_dir: String,
    file_name: String,
    data: Vec<u8>,
) -> Result<String, AppError> {
    let dir = files::absolute(&note_dir)?;
    tauri::async_runtime::spawn_blocking(move || files::save_asset(&dir, &file_name, &data))
        .await
        .map_err(|e| AppError::new("io", e.to_string()))?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    pub title: Option<String>,
    pub outline: Vec<jmd_core::OutlineEntry>,
    pub stats: jmd_core::Stats,
}

#[tauri::command]
pub fn analyze(markdown: String) -> Analysis {
    let doc = jmd_core::parse(&markdown);
    let (_, body) = jmd_core::split_front_matter(&markdown);
    Analysis {
        title: doc.title(),
        outline: jmd_core::outline(&doc),
        stats: jmd_core::stats(body),
    }
}

#[tauri::command]
pub fn resolve_url(url: String) -> Result<Resolved, AppError> {
    Ok(jmd_import::resolve(&url)?)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRequest {
    pub url: String,
    pub save_images: bool,
    pub record_source: bool,
    /// Folder the note will be saved in; images go to `<folder>/assets/<note>/`.
    pub folder: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub title: String,
    pub markdown: String,
    pub suggested_path: Option<String>,
    pub suggested_name: String,
    pub resolved: Resolved,
    pub images: jmd_import::ImageReport,
    pub bytes: usize,
}

#[tauri::command]
pub async fn import_url(request: ImportRequest) -> Result<ImportResult, AppError> {
    let resolved = jmd_import::resolve(&request.url)?;
    // Name the image folder after the note before downloading.
    let folder = request.folder.as_deref().map(files::absolute).transpose()?;
    let stem_hint = jmd_import::suggested_file_name("", &resolved);
    let stem = stem_hint.trim_end_matches(".md").to_string();
    let mut opts = ImportOptions {
        record_source: request.record_source,
        today: today(),
        ..Default::default()
    };
    if let (true, Some(dir)) = (request.save_images, &folder) {
        opts.image_dir = Some(dir.join("assets").join(&stem));
        opts.image_rel_dir = format!("assets/{stem}");
    }
    let note = jmd_import::import_url(&request.url, &opts).await?;
    let suggested_name = jmd_import::suggested_file_name(&note.title, &note.resolved);
    let suggested_path = folder.map(|d| {
        files::unique_path(&d, &suggested_name)
            .to_string_lossy()
            .into_owned()
    });
    Ok(ImportResult {
        title: note.title,
        markdown: note.markdown,
        suggested_path,
        suggested_name,
        resolved: note.resolved,
        images: note.images,
        bytes: note.bytes,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub markdown: String,
    /// Folder of the note, for relative image links.
    pub base_dir: Option<String>,
    pub options: ExportOptions,
    pub out_path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    pub bytes: usize,
    pub millis: u128,
}

#[tauri::command]
pub async fn export_document(
    request: ExportRequest,
    state: State<'_, AppState>,
) -> Result<ExportResult, AppError> {
    let out = files::absolute(&request.out_path)?;
    let base_dir = request
        .base_dir
        .as_deref()
        .map(files::absolute)
        .transpose()?;
    let engine_cell = state.engine.clone();
    let font_dirs = state.font_dirs.get().cloned().unwrap_or_default();
    // Web images are fetched now so the file contains them; offline they are skipped.
    let provided = jmd_import::prefetch_images(
        &request.markdown,
        &jmd_import::FetchLimits {
            timeout: std::time::Duration::from_secs(10),
            ..Default::default()
        },
    )
    .await;
    tauri::async_runtime::spawn_blocking(move || {
        let started = std::time::Instant::now();
        let engine = engine_cell.get_or_init(|| PdfEngine::new(&font_dirs));
        let images = ImageSource { base_dir, provided };
        let bytes =
            jmd_export::export_markdown(&request.markdown, &request.options, &images, engine)?;
        files::write_atomic(&out, &bytes)?;
        Ok(ExportResult {
            path: out.to_string_lossy().into_owned(),
            bytes: bytes.len(),
            millis: started.elapsed().as_millis(),
        })
    })
    .await
    .map_err(|e| AppError::new("export", e.to_string()))?
}

/// HTML of the note for the Read view and printing, using the export renderer.
#[tauri::command]
pub fn render_html(
    markdown: String,
    base_dir: Option<String>,
    options: ExportOptions,
) -> Result<String, AppError> {
    let base_dir = base_dir.as_deref().map(files::absolute).transpose()?;
    let images = ImageSource {
        base_dir,
        ..Default::default()
    };
    let doc = jmd_core::parse(&markdown);
    Ok(jmd_export::html::render(&doc, &options, &images))
}

#[tauri::command]
pub fn load_settings(app: AppHandle) -> Result<LoadedSettings, AppError> {
    Ok(settings::load(&config_dir(&app)?))
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<Settings, AppError> {
    settings::save(&config_dir(&app)?, settings)
}

#[tauri::command]
pub fn reset_settings(app: AppHandle) -> Result<Settings, AppError> {
    settings::save(&config_dir(&app)?, Settings::default())
}

/// Folder exports go to when none is chosen: Documents/JmdEditor exports.
#[tauri::command]
pub fn default_export_dir(app: AppHandle) -> Result<String, AppError> {
    let base = app
        .path()
        .document_dir()
        .or_else(|_| app.path().home_dir())
        .map_err(|e| AppError::new("io", e.to_string()))?;
    Ok(base
        .join("JmdEditor")
        .join("exports")
        .to_string_lossy()
        .into_owned())
}

#[tauri::command]
pub fn font_families(state: State<'_, AppState>) -> Vec<String> {
    state.engine().family_names()
}

pub fn pdf_font_dirs(app: &AppHandle) -> Vec<PathBuf> {
    app.path()
        .resource_dir()
        .map(|d| d.join("fonts"))
        .into_iter()
        .filter(|d: &PathBuf| Path::new(d).is_dir())
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn today_is_a_date() {
        let t = super::today();
        assert_eq!(t.len(), 10);
        assert_eq!(&t[4..5], "-");
        assert!(t.as_str() >= "2026-01-01");
    }
}

fn updater_pubkey(app: &AppHandle) -> Option<String> {
    app.config()
        .plugins
        .0
        .get("updater")?
        .get("pubkey")?
        .as_str()
        .map(str::to_string)
}

/// Checks the signed release policy. Never fails: a check that cannot reach
/// the server falls back to the last verified policy, or reports `unknown`.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> UpdateStatus {
    let current = app.package_info().version.to_string();
    let Ok(dir) = config_dir(&app) else {
        return update::decide(&current, None, Some("no config folder"));
    };
    let cached = update::load_cache(&dir);
    let fresh = match updater_pubkey(&app) {
        Some(key) => update::fetch_policy(&key, &update::bases()).await,
        None => Err("this build has no update key".to_string()),
    };
    let (fresh, failure) = match fresh {
        Ok(p) => (Some(p), None),
        Err(e) => (None, Some(e)),
    };
    let known = update::merge(cached.clone(), fresh);
    if let Some(p) = known.as_ref().filter(|p| Some(*p) != cached.as_ref()) {
        let _ = update::save_cache(&dir, p);
    }
    update::decide(&current, known.as_ref(), failure.as_deref())
}

/// Downloads, verifies and installs the newest release, then restarts. The UI
/// saves the open note before calling this.
#[tauri::command]
pub async fn install_update(
    app: AppHandle,
    on_progress: tauri::ipc::Channel<serde_json::Value>,
) -> Result<(), AppError> {
    #[cfg(desktop)]
    {
        use tauri_plugin_updater::UpdaterExt;
        let fail = |e: tauri_plugin_updater::Error| AppError::new("update_install", e.to_string());
        let endpoints = update::bases()
            .iter()
            .filter_map(|b| format!("{b}/latest.json").parse().ok())
            .collect();
        let updater = app
            .updater_builder()
            .endpoints(endpoints)
            .map_err(fail)?
            .build()
            .map_err(fail)?;
        let Some(pending) = updater.check().await.map_err(fail)? else {
            return Err(AppError::new("update_none", "no newer version was found"));
        };
        let mut received: u64 = 0;
        pending
            .download_and_install(
                |chunk, total| {
                    received += chunk as u64;
                    let _ = on_progress
                        .send(serde_json::json!({ "received": received, "total": total }));
                },
                || {
                    let _ = on_progress.send(serde_json::json!({ "installing": true }));
                },
            )
            .await
            .map_err(fail)?;
        app.restart();
    }
    #[cfg(mobile)]
    {
        let _ = (app, on_progress);
        Err(AppError::new(
            "update_unsupported",
            "install updates from the app store",
        ))
    }
}

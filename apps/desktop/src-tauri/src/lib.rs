//! JmdEditor app shell: wires the Rust crates to the Vue UI through Tauri.

mod commands;
mod error;
mod files;
mod settings;

use tauri::Manager;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // A Markdown file passed on the command line (file association) opens at start.
    let initial = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with('-') && std::path::Path::new(a).is_file())
        .and_then(|a| std::fs::canonicalize(a).ok())
        .map(|p| p.to_string_lossy().into_owned());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .setup(move |app| {
            let state = app.state::<AppState>();
            let _ = state.font_dirs.set(commands::pdf_font_dirs(app.handle()));
            let _ = state.initial_file.set(initial.clone());
            // Warm the PDF engine so the first export is quick.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let _ = handle.state::<AppState>().engine();
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::initial_file,
            commands::open_file,
            commands::save_file,
            commands::list_folder,
            commands::save_asset,
            commands::analyze,
            commands::resolve_url,
            commands::import_url,
            commands::export_document,
            commands::render_html,
            commands::load_settings,
            commands::save_settings,
            commands::reset_settings,
            commands::default_export_dir,
            commands::font_families,
        ])
        .run(tauri::generate_context!())
        .expect("error while running JmdEditor");
}

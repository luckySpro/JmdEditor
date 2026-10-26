//! JmdEditor app shell: wires the Rust crates to the Vue UI through Tauri.

mod commands;
mod error;
mod files;
mod open;
mod settings;
mod update;

use tauri::Manager;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // A Markdown file passed on the command line (file association) opens at start.
    let cwd = std::env::current_dir().unwrap_or_default();
    let initial = open::file_arg(std::env::args().skip(1), &cwd);

    let builder = tauri::Builder::default();
    // A second launch (double-clicking another file) opens it in this window.
    #[cfg(desktop)]
    let builder =
        builder.plugin(tauri_plugin_single_instance::init(
            |app, args, cwd| match open::file_arg(args.iter().skip(1), std::path::Path::new(&cwd)) {
                Some(path) => open::deliver(app, path),
                None => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                    }
                }
            },
        ));

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .manage(open::PendingOpen::default())
        .setup(move |app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            let state = app.state::<AppState>();
            let _ = state.font_dirs.set(commands::pdf_font_dirs(app.handle()));
            if let Some(path) = initial.clone() {
                open::deliver(app.handle(), path);
            }
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
            commands::check_update,
            commands::install_update,
        ])
        .build(tauri::generate_context!())
        .expect("error while building JmdEditor")
        .run(|_app, _event| {
            // macOS hands over double-clicked files as an event, not as arguments.
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            if let tauri::RunEvent::Opened { urls } = _event {
                for url in urls {
                    if let Ok(path) = url.to_file_path() {
                        open::deliver(_app, path.to_string_lossy().into_owned());
                    }
                }
            }
        });
}

// Tauri entry point. Each module owns one concept from CONTEXT.md:
// cv (content + validation), render (Typst PDF + preview/export identity),
// library (local project files + recovery), trash, backup, files (atomic writes).
mod backup;
mod cv;
mod files;
mod library;
mod render;
mod trash;
#[cfg(test)]
mod tests;

use std::fs;

use tauri::Manager;

use library::instance_lock;
use render::RenderCache;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("no application data directory: {e}"))?;
            fs::create_dir_all(&dir)
                .map_err(|e| format!("could not open the CV library: {e}"))?;
            match instance_lock(&dir) {
                Ok(lock) => {
                    app.manage(lock);
                }
                Err(message) => {
                    eprintln!("{message}");
                    std::process::exit(1);
                }
            }
            Ok(())
        })
        .manage(RenderCache::default())
        .invoke_handler(tauri::generate_handler![
            render::render_pdf,
            render::export_pdf,
            library::list_projects,
            library::create_project,
            library::duplicate_project,
            library::open_project,
            library::save_project,
            backup::export_backup,
            backup::import_backup,
            trash::list_trash,
            trash::delete_project,
            trash::restore_project,
            trash::delete_project_permanently
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}

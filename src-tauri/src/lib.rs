mod document;

use document::NotebookDocument;
use std::path::PathBuf;

#[tauri::command]
async fn open_notebook(path: PathBuf) -> Result<NotebookDocument, String> {
    tauri::async_runtime::spawn_blocking(move || document::open(&path))
        .await
        .map_err(|error| format!("notebook task failed: {error}"))?
}

#[tauri::command]
async fn save_notebook(path: PathBuf, notebook: NotebookDocument) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || document::save(&path, &notebook))
        .await
        .map_err(|error| format!("notebook task failed: {error}"))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![open_notebook, save_notebook])
        .run(tauri::generate_context!())
        .expect("error while running Taipan");
}

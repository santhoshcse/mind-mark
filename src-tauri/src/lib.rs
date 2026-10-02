use mind_mark_core::{
    AppProfile, Bookmark, BookmarkFolder, ChromeProfile, CreateBookmarkInput, Database, SyncReport,
    UpdateBookmarkInput,
};
use tauri::{Manager, State};

#[tauri::command]
fn list_chrome_profiles(
    handle: tauri::AppHandle,
) -> Result<Vec<ChromeProfile>, String> {
    mind_mark_core::discover_chrome_profiles(&handle).map_err(|error| error.to_string())
}

#[tauri::command]
fn sync_chrome_profile(
    profile_id: String,
    database: State<'_, Database>,
    handle: tauri::AppHandle,
) -> Result<SyncReport, String> {
    mind_mark_core::sync_chrome_profile(&database, &profile_id, &handle).map_err(|error| error.to_string())
}

#[tauri::command]
fn list_app_profiles(database: State<'_, Database>) -> Result<Vec<AppProfile>, String> {
    database
        .list_app_profiles()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn create_app_profile(name: String, database: State<'_, Database>) -> Result<AppProfile, String> {
    database
        .create_app_profile(&name)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn list_folders(
    app_profile_id: String,
    database: State<'_, Database>,
) -> Result<Vec<BookmarkFolder>, String> {
    database
        .list_folders(&app_profile_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn list_bookmarks(
    app_profile_id: String,
    folder_id: Option<String>,
    database: State<'_, Database>,
) -> Result<Vec<Bookmark>, String> {
    database
        .list_bookmarks(&app_profile_id, folder_id.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn create_bookmark(
    input: CreateBookmarkInput,
    database: State<'_, Database>,
) -> Result<Bookmark, String> {
    database
        .create_bookmark(&input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn update_bookmark(
    input: UpdateBookmarkInput,
    database: State<'_, Database>,
) -> Result<Bookmark, String> {
    database
        .update_bookmark(&input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn archive_bookmark(id: String, database: State<'_, Database>) -> Result<(), String> {
    database
        .archive_bookmark(&id)
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let database = mind_mark_core::Database::open(app_data_dir.join("mind-mark.sqlite3"))
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            app.manage(database);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_chrome_profiles,
            sync_chrome_profile,
            list_app_profiles,
            create_app_profile,
            list_folders,
            list_bookmarks,
            create_bookmark,
            update_bookmark,
            archive_bookmark
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

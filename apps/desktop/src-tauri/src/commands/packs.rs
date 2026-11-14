//! Exporting and importing theme packs (`.swpack`, see `crate::packs`).

use std::path::PathBuf;

use sw_core::Id;
use tauri::{AppHandle, Manager};

use crate::commands;
use crate::error::AppResult;
use crate::media;
use crate::packs::{self, PackImported};
use crate::state::AppState;

/// Saves the chosen themes and lower thirds, with the pictures and videos
/// behind the themes, as a `.swpack` file at `path`. Runs in the background
/// (it may copy videos).
#[tauri::command]
pub async fn pack_export(
    app: AppHandle,
    path: String,
    theme_ids: Vec<Id>,
    lower_third_ids: Vec<Id>,
) -> AppResult<()> {
    let export = move || -> AppResult<()> {
        let state = app.state::<AppState>();
        let mut target = PathBuf::from(path);
        if target
            .extension()
            .is_none_or(|ext| !ext.eq_ignore_ascii_case("swpack"))
        {
            target.as_mut_os_string().push(".swpack");
        }
        let contents = packs::gather(&state.database(), &theme_ids, &lower_third_ids)?;
        packs::write(&target, &contents, &state.media_folders)
    };
    tauri::async_runtime::spawn_blocking(export).await?
}

/// Imports a `.swpack` file: adds the themes, lower thirds and media files
/// this computer lacks, and says what it added and which fonts are
/// missing. Runs in the background (it may copy videos).
#[tauri::command]
pub async fn pack_import(app: AppHandle, path: String) -> AppResult<PackImported> {
    let fonts_here = commands::themes::fonts_list().await;
    let import = move || -> AppResult<PackImported> {
        let state = app.state::<AppState>();
        let imported = packs::read(
            &PathBuf::from(path),
            &state.database(),
            &state.media_folders,
            &fonts_here,
        )?;
        if imported.themes_added > 0 {
            commands::themes::sync_themes(&app, &state)?;
        }
        // Pictures it brought may have their own scaling.
        commands::media::sync_file_fits(&app, &state)?;
        media::announce(&app);
        Ok(imported)
    };
    tauri::async_runtime::spawn_blocking(import).await?
}

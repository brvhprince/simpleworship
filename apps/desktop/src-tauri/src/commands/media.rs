//! The media library: importing, listing, thumbnails, and showing media.

use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use sw_core::{Command, Id, ItemContent, MediaItem, MediaKind, MediaScaling, ScheduleItem};
use tauri::{AppHandle, State};
use ts_rs::TS;

use crate::commands::engine::dispatch;
use crate::error::{AppError, AppResult};
use crate::media;
use crate::state::AppState;

/// Where media files and thumbnails are, so the UI can build their URLs.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct MediaFolderPaths {
    /// The folder with the media files.
    pub media: String,
    /// The folder with the thumbnails.
    pub thumbnails: String,
}

/// Where media files and thumbnails are.
#[tauri::command]
pub fn media_folders(state: State<'_, AppState>) -> MediaFolderPaths {
    MediaFolderPaths {
        media: state.media_folders.media.display().to_string(),
        thumbnails: state.media_folders.thumbnails.display().to_string(),
    }
}

/// Every media item, newest first.
#[tauri::command]
pub fn media_list(state: State<'_, AppState>) -> AppResult<Vec<MediaItem>> {
    Ok(state.database().media().list()?)
}

/// Copies files into the media library. Runs in the background, reporting
/// progress; resolves to the new items. Files that are not images or
/// videos are skipped.
#[tauri::command]
pub async fn media_import(app: AppHandle, paths: Vec<String>) -> AppResult<Vec<MediaItem>> {
    let import = move || -> AppResult<Vec<MediaItem>> {
        let mut imported = Vec::new();
        for path in paths {
            let path = PathBuf::from(path);
            if media::kind_of(&path).is_some() {
                imported.push(media::import_file(&app, &path)?);
            }
        }
        Ok(imported)
    };
    tauri::async_runtime::spawn_blocking(import).await?
}

/// Records details the webview measured: a video's size and length.
#[tauri::command]
pub fn media_update_details(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Id,
    width: Option<u32>,
    height: Option<u32>,
    duration_ms: Option<u64>,
) -> AppResult<()> {
    let mut item = state
        .database()
        .media()
        .get(id)?
        .ok_or(AppError::MediaNotFound)?;
    item.width = width.or(item.width);
    item.height = height.or(item.height);
    item.duration_ms = duration_ms.or(item.duration_ms);
    state.database().media().save(&item)?;
    media::announce(&app);
    Ok(())
}

/// Stores a thumbnail (JPEG bytes drawn by the webview).
#[tauri::command]
pub fn media_save_thumbnail(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Id,
    jpeg: Vec<u8>,
) -> AppResult<()> {
    let path = media::thumbnail_path(&state.media_folders, id);
    crate::files::write_atomically(&path, &jpeg)?;
    media::announce(&app);
    Ok(())
}

/// Renames a media item.
#[tauri::command]
pub fn media_rename(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Id,
    name: String,
) -> AppResult<()> {
    let mut item = state
        .database()
        .media()
        .get(id)?
        .ok_or(AppError::MediaNotFound)?;
    item.name = name.trim().to_owned();
    state.database().media().save(&item)?;
    media::announce(&app);
    Ok(())
}

/// Sets how a picture or video fills the screen wherever it is used (the
/// Media tab's Scaling), and updates what the screens show straight away.
#[tauri::command]
pub fn media_set_scaling(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Id,
    scaling: MediaScaling,
) -> AppResult<()> {
    if !state.database().media().set_scaling(id, scaling)? {
        return Err(AppError::MediaNotFound);
    }
    media::announce(&app);
    sync_file_fits(&app, &state)
}

/// Gives the engine the media files' own fits (see `Command::SetFileFits`).
/// Called at startup and whenever a file's scaling may have changed.
///
/// Reading the fits and handing them over happen together, one call at a
/// time: otherwise two calls at once (several files changed together) could
/// hand over out of order, and an older list replace a newer one. The
/// database lock itself is not held while dispatching, as sending the
/// engine's events can need the database.
pub fn sync_file_fits(app: &AppHandle, state: &AppState) -> AppResult<()> {
    static SYNCING: Mutex<()> = Mutex::new(());
    let _one_at_a_time = SYNCING.lock().unwrap_or_else(PoisonError::into_inner);
    let fits = state.database().media().file_fits()?;
    dispatch(app, state, Command::SetFileFits { fits })
}

/// Deletes a media item and its file.
#[tauri::command]
pub fn media_delete(app: AppHandle, state: State<'_, AppState>, id: Id) -> AppResult<()> {
    if let Some(item) = state.database().media().delete(id)? {
        media::remove_files(&state.media_folders, &item);
    }
    media::announce(&app);
    Ok(())
}

/// Shows an image or video on its own in the preview pane.
#[tauri::command]
pub fn media_show_in_preview(app: AppHandle, state: State<'_, AppState>, id: Id) -> AppResult<()> {
    let media = state
        .database()
        .media()
        .get(id)?
        .ok_or(AppError::MediaNotFound)?;
    dispatch(
        &app,
        &state,
        Command::PreviewMedia {
            media: Box::new(media),
        },
    )
}

/// Adds an image or video to the schedule at `index` (or at the end). A
/// sound file becomes background music (a one-track playlist).
#[tauri::command]
pub fn schedule_add_media(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Id,
    index: Option<usize>,
) -> AppResult<()> {
    let media = state
        .database()
        .media()
        .get(id)?
        .ok_or(AppError::MediaNotFound)?;
    let content = if media.kind == MediaKind::Audio {
        ItemContent::Playlist {
            name: media.name.clone(),
            tracks: vec![media],
        }
    } else {
        ItemContent::Media { media }
    };
    let item = ScheduleItem::new(content);
    dispatch(
        &app,
        &state,
        Command::AddScheduleItem {
            item: Box::new(item),
            index,
        },
    )
}

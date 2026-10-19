//! The song library.

use sw_core::themes::ContentKind;
use sw_core::{Command, Id, ItemLook, Presentation, SlideOptions, Song, SongSummary};
use tauri::{AppHandle, State};

use crate::commands::engine::dispatch;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Searches titles, authors and lyrics. An empty query lists every song.
#[tauri::command]
pub fn songs_search(state: State<'_, AppState>, query: String) -> AppResult<Vec<SongSummary>> {
    Ok(state.database().songs().search(&query)?)
}

/// Loads one song for editing.
#[tauri::command]
pub fn song_get(state: State<'_, AppState>, id: Id) -> AppResult<Song> {
    state
        .database()
        .songs()
        .get(id)?
        .ok_or(AppError::SongNotFound)
}

/// Creates or updates a song and returns the saved copy. Copies of the song
/// in the schedule, preview and live panes are updated too, so a typo fixed
/// mid-service is fixed on screen straight away.
#[tauri::command]
pub fn song_save(app: AppHandle, state: State<'_, AppState>, song: Song) -> AppResult<Song> {
    let saved = state.database().songs().save(&song)?;
    dispatch(
        &app,
        &state,
        Command::RefreshSong {
            song: Box::new(saved.clone()),
        },
    )?;
    Ok(saved)
}

/// Deletes a song. Returns `false` if it was already gone.
#[tauri::command]
pub fn song_delete(state: State<'_, AppState>, id: Id) -> AppResult<bool> {
    Ok(state.database().songs().delete(id)?)
}

/// Builds the slides for a (possibly unsaved) song, so the editor can show
/// how the lyrics will be split while the operator types.
#[tauri::command]
pub fn song_presentation(state: State<'_, AppState>, song: Song) -> Presentation {
    let engine = state.engine();
    let look = engine
        .themes()
        .resolve(ContentKind::Song, song.theme_id, &ItemLook::default());
    Presentation::from_song(&song, look, SlideOptions::default())
}

/// Loads a song from the library into the preview pane.
#[tauri::command]
pub fn song_show_in_preview(app: AppHandle, state: State<'_, AppState>, id: Id) -> AppResult<()> {
    let song = state
        .database()
        .songs()
        .get(id)?
        .ok_or(AppError::SongNotFound)?;
    dispatch(
        &app,
        &state,
        Command::PreviewSong {
            song: Box::new(song),
        },
    )
}

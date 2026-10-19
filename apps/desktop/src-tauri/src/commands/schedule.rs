//! The schedule: adding items, and opening and saving `.sws` files.
//!
//! Changes to the schedule itself (moving, removing, notes) are plain engine
//! commands sent with `engine_dispatch`. The commands here are the ones that
//! need something the engine does not have: the library, or the disk.

use std::fs;
use std::path::PathBuf;

use sw_core::{Command, Id, ItemContent, Schedule, ScheduleItem};
use tauri::{AppHandle, State};

use crate::autosave::AutosaveInfo;
use crate::commands::engine::dispatch;
use crate::error::{AppError, AppResult};
use crate::files::write_atomically;
use crate::menu;
use crate::recent::RecentSchedule;
use crate::schedule_file::{self, ScheduleFileStatus, with_schedule_extension};
use crate::state::AppState;

// ── Adding items ─────────────────────────────────────────────────────────

/// Adds a song from the library at `index` (or at the end).
#[tauri::command]
pub fn schedule_add_song(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Id,
    index: Option<usize>,
) -> AppResult<()> {
    let song = state
        .database()
        .songs()
        .get(id)?
        .ok_or(AppError::SongNotFound)?;
    add_item(&app, &state, ItemContent::Song { song }, index)
}

/// Adds free text at `index` (or at the end).
#[tauri::command]
pub fn schedule_add_text(
    app: AppHandle,
    state: State<'_, AppState>,
    title: String,
    text: String,
    index: Option<usize>,
) -> AppResult<()> {
    add_item(&app, &state, ItemContent::Text { title, text }, index)
}

fn add_item(
    app: &AppHandle,
    state: &AppState,
    content: ItemContent,
    index: Option<usize>,
) -> AppResult<()> {
    let item = Box::new(ScheduleItem::new(content));
    dispatch(app, state, Command::AddScheduleItem { item, index })
}

// ── Songs that are not in this computer's library ────────────────────────

/// The ids of songs in the schedule that are not in the library, e.g.
/// because the schedule was made on another computer.
#[tauri::command]
pub fn schedule_missing_songs(state: State<'_, AppState>) -> AppResult<Vec<Id>> {
    let schedule = state.engine().schedule().clone();
    let database = state.database();

    let mut missing = Vec::new();
    for song in songs_in(&schedule) {
        if database.songs().get(song.id)?.is_none() && !missing.contains(&song.id) {
            missing.push(song.id);
        }
    }
    Ok(missing)
}

/// Adds the schedule's songs that are missing from the library. Returns how
/// many were added.
#[tauri::command]
pub fn schedule_add_missing_songs(state: State<'_, AppState>) -> AppResult<usize> {
    let schedule = state.engine().schedule().clone();
    let database = state.database();

    let mut added = 0;
    for song in songs_in(&schedule) {
        if database.songs().get(song.id)?.is_none() {
            database.songs().save(song)?;
            added += 1;
        }
    }
    Ok(added)
}

fn songs_in(schedule: &Schedule) -> impl Iterator<Item = &sw_core::Song> {
    schedule
        .items
        .iter()
        .filter_map(|item| match &item.content {
            ItemContent::Song { song } => Some(song),
            ItemContent::Text { .. } => None,
        })
}

/// Swaps each song copy in `schedule` for the library's version, when the
/// library has the song. The library copy wins because it may contain
/// corrections made after the schedule was saved.
fn prefer_library_songs(state: &AppState, schedule: &mut Schedule) -> AppResult<()> {
    let database = state.database();
    for item in &mut schedule.items {
        if let ItemContent::Song { song } = &mut item.content
            && let Some(library_song) = database.songs().get(song.id)?
        {
            *song = library_song;
        }
    }
    Ok(())
}

// ── Files ────────────────────────────────────────────────────────────────

/// The current file name and unsaved-changes flag.
#[tauri::command]
pub fn schedule_file_status(state: State<'_, AppState>) -> ScheduleFileStatus {
    state.schedule_file().status()
}

/// Starts a new, empty schedule. The UI asks about unsaved changes first.
#[tauri::command]
pub fn schedule_new(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    replace_schedule(&app, &state, Schedule::default(), None, false)
}

/// Opens a `.sws` file.
#[tauri::command]
pub fn schedule_open(app: AppHandle, state: State<'_, AppState>, path: String) -> AppResult<()> {
    let path = PathBuf::from(path);
    let mut schedule = sw_files::read_schedule(&fs::read(&path)?)?;
    prefer_library_songs(&state, &mut schedule)?;

    replace_schedule(&app, &state, schedule, Some(path.clone()), false)?;
    remember_recent(&app, &state, &path);
    Ok(())
}

/// Saves the schedule. With a `path` this is "Save As"; without one it
/// saves to the current file.
#[tauri::command]
pub fn schedule_save(
    app: AppHandle,
    state: State<'_, AppState>,
    path: Option<String>,
) -> AppResult<()> {
    let engine = state.engine();
    let mut file = state.schedule_file();

    let target = match path {
        Some(path) => with_schedule_extension(PathBuf::from(path)),
        None => file.path.clone().ok_or(AppError::NoSchedulePath)?,
    };
    write_atomically(&target, &sw_files::write_schedule(engine.schedule())?)?;

    file.path = Some(target.clone());
    file.dirty = false;
    state.autosave.discard();
    schedule_file::announce(&app, &file.status());

    drop((file, engine));
    remember_recent(&app, &state, &target);
    Ok(())
}

/// The recently used schedule files, most recent first.
#[tauri::command]
pub fn schedule_recent(state: State<'_, AppState>) -> Vec<RecentSchedule> {
    state.recent().list()
}

// ── Crash recovery ───────────────────────────────────────────────────────

/// The schedule autosaved by a run that did not close normally, if any.
#[tauri::command]
pub fn autosave_leftover(state: State<'_, AppState>) -> Option<AutosaveInfo> {
    state.autosave.leftover()
}

/// Brings back the autosaved schedule, still marked as unsaved.
#[tauri::command]
pub fn autosave_restore(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    let (mut schedule, original_path) = state.autosave.load()?;
    prefer_library_songs(&state, &mut schedule)?;
    replace_schedule(&app, &state, schedule, original_path, true)
}

/// Throws the autosaved schedule away, e.g. when the operator quits and
/// chooses "Don't Save". Waits until it is really gone, because the app is
/// usually about to exit.
#[tauri::command]
pub fn autosave_discard(state: State<'_, AppState>) {
    state.autosave.discard_and_wait();
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Loads `schedule` into the engine and records which file it belongs to.
/// `dirty` says whether it starts out with unsaved changes (only true for
/// a restored autosave).
fn replace_schedule(
    app: &AppHandle,
    state: &AppState,
    schedule: Schedule,
    path: Option<PathBuf>,
    dirty: bool,
) -> AppResult<()> {
    // Set the path first, so the autosave that dispatching triggers already
    // knows which file the schedule belongs to.
    state.schedule_file().path = path;

    let schedule = Box::new(schedule);
    dispatch(app, state, Command::ReplaceSchedule { schedule })?;

    // Dispatching marked the schedule as changed and autosaved it. A freshly
    // opened or new schedule has no unsaved changes, so undo both.
    let mut file = state.schedule_file();
    file.dirty = dirty;
    if !dirty {
        state.autosave.discard();
    }
    schedule_file::announce(app, &file.status());
    Ok(())
}

/// Puts `path` at the top of the recent list and refreshes the menu.
fn remember_recent(app: &AppHandle, state: &AppState, path: &std::path::Path) {
    let list = {
        let mut recent = state.recent();
        recent.add(path);
        recent.list()
    };
    if let Err(error) = menu::update_recent(app, &list) {
        eprintln!("could not update the Open Recent menu: {error}");
    }
}

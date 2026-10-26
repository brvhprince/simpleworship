//! The schedule: adding items, and opening and saving `.sws` files.
//!
//! Changes to the schedule itself (moving, removing, notes) are plain engine
//! commands sent with `engine_dispatch`. The commands here are the ones that
//! need something the engine does not have: the library, or the disk.

use std::path::PathBuf;

use sw_core::countdown::CountdownSlide;
use sw_core::output::RenderFrame;
use sw_core::{CameraRef, Command, Id, ItemContent, Schedule, ScheduleItem};
use tauri::{AppHandle, Manager, State};

use crate::autosave::AutosaveInfo;
use crate::commands::{self, engine::dispatch};
use crate::error::{AppError, AppResult};
use crate::menu;
use crate::recent::RecentSchedule;
use crate::schedule_file::{self, ScheduleFileStatus, with_schedule_extension};
use crate::state::AppState;
use crate::{media, schedule_archive};

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

/// Adds a camera's live picture at `index` (or at the end).
#[tauri::command]
pub fn schedule_add_camera(
    app: AppHandle,
    state: State<'_, AppState>,
    camera: CameraRef,
    index: Option<usize>,
) -> AppResult<()> {
    add_item(&app, &state, ItemContent::Camera { camera }, index)
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

/// Adds a countdown at `index` (or at the end).
#[tauri::command]
pub fn schedule_add_countdown(
    app: AppHandle,
    state: State<'_, AppState>,
    title: String,
    countdown: CountdownSlide,
    index: Option<usize>,
) -> AppResult<()> {
    add_item(
        &app,
        &state,
        ItemContent::Countdown { title, countdown },
        index,
    )
}

/// Adds background music: the audio files with `track_ids`, in that order.
#[tauri::command]
pub fn schedule_add_playlist(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    track_ids: Vec<Id>,
    index: Option<usize>,
) -> AppResult<()> {
    let tracks = {
        let database = state.database();
        track_ids
            .into_iter()
            .map(|id| database.media().get(id)?.ok_or(AppError::MediaNotFound))
            .collect::<AppResult<Vec<_>>>()?
    };
    add_item(&app, &state, ItemContent::Playlist { name, tracks }, index)
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
            _ => None,
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
/// The first slide of a schedule item, as the screens would draw it, for
/// the schedule's grid of thumbnails. `None` for background music, which
/// has nothing to show.
#[tauri::command]
pub fn schedule_item_thumbnail(state: State<'_, AppState>, id: Id) -> Option<RenderFrame> {
    state.engine().item_thumbnail(id)
}

#[tauri::command]
pub fn schedule_file_status(state: State<'_, AppState>) -> ScheduleFileStatus {
    state.schedule_file().status()
}

/// Starts a new, empty schedule. The UI asks about unsaved changes first.
#[tauri::command]
pub fn schedule_new(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    replace_schedule(&app, &state, Schedule::default(), None, false)
}

/// Opens a `.sws` file. Themes and media it carries that this computer's
/// library lacks are added. Runs in the background (it may copy videos).
#[tauri::command]
pub async fn schedule_open(app: AppHandle, path: String) -> AppResult<()> {
    let open = move || -> AppResult<()> {
        let state = app.state::<AppState>();
        let path = PathBuf::from(path);

        let (mut schedule, themes_added) = {
            let database = state.database();
            schedule_archive::read(&path, &database, &state.media_folders)?
        };
        if themes_added {
            commands::themes::sync_themes(&app, &state)?;
        }
        // Files it brought may have their own scaling.
        commands::media::sync_file_fits(&app, &state)?;
        media::announce(&app);
        prefer_library_songs(&state, &mut schedule)?;

        replace_schedule(&app, &state, schedule, Some(path.clone()), false)?;
        remember_recent(&app, &state, &path);
        Ok(())
    };
    tauri::async_runtime::spawn_blocking(open).await?
}

/// Saves the schedule. With a `path` this is "Save As"; without one it
/// saves to the current file. With `include_media` (the default) the
/// images and videos it uses are embedded, so it works on any computer.
/// Runs in the background (it may copy videos).
#[tauri::command]
pub async fn schedule_save(
    app: AppHandle,
    path: Option<String>,
    include_media: Option<bool>,
) -> AppResult<()> {
    let save = move || -> AppResult<()> {
        let state = app.state::<AppState>();

        // Take a snapshot quickly, then write without holding any lock.
        let (contents, target) = {
            let engine = state.engine();
            let file = state.schedule_file();
            let target = match path {
                Some(path) => with_schedule_extension(PathBuf::from(path)),
                None => file.path.clone().ok_or(AppError::NoSchedulePath)?,
            };
            let database = state.database();
            let contents = schedule_archive::gather(engine.schedule(), engine.themes(), &database)?;
            (contents, target)
        };
        schedule_archive::write(
            &target,
            &contents,
            &state.media_folders,
            include_media.unwrap_or(true),
        )?;

        // Only call it "saved" if nothing changed while the file was written.
        let engine = state.engine();
        let mut file = state.schedule_file();
        file.path = Some(target.clone());
        if *engine.schedule() == contents.schedule {
            file.dirty = false;
            state.autosave.discard();
        }
        schedule_file::announce(&app, &file.status());
        drop((file, engine));

        remember_recent(&app, &state, &target);
        Ok(())
    };
    tauri::async_runtime::spawn_blocking(save).await?
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
        log::warn!("could not update the Open Recent menu: {error}");
    }
}

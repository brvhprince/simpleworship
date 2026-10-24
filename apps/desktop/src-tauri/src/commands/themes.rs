//! The theme library, default themes, fonts and the logo.

use std::collections::BTreeSet;

use sw_core::{Command, Id, MediaRef, ThemeDefaults, ThemeRecord};
use tauri::{AppHandle, State};

use crate::commands::engine::dispatch;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Settings key of the logo image.
const LOGO_KEY: &str = "logo";

/// Fonts that ship with the app (see `packages/renderer/fonts`), so themes
/// look the same on every computer. Listed first in the font picker.
const BUNDLED_FONTS: &[&str] = &[
    "Bebas Neue",
    "Lato",
    "Merriweather",
    "Montserrat",
    "Open Sans",
];

/// Creates or updates a theme.
#[tauri::command]
pub fn theme_save(app: AppHandle, state: State<'_, AppState>, theme: ThemeRecord) -> AppResult<()> {
    state.database().themes().save(&theme)?;
    sync_themes(&app, &state)
}

/// Deletes a theme. Content that used it falls back to the default theme.
#[tauri::command]
pub fn theme_delete(app: AppHandle, state: State<'_, AppState>, id: Id) -> AppResult<()> {
    if !state.database().themes().delete(id)? {
        return Err(AppError::ThemeNotFound);
    }
    sync_themes(&app, &state)
}

/// Chooses the default theme for songs, scripture and text.
#[tauri::command]
pub fn theme_set_defaults(
    app: AppHandle,
    state: State<'_, AppState>,
    defaults: ThemeDefaults,
) -> AppResult<()> {
    state.database().themes().set_defaults(&defaults)?;
    sync_themes(&app, &state)
}

/// Font families for the theme designer: the bundled ones first, then the
/// fonts installed on this computer, alphabetically.
#[tauri::command]
pub async fn fonts_list() -> Vec<String> {
    let installed = installed_fonts();

    BUNDLED_FONTS
        .iter()
        .map(|name| (*name).to_owned())
        .chain(
            installed
                .into_iter()
                .filter(|name| !BUNDLED_FONTS.contains(&name.as_str())),
        )
        .collect()
}

/// The font families installed on Windows, by the names the output window
/// knows them by.
///
/// The output (WebView2) finds fonts through DirectWrite, so the list comes
/// from DirectWrite too. It includes fonts that installers and font managers
/// put outside `C:\Windows\Fonts` (Windows keeps those in the registry),
/// and fonts installed while SimpleWorship is open.
#[cfg(windows)]
fn installed_fonts() -> BTreeSet<String> {
    dwrote::FontCollection::get_system(true)
        .families_iter()
        .filter_map(|family| family.family_name().ok())
        .collect()
}

/// The font families installed on macOS and Linux: the system's and the
/// user's font folders (and, on Linux, fontconfig's).
#[cfg(not(windows))]
fn installed_fonts() -> BTreeSet<String> {
    let mut database = fontdb::Database::new();
    database.load_system_fonts();

    database
        .faces()
        .flat_map(|face| face.families.iter().map(|(name, _)| name.clone()))
        // Hidden system fonts on macOS start with a dot.
        .filter(|name| !name.starts_with('.'))
        .collect()
}

/// Sets (or, with `None`, removes) the church logo.
#[tauri::command]
pub fn logo_set(app: AppHandle, state: State<'_, AppState>, media_id: Option<Id>) -> AppResult<()> {
    let logo = match media_id {
        Some(id) => Some(
            state
                .database()
                .media()
                .get(id)?
                .ok_or(AppError::MediaNotFound)?
                .as_ref(),
        ),
        None => None,
    };
    state.database().settings().set(LOGO_KEY, &logo)?;
    dispatch(&app, &state, Command::SetLogoImage { logo })
}

/// Sends the theme library and the logo from the database to the engine.
/// Called at startup and after every change.
pub fn sync_themes(app: &AppHandle, state: &AppState) -> AppResult<()> {
    let library = state.database().themes().library()?;
    dispatch(
        app,
        state,
        Command::SetThemes {
            library: Box::new(library),
        },
    )
}

/// Gives the engine the settings it needs: the CCLI licence number (from
/// the Settings dialog) and this computer's time zone (for countdowns to a
/// time of day). Called at startup.
pub fn sync_settings(app: &AppHandle, state: &AppState) -> AppResult<()> {
    let settings = crate::settings::load(&state.database())?;
    dispatch(
        app,
        state,
        Command::SetCcliLicense {
            license: crate::settings::license(&settings),
        },
    )?;
    let minutes = chrono::Local::now().offset().local_minus_utc() / 60;
    dispatch(app, state, Command::SetUtcOffset { minutes })?;
    dispatch(
        app,
        state,
        Command::SetAutoPreviewNext {
            enabled: settings.auto_preview_next,
        },
    )?;
    let transitions = crate::settings::load_transitions(&state.database())?;
    dispatch(app, state, Command::SetTransitions { transitions })?;
    let settings = crate::settings::load_stream(&state.database())?;
    dispatch(app, state, Command::SetStreamSettings { settings })
}

/// Loads the saved logo into the engine. Called at startup.
pub fn sync_logo(app: &AppHandle, state: &AppState) -> AppResult<()> {
    let logo: Option<MediaRef> = state.database().settings().get(LOGO_KEY)?.flatten();
    dispatch(app, state, Command::SetLogoImage { logo })
}

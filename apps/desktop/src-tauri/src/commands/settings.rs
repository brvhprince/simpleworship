//! The Settings dialog, and the CCLI usage report.

use std::fmt::Write as _;

use sw_core::Command;
use sw_core::StreamSettings;
use sw_core::lower_thirds::LowerThird;
use sw_core::output::TransitionSettings;
use sw_core::usage::SongUsage;
use tauri::{AppHandle, State};

use crate::commands::engine::dispatch;
use crate::error::AppResult;
use crate::settings::{self, AppSettings};
use crate::state::AppState;

/// Broadcast with the new settings whenever they are saved.
pub const SETTINGS_EVENT: &str = "settings://changed";

/// The current settings.
#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> AppResult<AppSettings> {
    settings::load(&state.database())
}

/// Saves the settings and passes the CCLI licence on to the engine.
#[tauri::command]
pub fn settings_save(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> AppResult<()> {
    settings::save(&state.database(), &settings)?;
    crate::remote::share(&app, SETTINGS_EVENT, &settings);
    dispatch(
        &app,
        &state,
        Command::SetAutoPreviewNext {
            enabled: settings.auto_preview_next,
        },
    )?;
    dispatch(
        &app,
        &state,
        Command::SetCcliLicense {
            license: settings::license(&settings),
        },
    )
}

/// Changes and saves the transition for each kind of content.
#[tauri::command]
pub fn transitions_save(
    app: AppHandle,
    state: State<'_, AppState>,
    transitions: TransitionSettings,
) -> AppResult<()> {
    settings::save_transitions(&state.database(), &transitions)?;
    dispatch(&app, &state, Command::SetTransitions { transitions })
}

/// Changes and saves how the stream output shows each kind of content.
#[tauri::command]
pub fn stream_save(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: StreamSettings,
) -> AppResult<()> {
    crate::settings::save_stream(&state.database(), &settings)?;
    dispatch(&app, &state, Command::SetStreamSettings { settings })
}

/// The saved lower thirds, in the order shown.
#[tauri::command]
pub fn lower_thirds_get(state: State<'_, AppState>) -> AppResult<Vec<LowerThird>> {
    settings::load_lower_thirds(&state.database())
}

/// Saves the lower thirds (the whole list, in order), if the saved list is
/// still `previous`, the one this change was made to.
///
/// Every save writes the whole list, so one made to an out-of-date copy
/// (on a computer that joined this one, while a pack was imported here)
/// would drop what was added meanwhile: it is refused instead, and the
/// window reloads. Without `previous` (an older joined computer) it saves.
#[tauri::command]
pub fn lower_thirds_save(
    state: State<'_, AppState>,
    items: Vec<LowerThird>,
    previous: Option<Vec<LowerThird>>,
) -> AppResult<()> {
    settings::save_lower_thirds_over(&state.database(), &items, previous.as_deref())
}

/// Songs used from `from` to `to` (inclusive, `YYYY-MM-DD`), most used first.
#[tauri::command]
pub fn usage_report(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> AppResult<Vec<SongUsage>> {
    Ok(state.database().usage().report(&from, &to)?)
}

/// Writes the usage report as a CSV file (for CCLI's reporting site or a
/// spreadsheet).
#[tauri::command]
pub fn usage_export_csv(
    state: State<'_, AppState>,
    from: String,
    to: String,
    path: String,
) -> AppResult<()> {
    let rows = state.database().usage().report(&from, &to)?;
    let mut csv = String::from("Title,Author,Copyright,CCLI Song Number,Times Used,Last Used\n");
    for row in rows {
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{}",
            quote(&row.title),
            quote(&row.author),
            quote(&row.copyright),
            quote(&row.ccli_number),
            row.times,
            row.last_used
        );
    }
    crate::files::write_atomically(std::path::Path::new(&path), csv.as_bytes())?;
    Ok(())
}

/// A CSV field: quoted, with quotes doubled.
fn quote(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_csv_fields() {
        assert_eq!(quote(r#"He said "Holy""#), r#""He said ""Holy""""#);
        assert_eq!(quote("A, B"), "\"A, B\"");
    }
}

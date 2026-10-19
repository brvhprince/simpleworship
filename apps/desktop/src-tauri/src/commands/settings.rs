//! The Settings dialog, and the CCLI usage report.

use std::fmt::Write as _;

use sw_core::Command;
use sw_core::usage::SongUsage;
use tauri::{AppHandle, Emitter, State};

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
    app.emit(SETTINGS_EVENT, &settings)?;
    dispatch(
        &app,
        &state,
        Command::SetCcliLicense {
            license: settings::license(&settings),
        },
    )
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

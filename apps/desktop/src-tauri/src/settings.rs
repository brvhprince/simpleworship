//! The church's settings: name, CCLI licence, alert templates and stage
//! display options, kept in the library's settings table.

use serde::{Deserialize, Serialize};
use sw_core::StreamSettings;
use sw_core::alerts::{AlertTemplate, starter_templates};
use sw_core::output::TransitionSettings;
use sw_db::Database;
use ts_rs::TS;

use crate::error::AppResult;

const SETTINGS_KEY: &str = "appSettings";

/// What the stage display shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "desktop.ts")]
pub struct StageOptions {
    /// Show the time of day.
    pub show_clock: bool,
    /// Show the next slide under the current one.
    pub show_next: bool,
    /// Text size, as a percentage of normal.
    pub text_scale: u16,
}

impl Default for StageOptions {
    fn default() -> Self {
        Self {
            show_clock: true,
            show_next: true,
            text_scale: 100,
        }
    }
}

/// Everything in the Settings dialog.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "desktop.ts")]
pub struct AppSettings {
    /// The church's name (used in reports).
    pub church_name: String,
    /// The church's CCLI licence number, added to song footers. Empty to
    /// leave it off.
    pub ccli_license: String,
    /// Alerts the operator can show in one click.
    pub alert_templates: Vec<AlertTemplate>,
    /// Stage display options.
    pub stage: StageOptions,
    /// Look for a new version at startup and once a day. Updates are only
    /// ever installed when the operator says so.
    pub check_for_updates: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            church_name: String::new(),
            ccli_license: String::new(),
            alert_templates: starter_templates(),
            stage: StageOptions::default(),
            check_for_updates: true,
        }
    }
}

/// The saved settings, or the defaults.
pub fn load(database: &Database) -> AppResult<AppSettings> {
    Ok(database.settings().get(SETTINGS_KEY)?.unwrap_or_default())
}

/// Saves the settings.
pub fn save(database: &Database, settings: &AppSettings) -> AppResult<()> {
    Ok(database.settings().set(SETTINGS_KEY, settings)?)
}

/// Settings key for the transitions (saved separately: the toolbar changes
/// them, not the Settings dialog).
const TRANSITIONS_KEY: &str = "transitions";

/// The saved transitions, or the defaults.
pub fn load_transitions(database: &Database) -> AppResult<TransitionSettings> {
    Ok(database
        .settings()
        .get(TRANSITIONS_KEY)?
        .unwrap_or_default())
}

/// Saves the transitions, so they are the same after a restart.
pub fn save_transitions(database: &Database, transitions: &TransitionSettings) -> AppResult<()> {
    Ok(database.settings().set(TRANSITIONS_KEY, transitions)?)
}

/// Settings key for the stream look.
const STREAM_KEY: &str = "stream";

/// The saved stream look, or the defaults (songs and scripture as a lower
/// third, everything else as on the main output).
pub fn load_stream(database: &Database) -> AppResult<StreamSettings> {
    Ok(database.settings().get(STREAM_KEY)?.unwrap_or_default())
}

/// Saves the stream look.
pub fn save_stream(database: &Database, stream: &StreamSettings) -> AppResult<()> {
    Ok(database.settings().set(STREAM_KEY, stream)?)
}

/// The licence number to put on slides, if one is set.
pub fn license(settings: &AppSettings) -> Option<String> {
    Some(settings.ccli_license.trim().to_owned()).filter(|number| !number.is_empty())
}

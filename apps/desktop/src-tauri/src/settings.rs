//! The church's settings: name, CCLI licence, alert templates and stage
//! display options, kept in the library's settings table.

use serde::{Deserialize, Serialize};
use sw_core::StreamSettings;
use sw_core::alerts::{AlertTemplate, starter_templates};
use sw_core::lower_thirds::{LowerThird, starter_lower_thirds};
use sw_core::output::TransitionSettings;
use sw_db::Database;
use ts_rs::TS;

use crate::error::{AppError, AppResult};

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
    /// Automatically put the next schedule item into preview when an item goes live.
    pub auto_preview_next: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            church_name: String::new(),
            ccli_license: String::new(),
            alert_templates: starter_templates(),
            stage: StageOptions::default(),
            check_for_updates: true,
            auto_preview_next: true,
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

/// Settings key for the lower thirds.
const LOWER_THIRDS_KEY: &str = "lower-thirds";

/// The saved lower thirds, or one of each kind to start from.
pub fn load_lower_thirds(database: &Database) -> AppResult<Vec<LowerThird>> {
    Ok(saved_lower_thirds(database)?.unwrap_or_else(starter_lower_thirds))
}

/// The saved lower thirds, or `None` if none were ever saved (the starters
/// are then made afresh, with new ids, each time they are loaded).
pub fn saved_lower_thirds(database: &Database) -> AppResult<Option<Vec<LowerThird>>> {
    Ok(database.settings().get(LOWER_THIRDS_KEY)?)
}

/// Saves the lower thirds, in the order shown.
pub fn save_lower_thirds(database: &Database, items: &[LowerThird]) -> AppResult<()> {
    Ok(database.settings().set(LOWER_THIRDS_KEY, &items)?)
}

/// Saves the lower thirds if the saved list is still `previous`, the one
/// the change was made to (see `commands::settings::lower_thirds_save`).
/// Before anything was saved there is nothing to lose (and the starters
/// get new ids each load, so they could never match).
pub fn save_lower_thirds_over(
    database: &Database,
    items: &[LowerThird],
    previous: &[LowerThird],
) -> AppResult<()> {
    if let Some(saved) = saved_lower_thirds(database)?
        && previous != saved.as_slice()
    {
        return Err(AppError::LowerThirdsChanged);
    }
    save_lower_thirds(database, items)
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

#[cfg(test)]
mod tests {
    use super::*;
    use sw_core::lower_thirds::LowerThirdKind;

    #[test]
    fn a_lower_third_save_made_to_an_old_list_is_refused() {
        let database = Database::open_in_memory().unwrap();
        let name = LowerThird::new(LowerThirdKind::Name, "Pastor", "");
        let momo = LowerThird::new(LowerThirdKind::Offering, "MoMo", "");

        // Nothing saved yet: anything goes (the starters have new ids each load).
        let starters = load_lower_thirds(&database).unwrap();
        save_lower_thirds_over(&database, std::slice::from_ref(&name), &starters).unwrap();

        // Made to the list as saved: saved.
        let both = [name.clone(), momo];
        save_lower_thirds_over(&database, &both, std::slice::from_ref(&name)).unwrap();

        // Made to the list before (a pack added one meanwhile): refused.
        let stale = save_lower_thirds_over(&database, &[], std::slice::from_ref(&name));
        assert!(matches!(stale, Err(AppError::LowerThirdsChanged)));
        assert_eq!(load_lower_thirds(&database).unwrap(), both);
    }
}

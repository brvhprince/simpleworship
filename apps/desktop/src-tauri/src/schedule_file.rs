//! Which file the current schedule belongs to, and whether it has unsaved
//! changes.
//!
//! The schedule itself lives in the engine. This module tracks the file
//! around it and tells the UI (through [`SCHEDULE_FILE_EVENT`]) whenever the
//! file name or the unsaved-changes flag changes, so the window title can
//! show "Sunday.sws •".

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use ts_rs::TS;

/// The event channel file status updates are broadcast on.
/// Keep in sync with `apps/desktop/src/lib/schedule-file.svelte.ts`.
pub const SCHEDULE_FILE_EVENT: &str = "schedule-file://status";

/// The file behind the current schedule.
#[derive(Debug, Default)]
pub struct ScheduleFile {
    /// Where the schedule was last opened from or saved to. `None` for a
    /// schedule that has never been saved.
    pub path: Option<PathBuf>,
    /// Whether the schedule changed since it was last opened or saved.
    pub dirty: bool,
}

/// What the UI shows about the schedule file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct ScheduleFileStatus {
    /// Full path of the file, or `None` if it was never saved.
    pub path: Option<String>,
    /// The name to show, e.g. "Sunday Service" or "Untitled".
    pub name: String,
    /// Whether there are unsaved changes.
    pub dirty: bool,
}

impl ScheduleFile {
    /// The status to show in the UI.
    pub fn status(&self) -> ScheduleFileStatus {
        ScheduleFileStatus {
            path: self.path.as_ref().map(|p| p.display().to_string()),
            name: self
                .path
                .as_deref()
                .map(display_name)
                .unwrap_or_else(|| "Untitled".to_owned()),
            dirty: self.dirty,
        }
    }
}

/// A file's name without its folder or extension: "/a/Sunday.sws" → "Sunday".
pub fn display_name(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Makes sure a path chosen in a save dialog ends in `.sws`.
pub fn with_schedule_extension(path: PathBuf) -> PathBuf {
    let has_extension = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("sws"));
    if has_extension {
        path
    } else {
        let mut name = path.into_os_string();
        name.push(".sws");
        PathBuf::from(name)
    }
}

/// Tells every window about a new file status.
pub fn announce(app: &AppHandle, status: &ScheduleFileStatus) {
    crate::remote::share(app, SCHEDULE_FILE_EVENT, status);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_extensions() {
        assert_eq!(
            display_name(Path::new("/x/Sunday Service.sws")),
            "Sunday Service"
        );
        assert_eq!(
            with_schedule_extension(PathBuf::from("/x/Sunday")),
            PathBuf::from("/x/Sunday.sws")
        );
        assert_eq!(
            with_schedule_extension(PathBuf::from("/x/Sunday.SWS")),
            PathBuf::from("/x/Sunday.SWS")
        );
    }

    #[test]
    fn unsaved_schedules_are_untitled() {
        let file = ScheduleFile::default();
        assert_eq!(file.status().name, "Untitled");
        assert_eq!(file.status().path, None);
    }
}

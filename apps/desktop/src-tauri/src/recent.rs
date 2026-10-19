//! The list of recently used schedule files ("Open Recent").

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::files::write_atomically;
use crate::schedule_file::display_name;

const RECENT_FILE: &str = "recent-schedules.json";

/// How many recent schedules to remember.
const MAX_RECENT: usize = 10;

/// One entry in the recent list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct RecentSchedule {
    /// Full path of the file.
    pub path: String,
    /// The name to show, e.g. "Sunday Service".
    pub name: String,
}

/// Recently opened or saved schedules, most recent first, stored as JSON in
/// the app data folder.
pub struct RecentSchedules {
    file: PathBuf,
    paths: Vec<PathBuf>,
}

impl RecentSchedules {
    /// Loads the list from `directory`. A missing or unreadable list is
    /// simply empty: it is a convenience, not data worth failing over.
    pub fn load(directory: &Path) -> Self {
        let file = directory.join(RECENT_FILE);
        let paths = fs::read(&file)
            .ok()
            .and_then(|json| serde_json::from_slice(&json).ok())
            .unwrap_or_default();
        Self { file, paths }
    }

    /// Moves `path` to the top of the list and saves the list.
    pub fn add(&mut self, path: &Path) {
        self.paths.retain(|existing| existing != path);
        self.paths.insert(0, path.to_owned());
        self.paths.truncate(MAX_RECENT);

        let saved = serde_json::to_vec_pretty(&self.paths)
            .map_err(std::io::Error::other)
            .and_then(|json| write_atomically(&self.file, &json));
        if let Err(error) = saved {
            eprintln!("could not save the recent schedules list: {error}");
        }
    }

    /// The entries whose files still exist, most recent first.
    pub fn list(&self) -> Vec<RecentSchedule> {
        self.paths
            .iter()
            .filter(|path| path.exists())
            .map(|path| RecentSchedule {
                path: path.display().to_string(),
                name: display_name(path),
            })
            .collect()
    }
}

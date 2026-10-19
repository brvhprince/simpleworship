//! EasyWorship's media library: the pictures, videos and sound files it
//! shows, so they can be copied into SimpleWorship's media library.
//!
//! The list is in `Media.db`, table `media`, next to the songs. The files
//! themselves are in the profile's `Resources` folder:
//!
//! ```text
//! …\Softouch\EasyWorship\Default\
//!     Resources\Images, Videos, Audio, SharedMedia, …
//!     v6.1\Databases\Data\Media.db
//! ```
//!
//! A row's `path` is usually a placeholder such as `<videos>` (meaning the
//! `Videos` folder) and `filename` is the file's name. Rows of type 3 are
//! live camera feeds, not files, and are left out.
//!
//! When a library was copied to another computer, its `Resources` folder
//! may be somewhere else (or missing): [`locate`] also searches folders the
//! operator points to.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::easyworship::open_read_only;

/// How many folder levels up from `Data` to look for `Resources`.
const PROFILE_LEVELS: usize = 4;

/// How deep to look for files below a folder searched by name.
const SEARCH_DEPTH: usize = 5;

/// EasyWorship's `media_type` for a live camera feed.
const LIVE_FEED: i64 = 3;

/// One item of EasyWorship's media library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EasyWorshipMedia {
    /// The title shown in EasyWorship, e.g. "Loops Waves".
    pub title: String,
    /// The file's name, e.g. "Loops Waves.mp4".
    pub file_name: String,
    /// The `Resources` subfolder its placeholder names, e.g. "Videos".
    pub folder: Option<String>,
    /// A real folder, for media EasyWorship kept where it was found.
    pub absolute_folder: Option<String>,
}

/// Reads the media library from an EasyWorship `Data` folder. A library
/// without `Media.db` (older versions) has no media to import.
pub fn read_media(data_folder: &Path) -> Result<Vec<EasyWorshipMedia>, String> {
    let database = data_folder.join("Media.db");
    if !database.is_file() {
        return Ok(Vec::new());
    }
    let read = || -> rusqlite::Result<Vec<EasyWorshipMedia>> {
        let connection = open_read_only(&database)?;
        // No sorting or comparing on text columns: they use a collation only
        // EasyWorship knows (see the `easyworship` module).
        let mut statement = connection
            .prepare("SELECT media_type, title, path, filename FROM media ORDER BY rowid")?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        let mut media = Vec::new();
        for row in rows {
            let (kind, title, path, file_name) = row?;
            let file_name = file_name.unwrap_or_default();
            if kind == Some(LIVE_FEED) || file_name.trim().is_empty() {
                continue;
            }
            let path = path.unwrap_or_default();
            let (folder, absolute_folder) = match placeholder(&path) {
                Some(name) => (Some(name), None),
                None if path.trim().is_empty() => (None, None),
                None => (None, Some(path)),
            };
            media.push(EasyWorshipMedia {
                title: title
                    .filter(|t| !t.trim().is_empty())
                    .unwrap_or_else(|| stem(&file_name)),
                file_name,
                folder,
                absolute_folder,
            });
        }
        Ok(media)
    };
    read().map_err(|error| format!("could not read the EasyWorship media library: {error}"))
}

/// `<videos>` → `Videos`.
fn placeholder(path: &str) -> Option<String> {
    let inner = path.trim().strip_prefix('<')?.strip_suffix('>')?;
    let mut letters = inner.chars();
    let first = letters.next()?;
    Some(first.to_uppercase().chain(letters).collect())
}

fn stem(file_name: &str) -> String {
    Path::new(file_name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| file_name.to_owned())
}

/// The profile's `Resources` folder, found by going up from `Data`.
pub fn resources_folder(data_folder: &Path) -> Option<PathBuf> {
    data_folder
        .ancestors()
        .take(PROFILE_LEVELS + 1)
        .map(|folder| folder.join("Resources"))
        .find(|resources| resources.is_dir())
}

/// Finds each item's file: in its `Resources` subfolder, in its own folder,
/// or by name anywhere below `Resources` and the `extra` folders. Returns
/// one entry per item, `None` where the file could not be found.
pub fn locate(
    data_folder: &Path,
    media: &[EasyWorshipMedia],
    extra: &[PathBuf],
) -> Vec<Option<PathBuf>> {
    let resources = resources_folder(data_folder);
    let mut index: Option<HashMap<String, PathBuf>> = None;

    media
        .iter()
        .map(|item| {
            let direct = [
                resources
                    .as_ref()
                    .zip(item.folder.as_ref())
                    .map(|(resources, folder)| resources.join(folder).join(&item.file_name)),
                item.absolute_folder
                    .as_ref()
                    .map(|folder| Path::new(folder).join(&item.file_name)),
            ];
            if let Some(found) = direct.into_iter().flatten().find(|path| path.is_file()) {
                return Some(found);
            }
            // Build the by-name index only if something was not where expected.
            let index = index.get_or_insert_with(|| {
                let mut index = HashMap::new();
                for folder in extra.iter().chain(resources.iter()) {
                    index_files(folder, SEARCH_DEPTH, &mut index);
                }
                index
            });
            index.get(&item.file_name.to_lowercase()).cloned()
        })
        .collect()
}

/// Records every file below `folder` by its lower-case name (the first one
/// found wins).
fn index_files(folder: &Path, depth: usize, index: &mut HashMap<String, PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth > 0 {
                index_files(&path, depth - 1, index);
            }
        } else if let Some(name) = path.file_name() {
            index
                .entry(name.to_string_lossy().to_lowercase())
                .or_insert(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// A profile laid out like EasyWorship's, with two files on disk.
    fn make_profile(root: &Path) -> PathBuf {
        let data = root.join("Default/v6.1/Databases/Data");
        std::fs::create_dir_all(&data).unwrap();
        for (folder, file) in [("Videos", "Loops Waves.mp4"), ("Images", "cross.jpg")] {
            let dir = root.join("Default/Resources").join(folder);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(file), b"x").unwrap();
        }
        let db = Connection::open(data.join("Media.db")).unwrap();
        db.create_collation("UTF8_U_CI", |a, b| a.to_lowercase().cmp(&b.to_lowercase()))
            .unwrap();
        db.execute_batch(
            "CREATE TABLE media (rowid integer PRIMARY KEY, media_type int8,
                title text NOT NULL COLLATE UTF8_U_CI, path text, filename text COLLATE UTF8_U_CI);
             INSERT INTO media (media_type, title, path, filename) VALUES
                (1, 'Loops Waves', '<videos>', 'Loops Waves.mp4'),
                (2, 'Cross Sunrise', '<images>', 'cross.jpg'),
                (2, 'Moved Away', '<images>', 'gone.png'),
                (3, 'obs', '', '');",
        )
        .unwrap();
        data
    }

    #[test]
    fn reads_files_and_leaves_out_live_feeds() {
        let root = tempfile::tempdir().unwrap();
        let data = make_profile(root.path());
        let media = read_media(&data).unwrap();
        let titles: Vec<_> = media.iter().map(|item| item.title.as_str()).collect();
        assert_eq!(titles, ["Loops Waves", "Cross Sunrise", "Moved Away"]);
        assert_eq!(media[0].folder.as_deref(), Some("Videos"));
    }

    #[test]
    fn finds_files_in_resources_and_in_other_folders() {
        let root = tempfile::tempdir().unwrap();
        let data = make_profile(root.path());
        let media = read_media(&data).unwrap();

        let found = locate(&data, &media, &[]);
        assert!(
            found[0]
                .as_ref()
                .unwrap()
                .ends_with("Resources/Videos/Loops Waves.mp4")
        );
        assert!(found[1].is_some());
        assert_eq!(found[2], None, "not on disk");

        // The operator points to a folder where it was copied.
        let elsewhere = root.path().join("USB stick");
        std::fs::create_dir_all(&elsewhere).unwrap();
        std::fs::write(elsewhere.join("Gone.PNG"), b"x").unwrap();
        assert!(
            locate(&data, &media, &[elsewhere])[2].is_some(),
            "found by name, any case"
        );
    }

    #[test]
    fn a_library_without_media_has_none() {
        let folder = tempfile::tempdir().unwrap();
        assert_eq!(read_media(folder.path()).unwrap(), Vec::new());
    }
}

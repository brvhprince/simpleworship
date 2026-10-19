//! EasyWorship 6 and 7 song libraries.
//!
//! EasyWorship keeps its library in SQLite files in a `Data` folder, by
//! default `C:\Users\Public\Documents\Softouch\EasyWorship\<profile>\v6.1\Databases\Data`:
//!
//! - `Songs.db`, table `song`: `rowid`, `title`, `author`, `copyright`,
//!   `administrator`, `reference_number` (the CCLI number), …
//! - `SongWords.db`, table `word`: `song_id` (the song's `rowid`) and
//!   `words`, the lyrics as RTF (see [`crate::rtf`]). Section names such as
//!   "Verse 1" are ordinary lines; an empty paragraph starts a new slide.
//!
//! The databases are opened read-only and "immutable", so SimpleWorship
//! can never change an EasyWorship library, even while EasyWorship is
//! running. Their text columns use a collation only EasyWorship defines
//! (`UTF8_U_CI`), so the queries here never sort or compare on them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use crate::lyrics::tag_sections;
use crate::{ImportReport, ImportedSong, rtf};

/// Shown as the source of every song from an EasyWorship library.
const SOURCE: &str = "EasyWorship";

/// How deep [`find_data_folder`] looks below the folder it is given.
const SEARCH_DEPTH: usize = 6;

/// Reads every song from an EasyWorship `Data` folder.
pub fn read_library(data_folder: &Path) -> ImportReport {
    let mut report = ImportReport::default();
    match read(data_folder, &mut report) {
        Ok(()) => {}
        Err(error) => report.problem(
            data_folder.display().to_string(),
            format!("could not read the EasyWorship library: {error}"),
        ),
    }
    report
}

fn read(data_folder: &Path, report: &mut ImportReport) -> rusqlite::Result<()> {
    let songs = open_read_only(&data_folder.join("Songs.db"))?;
    let words = open_read_only(&data_folder.join("SongWords.db"))?;

    let mut lyrics: HashMap<i64, String> = HashMap::new();
    let mut statement = words.prepare("SELECT song_id, words FROM word")?;
    for row in statement.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
    })? {
        let (song_id, rtf) = row?;
        if let Some(rtf) = rtf {
            lyrics.insert(song_id, rtf);
        }
    }

    let mut statement = songs.prepare(
        "SELECT rowid, title, author, copyright, administrator, reference_number FROM song",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            text(row, 1)?,
            text(row, 2)?,
            text(row, 3)?,
            text(row, 4)?,
            text(row, 5)?,
        ))
    })?;

    for row in rows {
        let (id, title, author, copyright, administrator, ccli) = row?;
        let title = title.trim().to_owned();
        let words = lyrics
            .get(&id)
            .map(|rtf| tag_sections(&rtf::to_text(rtf)))
            .unwrap_or_default();

        if title.is_empty() {
            report.problem(
                format!("{SOURCE} song #{id}"),
                "it has no title, so it was skipped",
            );
            continue;
        }
        if words.trim().is_empty() {
            report.problem(title.clone(), "it has no lyrics, so it was skipped");
            continue;
        }

        report.songs.push(ImportedSong {
            title,
            author: author.trim().to_owned(),
            copyright: with_administrator(copyright.trim(), administrator.trim()),
            ccli_number: ccli.trim().to_owned(),
            lyrics: words,
            arrangement: String::new(),
            source: SOURCE.to_owned(),
        });
    }
    Ok(())
}

/// A text column that may be NULL.
fn text(row: &rusqlite::Row<'_>, column: usize) -> rusqlite::Result<String> {
    Ok(row.get::<_, Option<String>>(column)?.unwrap_or_default())
}

/// "© 2004 Example Music" plus "(admin. by Publisher)" when EasyWorship
/// lists an administrator.
fn with_administrator(copyright: &str, administrator: &str) -> String {
    match (copyright.is_empty(), administrator.is_empty()) {
        (_, true) => copyright.to_owned(),
        (true, false) => format!("Admin. by {administrator}"),
        (false, false) => format!("{copyright} (admin. by {administrator})"),
    }
}

/// Opens a SQLite file without ever writing to it, even its journal.
pub(crate) fn open_read_only(path: &Path) -> rusqlite::Result<Connection> {
    let uri = format!("file:{}?immutable=1", uri_path(path));
    Connection::open_with_flags(
        uri,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )
}

/// A path written for a SQLite `file:` URI (forward slashes, `%`, `?` and
/// `#` escaped).
fn uri_path(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "/")
        .replace('%', "%25")
        .replace('?', "%3f")
        .replace('#', "%23")
}

/// Finds an EasyWorship `Data` folder (the one holding `Songs.db` and
/// `SongWords.db`) at or below `start`, which may also be `Songs.db` itself.
pub fn find_data_folder(start: &Path) -> Option<PathBuf> {
    if start.is_file() {
        return start
            .parent()
            .filter(|folder| is_data_folder(folder))
            .map(Path::to_path_buf);
    }
    find_below(start, SEARCH_DEPTH, &is_data_folder)
}

/// Finds EasyWorship Bible files (`*.ewb`) at or below `start`.
pub fn find_bibles(start: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect_files(start, SEARCH_DEPTH, "ewb", &mut found);
    found.sort();
    found
}

fn is_data_folder(folder: &Path) -> bool {
    folder.join("Songs.db").is_file() && folder.join("SongWords.db").is_file()
}

/// Depth-limited search for a folder matching `wanted`.
fn find_below(folder: &Path, depth: usize, wanted: &dyn Fn(&Path) -> bool) -> Option<PathBuf> {
    if wanted(folder) {
        return Some(folder.to_path_buf());
    }
    if depth == 0 {
        return None;
    }
    let mut children: Vec<PathBuf> = std::fs::read_dir(folder)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    children.sort();
    children
        .iter()
        .find_map(|child| find_below(child, depth - 1, wanted))
}

fn collect_files(folder: &Path, depth: usize, extension: &str, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_dir() && depth > 0 {
            collect_files(&path, depth - 1, extension, found);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case(extension))
        {
            found.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a tiny library with EasyWorship's real schema (including its
    /// custom collation, which plain SQLite does not know).
    fn make_library(folder: &Path) {
        let songs = Connection::open(folder.join("Songs.db")).unwrap();
        songs
            .create_collation("UTF8_U_CI", |a, b| a.to_lowercase().cmp(&b.to_lowercase()))
            .unwrap();
        songs
            .execute_batch(
                "CREATE TABLE song (
                    rowid integer PRIMARY KEY AUTOINCREMENT NOT NULL UNIQUE,
                    song_item_uid text UNIQUE, song_rev_uid text, song_uid text,
                    title text NOT NULL COLLATE UTF8_U_CI, author text COLLATE UTF8_U_CI,
                    copyright text COLLATE UTF8_U_CI, administrator text COLLATE UTF8_U_CI,
                    description text, tags text COLLATE UTF8_U_CI,
                    reference_number text COLLATE UTF8_U_CI, vendor_id integer,
                    presentation_id integer, layout_revision integer DEFAULT 1,
                    revision integer DEFAULT 1);
                 CREATE INDEX song_Index01 ON song (title);
                 INSERT INTO song (title, author, copyright, administrator, reference_number)
                 VALUES ('Amazing Grace', 'John Newton', 'Public Domain', '', '22025'),
                        ('Untitled Words', NULL, NULL, NULL, NULL),
                        ('', 'Nobody', '', '', '');",
            )
            .unwrap();

        let words = Connection::open(folder.join("SongWords.db")).unwrap();
        words
            .execute_batch(
                r"CREATE TABLE word (rowid integer PRIMARY KEY NOT NULL UNIQUE, song_id integer,
                    words rtf, slide_uids text, slide_layout_revisions int64a, slide_revisions int64a);
                 INSERT INTO word (song_id, words) VALUES (1,
                 '{\rtf1\ansi{\fonttbl{\f0 Arial;}}{\pard\plain\f1\fntnamaut Verse 1\par
                  \plain\f1\fntnamaut Amazing grace how sweet the sound\par
                  \par
                  \plain\f1\fntnamaut Verse 2\par
                  \plain\f1\fntnamaut T\u8217?was grace that taught\par}}');",
            )
            .unwrap();
    }

    #[test]
    fn reads_songs_and_reports_the_rest() {
        let folder = tempfile::tempdir().unwrap();
        make_library(folder.path());

        let report = read_library(folder.path());
        assert_eq!(report.songs.len(), 1);
        let song = &report.songs[0];
        assert_eq!(song.title, "Amazing Grace");
        assert_eq!(song.ccli_number, "22025");
        assert_eq!(
            song.lyrics,
            "[Verse 1]\nAmazing grace how sweet the sound\n\n[Verse 2]\nT\u{2019}was grace that taught"
        );

        let problems: Vec<_> = report.problems.iter().map(|p| p.message.as_str()).collect();
        assert_eq!(problems.len(), 2, "{problems:?}");
    }

    #[test]
    fn finds_the_data_folder_and_bibles() {
        let root = tempfile::tempdir().unwrap();
        let data = root
            .path()
            .join("Softouch/EasyWorship/Default/v6.1/Databases/Data");
        let bibles = root.path().join("Plugins/Bibles");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::create_dir_all(&bibles).unwrap();
        make_library(&data);
        std::fs::write(bibles.join("KJV.ewb"), b"").unwrap();

        assert_eq!(find_data_folder(root.path()), Some(data.clone()));
        assert_eq!(find_data_folder(&data.join("Songs.db")), Some(data));
        assert_eq!(find_bibles(root.path()), [bibles.join("KJV.ewb")]);
    }

    #[test]
    fn credits_the_administrator() {
        assert_eq!(
            with_administrator("© 2004 Example", "Music Co"),
            "© 2004 Example (admin. by Music Co)"
        );
        assert_eq!(with_administrator("", "Music Co"), "Admin. by Music Co");
        assert_eq!(with_administrator("Public Domain", ""), "Public Domain");
    }
}

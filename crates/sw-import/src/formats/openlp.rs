//! OpenLP song databases (`songs.sqlite`).
//!
//! OpenLP runs on Windows, macOS and Linux and keeps its songs in a SQLite
//! file: table `songs` (`title`, `lyrics`, `verse_order`, `copyright`,
//! `ccli_number`) with authors in `authors` / `authors_songs`. The lyrics
//! are XML:
//!
//! ```xml
//! <song version="1.0"><lyrics>
//!   <verse type="v" label="1"><![CDATA[Amazing grace…]]></verse>
//! </lyrics></song>
//! ```
//!
//! Inside a verse, `[---]` marks where OpenLP may split the slide, and
//! `{tag}` … `{/tag}` are formatting tags, which are dropped.

use std::collections::HashMap;
use std::path::Path;

use super::easyworship::open_read_only;
use super::openlyrics::tidy_lines;
use crate::lyrics::{arrangement_from_codes, section_name_for_code};
use crate::xml::{XmlEvent, for_each_event};
use crate::{ImportReport, ImportedSong};

const SOURCE: &str = "OpenLP";

/// Reads every song from an OpenLP `songs.sqlite`.
pub fn read_library(database: &Path) -> ImportReport {
    let mut report = ImportReport::default();
    if let Err(error) = read(database, &mut report) {
        report.problem(
            database.display().to_string(),
            format!("could not read the OpenLP library: {error}"),
        );
    }
    report
}

fn read(database: &Path, report: &mut ImportReport) -> rusqlite::Result<()> {
    let conn = open_read_only(database)?;

    let mut authors: HashMap<i64, Vec<String>> = HashMap::new();
    let mut statement = conn.prepare(
        "SELECT authors_songs.song_id, authors.display_name
         FROM authors_songs JOIN authors ON authors.id = authors_songs.author_id",
    )?;
    for row in statement.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })? {
        let (song, name) = row?;
        authors.entry(song).or_default().push(name);
    }

    let mut statement =
        conn.prepare("SELECT id, title, lyrics, verse_order, copyright, ccli_number FROM songs")?;
    let rows = statement.query_map([], |row| {
        let text = |i| {
            row.get::<_, Option<String>>(i)
                .map(Option::unwrap_or_default)
        };
        Ok((
            row.get::<_, i64>(0)?,
            text(1)?,
            text(2)?,
            text(3)?,
            text(4)?,
            text(5)?,
        ))
    })?;

    for row in rows {
        let (id, title, lyrics_xml, order, copyright, ccli) = row?;
        let song = ImportedSong {
            title: title.trim().to_owned(),
            author: authors.remove(&id).unwrap_or_default().join(", "),
            copyright: copyright.trim().to_owned(),
            ccli_number: ccli.trim().to_owned(),
            lyrics: lyrics_from_xml(&lyrics_xml),
            arrangement: arrangement_from_codes(&order),
            source: SOURCE.to_owned(),
        };
        super::openlyrics::push_if_complete(report, song);
    }
    Ok(())
}

/// Turns OpenLP's verse XML into tagged lyrics.
fn lyrics_from_xml(xml: &str) -> String {
    let mut lyrics = String::new();
    let mut in_verse = false;
    let _ = for_each_event(xml, |event| match event {
        XmlEvent::Start {
            name, attributes, ..
        } if name == "verse" => {
            let kind = attributes.get("type").map_or("v", String::as_str);
            let label = attributes.get("label").map_or("", String::as_str);
            if !lyrics.is_empty() {
                lyrics.push_str("\n\n");
            }
            lyrics.push_str(&format!(
                "[{}]\n",
                section_name_for_code(&format!("{kind}{label}"))
            ));
            in_verse = true;
        }
        XmlEvent::End { name } if name == "verse" => in_verse = false,
        XmlEvent::Text(text) if in_verse => lyrics.push_str(&clean_verse(&text)),
        _ => {}
    });
    tidy_lines(&lyrics)
}

/// Optional splits become slide breaks; formatting tags are removed.
fn clean_verse(text: &str) -> String {
    let text = text.replace("[---]", "\n\n");
    let mut clean = String::with_capacity(text.len());
    let mut in_tag = false;
    for c in text.chars() {
        match c {
            '{' => in_tag = true,
            '}' if in_tag => in_tag = false,
            _ if !in_tag => clean.push(c),
            _ => {}
        }
    }
    clean.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn reads_an_openlp_library() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("songs.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            r#"CREATE TABLE songs (id INTEGER PRIMARY KEY, title TEXT, alternate_title TEXT,
                 lyrics TEXT, verse_order TEXT, copyright TEXT, ccli_number TEXT);
               CREATE TABLE authors (id INTEGER PRIMARY KEY, display_name TEXT);
               CREATE TABLE authors_songs (author_id INTEGER, song_id INTEGER);
               INSERT INTO songs VALUES (1, 'Amazing Grace', '',
                 '<?xml version=''1.0'' encoding=''UTF-8''?><song version="1.0"><lyrics><verse type="v" label="1"><![CDATA[Amazing {st}grace{/st}
how sweet[---]That saved]]></verse><verse type="c" label="1"><![CDATA[My chains]]></verse></lyrics></song>',
                 'v1 c1 v1', 'Public Domain', '22025');
               INSERT INTO authors VALUES (1, 'John Newton');
               INSERT INTO authors_songs VALUES (1, 1);"#,
        )
        .unwrap();
        drop(conn);

        let report = read_library(&path);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        let song = &report.songs[0];
        assert_eq!(song.author, "John Newton");
        assert_eq!(song.arrangement, "V1 C1 V1");
        assert_eq!(
            song.lyrics,
            "[Verse 1]\nAmazing grace\nhow sweet\n\nThat saved\n\n[Chorus 1]\nMy chains"
        );
    }
}

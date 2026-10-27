//! # sw-import
//!
//! Brings songs in from other worship software and song files:
//!
//! | Source              | Module                         |
//! | ------------------- | ------------------------------ |
//! | EasyWorship 6 / 7   | [`formats::easyworship`]       |
//! | OpenLP              | [`formats::openlp`]            |
//! | ProPresenter 6      | [`formats::propresenter`]      |
//! | OpenLyrics          | [`formats::openlyrics`]        |
//! | ChordPro            | [`formats::chordpro`]          |
//! | CCLI SongSelect     | [`formats::songselect`]        |
//! | Plain text          | [`formats::plain`]             |
//!
//! Every importer produces [`ImportedSong`]s, never writes to the library,
//! and reports problems instead of failing: one bad file must not stop a
//! whole library from coming across.
//!
//! [`lyrics::tag_sections`] turns heading lines such as "Verse 1" into
//! SimpleWorship's `[Verse 1]` tags, and [`duplicates`] spots songs that
//! are already in the library.

pub mod duplicates;
pub mod formats;
pub mod lyrics;
/// The RTF reader, shared with the Bible importer (EasyWorship Bibles keep
/// their copyright notice as RTF).
pub use sw_rtf as rtf;
mod xml;

use serde::{Deserialize, Serialize};
use sw_core::{Id, Song};
use ts_rs::TS;

/// A song read from another program or file, before it is saved.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "import.ts")]
pub struct ImportedSong {
    /// Song title.
    pub title: String,
    /// Author(s).
    pub author: String,
    /// Copyright notice.
    pub copyright: String,
    /// CCLI song number, if known.
    pub ccli_number: String,
    /// Lyrics with `[Section]` tags, as SimpleWorship stores them.
    pub lyrics: String,
    /// Section order, e.g. "V1 C V2 C", if the source had one.
    pub arrangement: String,
    /// Where it came from, for the import list (a file name, or
    /// "EasyWorship" for a library).
    pub source: String,
}

impl ImportedSong {
    /// The song as it will be saved, with a fresh id.
    pub fn into_song(self) -> Song {
        Song {
            id: Id::new(),
            title: self.title,
            author: self.author,
            copyright: self.copyright,
            ccli_number: self.ccli_number,
            lyrics: self.lyrics,
            arrangement: self.arrangement,
            theme_id: None,
            created_at: 0,
            updated_at: 0,
        }
    }
}

/// Something that went wrong with one file or one song.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "import.ts")]
pub struct ImportProblem {
    /// The file (or song) it is about.
    pub source: String,
    /// What went wrong, in words an operator understands.
    pub message: String,
}

/// The outcome of reading one or more sources.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "import.ts")]
pub struct ImportReport {
    /// Every song found.
    pub songs: Vec<ImportedSong>,
    /// Everything that could not be read.
    pub problems: Vec<ImportProblem>,
}

impl ImportReport {
    /// Adds another report's songs and problems to this one.
    pub fn extend(&mut self, other: ImportReport) {
        self.songs.extend(other.songs);
        self.problems.extend(other.problems);
    }

    /// Records a problem.
    pub fn problem(&mut self, source: impl Into<String>, message: impl Into<String>) {
        self.problems.push(ImportProblem {
            source: source.into(),
            message: message.into(),
        });
    }
}

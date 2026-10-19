//! The `.sws` schedule file.
//!
//! A `.sws` file is a zip archive containing:
//!
//! - `schedule.json`: `{ "formatVersion", "generator", "schedule", "themes" }`.
//!   The schedule uses the same JSON shape as the engine's IPC messages.
//!   `themes` holds copies of the library themes the items use, and
//!   `media` describes the embedded media files, so the schedule looks the
//!   same on another computer.
//! - `media/<file>`: the images and videos the schedule uses, when it was
//!   saved with media. They are stored without recompression (they are
//!   already compressed) and are streamed, never loaded into memory whole.
//!
//! ## Versioning
//!
//! `formatVersion` goes up whenever a change would confuse an older
//! SimpleWorship. Opening a file with a *newer* version is refused with a
//! clear message instead of silently losing data. Unknown fields are
//! ignored, so adding optional fields does not need a new version.
//!
//! | Version | Added                          |
//! | ------- | ------------------------------ |
//! | 1       | songs and text items           |
//! | 2       | scripture items                |
//! | 3       | media items and media files    |
//! | 4       | countdowns, presentations, playlists |
//!
//! A file is written with the *lowest* version that can hold it, so a
//! schedule without scripture still opens in versions that only know v1.

use std::io::{Cursor, Read, Seek, Write};

use serde::{Deserialize, Serialize};
use sw_core::{ItemContent, MediaItem, Schedule, ThemeRecord};
use thiserror::Error;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// The newest format version this build reads (and writes, when needed).
pub const FORMAT_VERSION: u32 = 4;

/// Name of the schedule entry inside the archive.
const SCHEDULE_ENTRY: &str = "schedule.json";

/// Folder of the media entries inside the archive.
const MEDIA_FOLDER: &str = "media/";

/// Everything in `schedule.json`, apart from the bookkeeping.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScheduleContents {
    /// The schedule.
    pub schedule: Schedule,
    /// Copies of the library themes the items use.
    pub themes: Vec<ThemeRecord>,
    /// The media library records of the embedded media files.
    pub media: Vec<MediaItem>,
}

/// The contents of `schedule.json`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScheduleDocument {
    /// See the [module docs](self) on versioning.
    format_version: u32,
    /// Which program wrote the file, e.g. "SimpleWorship 0.1.0". For
    /// troubleshooting only; never used to make decisions.
    generator: String,
    /// The schedule itself.
    schedule: Schedule,
    /// Copies of the themes the items use (absent before version 3).
    #[serde(default)]
    themes: Vec<ThemeRecord>,
    /// Records of the embedded media files (absent before version 3).
    #[serde(default)]
    media: Vec<MediaItem>,
}

/// A media file to embed: its file name in the media folder, and its data.
pub struct MediaSource<'a> {
    /// The file name, e.g. `"0192….mp4"`.
    pub file: String,
    /// Where to read the data from (usually the open file).
    pub data: Box<dyn Read + 'a>,
}

/// Why a schedule file could not be read or written.
#[derive(Debug, Error)]
pub enum ScheduleFileError {
    /// The bytes are not a zip archive, or the archive has no schedule.
    #[error("this is not a SimpleWorship schedule file")]
    NotASchedule,

    /// The file was made by a newer SimpleWorship.
    #[error(
        "this schedule was made with a newer version of SimpleWorship \
         (format {found}, this version reads up to {supported}); please update"
    )]
    NewerVersion {
        /// The file's format version.
        found: u32,
        /// The newest version this build reads.
        supported: u32,
    },

    /// The schedule JSON is damaged or not what we expect.
    #[error("the schedule file is damaged: {0}")]
    Damaged(#[from] serde_json::Error),

    /// Reading or writing the archive failed.
    #[error("could not read or write the schedule file: {0}")]
    Io(String),
}

impl ScheduleFileError {
    fn io(error: impl std::fmt::Display) -> Self {
        Self::Io(error.to_string())
    }
}

// ── Reading ──────────────────────────────────────────────────────────────

/// Reads just the schedule from the bytes of a small `.sws` file (such as
/// an autosave). For files that may contain media, use [`ScheduleReader`].
pub fn read_schedule(bytes: &[u8]) -> Result<Schedule, ScheduleFileError> {
    Ok(ScheduleReader::new(Cursor::new(bytes))?
        .contents()?
        .schedule)
}

/// An open `.sws` file, read piece by piece.
pub struct ScheduleReader<R: Read + Seek> {
    archive: ZipArchive<R>,
}

impl<R: Read + Seek> ScheduleReader<R> {
    /// Opens a `.sws` file.
    pub fn new(reader: R) -> Result<Self, ScheduleFileError> {
        let archive = ZipArchive::new(reader).map_err(|_| ScheduleFileError::NotASchedule)?;
        Ok(Self { archive })
    }

    /// Reads `schedule.json`.
    pub fn contents(&mut self) -> Result<ScheduleContents, ScheduleFileError> {
        let mut entry = self
            .archive
            .by_name(SCHEDULE_ENTRY)
            .map_err(|_| ScheduleFileError::NotASchedule)?;
        let mut json = Vec::new();
        entry
            .read_to_end(&mut json)
            .map_err(|_| ScheduleFileError::NotASchedule)?;

        // Check the version before parsing the rest, so a newer file gives
        // the "please update" message rather than a confusing parse error.
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct VersionOnly {
            format_version: u32,
        }
        let VersionOnly { format_version } = serde_json::from_slice(&json)?;
        if format_version > FORMAT_VERSION {
            return Err(ScheduleFileError::NewerVersion {
                found: format_version,
                supported: FORMAT_VERSION,
            });
        }

        let document: ScheduleDocument = serde_json::from_slice(&json)?;
        Ok(ScheduleContents {
            schedule: document.schedule,
            themes: document.themes,
            media: document.media,
        })
    }

    /// The file names of the embedded media files.
    pub fn media_files(&self) -> Vec<String> {
        self.archive
            .file_names()
            .flatten()
            .filter_map(|name| name.strip_prefix(MEDIA_FOLDER).map(str::to_owned))
            .filter(|file| is_plain_file_name(file))
            .collect()
    }

    /// Copies the embedded media file `file` into `out`. Returns how many
    /// bytes were copied.
    pub fn copy_media(
        &mut self,
        file: &str,
        out: &mut impl Write,
    ) -> Result<u64, ScheduleFileError> {
        if !is_plain_file_name(file) {
            return Err(ScheduleFileError::NotASchedule);
        }
        let mut entry = self
            .archive
            .by_name(&format!("{MEDIA_FOLDER}{file}"))
            .map_err(ScheduleFileError::io)?;
        std::io::copy(&mut entry, out).map_err(ScheduleFileError::io)
    }
}

/// A media file name must be a single name: no folders and no `..`, so a
/// crafted schedule file can never write outside the media folder.
fn is_plain_file_name(file: &str) -> bool {
    !file.is_empty() && !file.contains(['/', '\\']) && file != "." && file != ".."
}

// ── Writing ──────────────────────────────────────────────────────────────

/// Writes a schedule without themes or media as the bytes of a `.sws` file
/// (used for autosaves, which stay small).
pub fn write_schedule(schedule: &Schedule) -> Result<Vec<u8>, ScheduleFileError> {
    let contents = ScheduleContents {
        schedule: schedule.clone(),
        ..ScheduleContents::default()
    };
    let cursor = write_schedule_to(Cursor::new(Vec::new()), &contents, Vec::new())?;
    Ok(cursor.into_inner())
}

/// Writes a `.sws` file to `out`, embedding `media`. Returns `out`.
pub fn write_schedule_to<W: Write + Seek>(
    out: W,
    contents: &ScheduleContents,
    media: Vec<MediaSource<'_>>,
) -> Result<W, ScheduleFileError> {
    let document = ScheduleDocument {
        format_version: version_needed(&contents.schedule, !media.is_empty()),
        generator: format!("SimpleWorship {}", env!("CARGO_PKG_VERSION")),
        schedule: contents.schedule.clone(),
        themes: contents.themes.clone(),
        media: contents.media.clone(),
    };
    // Pretty-printed, so a curious user (or a bug report) can read it.
    let json = serde_json::to_vec_pretty(&document)?;

    let mut archive = ZipWriter::new(out);
    let compressed = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    archive
        .start_file(SCHEDULE_ENTRY, compressed)
        .map_err(ScheduleFileError::io)?;
    archive.write_all(&json).map_err(ScheduleFileError::io)?;

    // Large files need the 64-bit zip extension; images and videos are
    // already compressed, so they are stored as they are.
    let stored = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .large_file(true);
    for mut source in media {
        if !is_plain_file_name(&source.file) {
            continue;
        }
        archive
            .start_file(format!("{MEDIA_FOLDER}{}", source.file), stored)
            .map_err(ScheduleFileError::io)?;
        std::io::copy(&mut source.data, &mut archive).map_err(ScheduleFileError::io)?;
    }

    archive.finish().map_err(ScheduleFileError::io)
}

/// The lowest format version that can hold `schedule` (see the table in
/// the [module docs](self)).
fn version_needed(schedule: &Schedule, has_media_files: bool) -> u32 {
    let has =
        |wanted: fn(&ItemContent) -> bool| schedule.items.iter().any(|item| wanted(&item.content));

    if has(|content| {
        matches!(
            content,
            ItemContent::Countdown { .. }
                | ItemContent::Slides { .. }
                | ItemContent::Playlist { .. }
        )
    }) {
        4
    } else if has_media_files || has(|content| matches!(content, ItemContent::Media { .. })) {
        3
    } else if has(|content| matches!(content, ItemContent::Scripture { .. })) {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sw_core::{ScheduleItem, ScriptureOptions, Song};

    fn sample_schedule() -> Schedule {
        let mut song = Song::new("Amazing Grace");
        song.lyrics = "[Verse 1]\nAmazing grace".into();

        let mut welcome = ScheduleItem::new(ItemContent::Text {
            title: "Welcome".into(),
            text: "Welcome to church!".into(),
        });
        welcome.notes = "Before the countdown".into();

        Schedule {
            items: vec![welcome, ScheduleItem::new(ItemContent::Song { song })],
        }
    }

    /// Builds a zip with a single `schedule.json` containing `json`.
    fn archive_with(json: &str) -> Vec<u8> {
        let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
        archive
            .start_file(SCHEDULE_ENTRY, SimpleFileOptions::default())
            .unwrap();
        archive.write_all(json.as_bytes()).unwrap();
        archive.finish().unwrap().into_inner()
    }

    #[test]
    fn round_trips() {
        let schedule = sample_schedule();
        let bytes = write_schedule(&schedule).unwrap();
        assert_eq!(read_schedule(&bytes).unwrap(), schedule);
    }

    /// Reads the `formatVersion` back out of written bytes.
    fn written_version(bytes: &[u8]) -> u64 {
        let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut json = String::new();
        archive
            .by_name(SCHEDULE_ENTRY)
            .unwrap()
            .read_to_string(&mut json)
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        value["formatVersion"].as_u64().unwrap()
    }

    #[test]
    fn writes_the_lowest_version_that_fits() {
        let mut schedule = sample_schedule();
        assert_eq!(written_version(&write_schedule(&schedule).unwrap()), 1);

        let passage = sw_bible::reference::parse("jn 3:16").unwrap().passage;
        schedule
            .items
            .push(ScheduleItem::new(ItemContent::Scripture {
                reference: passage.to_string(),
                passage,
                translations: Vec::new(),
                options: ScriptureOptions::default(),
            }));
        let bytes = write_schedule(&schedule).unwrap();
        assert_eq!(written_version(&bytes), 2);
        assert_eq!(read_schedule(&bytes).unwrap(), schedule);
    }

    #[test]
    fn refuses_files_from_a_newer_version() {
        let bytes = archive_with(r#"{"formatVersion": 99, "something": "new"}"#);
        assert!(matches!(
            read_schedule(&bytes),
            Err(ScheduleFileError::NewerVersion { found: 99, .. })
        ));
    }

    #[test]
    fn ignores_unknown_fields() {
        let bytes = archive_with(
            r#"{"formatVersion": 1, "generator": "x", "futureField": true, "schedule": {"items": []}}"#,
        );
        assert_eq!(read_schedule(&bytes).unwrap(), Schedule::default());
    }

    #[test]
    fn reports_files_that_are_not_schedules() {
        assert!(matches!(
            read_schedule(b"definitely not a zip"),
            Err(ScheduleFileError::NotASchedule)
        ));

        let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
        archive
            .start_file("other.txt", SimpleFileOptions::default())
            .unwrap();
        let other_zip = archive.finish().unwrap().into_inner();
        assert!(matches!(
            read_schedule(&other_zip),
            Err(ScheduleFileError::NotASchedule)
        ));
    }

    #[test]
    fn reports_damaged_json() {
        let bytes = archive_with(r#"{"formatVersion": 1, "schedule": {"items": "oops"}}"#);
        assert!(matches!(
            read_schedule(&bytes),
            Err(ScheduleFileError::Damaged(_))
        ));
    }

    #[test]
    fn embeds_media_and_themes() {
        let contents = ScheduleContents {
            schedule: sample_schedule(),
            themes: sw_core::themes::starter_library().themes,
            media: Vec::new(),
        };
        let video_bytes = vec![7u8; 100_000];
        let media = vec![MediaSource {
            file: "clip.mp4".into(),
            data: Box::new(Cursor::new(video_bytes.clone())),
        }];
        let written = write_schedule_to(Cursor::new(Vec::new()), &contents, media)
            .unwrap()
            .into_inner();
        assert_eq!(written_version(&written), 3, "embedded media needs v3");

        let mut reader = ScheduleReader::new(Cursor::new(written)).unwrap();
        assert_eq!(reader.contents().unwrap(), contents);
        assert_eq!(reader.media_files(), ["clip.mp4"]);

        let mut copy = Vec::new();
        assert_eq!(reader.copy_media("clip.mp4", &mut copy).unwrap(), 100_000);
        assert_eq!(copy, video_bytes);
    }

    #[test]
    fn media_names_cannot_escape_the_media_folder() {
        let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
        archive
            .start_file(SCHEDULE_ENTRY, SimpleFileOptions::default())
            .unwrap();
        archive
            .write_all(br#"{"formatVersion": 3, "generator": "x", "schedule": {"items": []}}"#)
            .unwrap();
        archive
            .start_file("media/../../evil.sh", SimpleFileOptions::default())
            .unwrap();
        let bytes = archive.finish().unwrap().into_inner();

        let mut reader = ScheduleReader::new(Cursor::new(bytes)).unwrap();
        assert!(reader.media_files().is_empty());
        assert!(reader.copy_media("../../evil.sh", &mut Vec::new()).is_err());
    }
}

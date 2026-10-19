//! The `.sws` schedule file.
//!
//! A `.sws` file is a zip archive, so it can grow to carry images, videos
//! and fonts alongside the schedule (planned with the media library). Today
//! it contains a single entry:
//!
//! - `schedule.json`: `{ "formatVersion", "generator", "schedule" }`. The
//!   schedule uses the same JSON shape as the engine's IPC messages.
//!
//! ## Versioning
//!
//! `formatVersion` goes up whenever a change would confuse an older
//! SimpleWorship. Opening a file with a *newer* version is refused with a
//! clear message instead of silently losing data. Unknown fields are
//! ignored, so adding optional fields does not need a new version.

use std::io::{Cursor, Read, Write};

use serde::{Deserialize, Serialize};
use sw_core::Schedule;
use thiserror::Error;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// The format version this build writes and the newest it can read.
pub const FORMAT_VERSION: u32 = 1;

/// Name of the schedule entry inside the archive.
const SCHEDULE_ENTRY: &str = "schedule.json";

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

    /// Writing the archive failed.
    #[error("could not create the schedule file: {0}")]
    Write(String),
}

/// Reads a schedule from the bytes of a `.sws` file.
pub fn read_schedule(bytes: &[u8]) -> Result<Schedule, ScheduleFileError> {
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|_| ScheduleFileError::NotASchedule)?;
    let mut entry = archive
        .by_name(SCHEDULE_ENTRY)
        .map_err(|_| ScheduleFileError::NotASchedule)?;

    let mut json = Vec::new();
    entry
        .read_to_end(&mut json)
        .map_err(|_| ScheduleFileError::NotASchedule)?;

    // Check the version before parsing the rest, so a newer file gives the
    // "please update" message rather than a confusing parse error.
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
    Ok(document.schedule)
}

/// Writes a schedule as the bytes of a `.sws` file.
pub fn write_schedule(schedule: &Schedule) -> Result<Vec<u8>, ScheduleFileError> {
    let document = ScheduleDocument {
        format_version: FORMAT_VERSION,
        generator: format!("SimpleWorship {}", env!("CARGO_PKG_VERSION")),
        schedule: schedule.clone(),
    };
    // Pretty-printed, so a curious user (or a bug report) can read it.
    let json = serde_json::to_vec_pretty(&document)?;

    let write_error = |error: &dyn std::fmt::Display| ScheduleFileError::Write(error.to_string());
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    archive
        .start_file(SCHEDULE_ENTRY, options)
        .map_err(|e| write_error(&e))?;
    archive.write_all(&json).map_err(|e| write_error(&e))?;
    let cursor = archive.finish().map_err(|e| write_error(&e))?;

    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sw_core::{ItemContent, ScheduleItem, Song};

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
}

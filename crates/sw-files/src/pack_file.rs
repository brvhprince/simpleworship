//! The `.swpack` file: themes and lower thirds to share with another church
//! or computer (and, later, the format of the online library).
//!
//! A `.swpack` file is a zip archive containing:
//!
//! - `pack.json`: `{ "formatVersion", "generator", "themes", "lowerThirds",
//!   "media" }`, in the same JSON shapes as the rest of the app. `media`
//!   describes the embedded files.
//! - `media/<file>`: the pictures and videos the themes' backgrounds use,
//!   stored without recompression and streamed, never loaded whole.
//!
//! Fonts are not included (most are licensed per computer): the app says
//! which ones a pack uses that this computer lacks.
//!
//! Versioning works as for schedules (see [`crate::schedule_file`]): a file
//! with a newer `formatVersion` is refused with a "please update" message,
//! and unknown fields are ignored, so new optional fields need no new
//! version.
//!
//! | Version | Added                                  |
//! | ------- | -------------------------------------- |
//! | 1       | themes, lower thirds, background media |

use std::io::{Read, Seek, Write};

use serde::{Deserialize, Serialize};
use sw_core::lower_thirds::LowerThird;
use sw_core::{MediaItem, ThemeRecord};
use thiserror::Error;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::schedule_file::is_plain_file_name;

/// The newest format version this build reads and writes.
pub const PACK_FORMAT_VERSION: u32 = 1;

/// Name of the description entry inside the archive.
const PACK_ENTRY: &str = "pack.json";

/// Folder of the media entries inside the archive.
const MEDIA_FOLDER: &str = "media/";

/// What a pack holds.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackContents {
    /// Themes, as in the library.
    #[serde(default)]
    pub themes: Vec<ThemeRecord>,
    /// Lower thirds, as in the Lower Thirds list.
    #[serde(default)]
    pub lower_thirds: Vec<LowerThird>,
    /// The media library records of the embedded files.
    #[serde(default)]
    pub media: Vec<MediaItem>,
}

/// The contents of `pack.json`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackDocument {
    /// See the [module docs](self) on versioning.
    format_version: u32,
    /// Which program wrote the file, e.g. "SimpleWorship 0.9.0". For
    /// troubleshooting only.
    #[serde(default)]
    generator: String,
    #[serde(flatten)]
    contents: PackContents,
}

/// A media file to embed: its file name in the media folder, and its data.
pub struct PackMedia<'a> {
    /// The file name, e.g. `"0192….jpg"`.
    pub file: String,
    /// Where to read the data from (usually the open file).
    pub data: Box<dyn Read + 'a>,
}

/// Why a pack could not be read or written.
#[derive(Debug, Error)]
pub enum PackFileError {
    /// The bytes are not a zip archive, or the archive has no `pack.json`.
    #[error("this is not a SimpleWorship theme pack")]
    NotAPack,

    /// The file was made by a newer SimpleWorship.
    #[error(
        "this pack was made with a newer version of SimpleWorship \
         (format {found}, this version reads up to {supported}); please update"
    )]
    NewerVersion {
        /// The file's format version.
        found: u32,
        /// The newest version this build reads.
        supported: u32,
    },

    /// The description is damaged or not what we expect.
    #[error("the pack is damaged: {0}")]
    Damaged(#[from] serde_json::Error),

    /// Reading or writing the archive failed.
    #[error("could not read or write the pack: {0}")]
    Io(String),
}

impl PackFileError {
    fn io(error: impl std::fmt::Display) -> Self {
        Self::Io(error.to_string())
    }
}

/// An open `.swpack` file, read piece by piece.
pub struct PackReader<R: Read + Seek> {
    archive: ZipArchive<R>,
}

impl<R: Read + Seek> PackReader<R> {
    /// Opens a `.swpack` file.
    pub fn new(reader: R) -> Result<Self, PackFileError> {
        let archive = ZipArchive::new(reader).map_err(|_| PackFileError::NotAPack)?;
        Ok(Self { archive })
    }

    /// Reads `pack.json`.
    pub fn contents(&mut self) -> Result<PackContents, PackFileError> {
        let mut entry = self
            .archive
            .by_name(PACK_ENTRY)
            .map_err(|_| PackFileError::NotAPack)?;
        let mut json = Vec::new();
        entry
            .read_to_end(&mut json)
            .map_err(|_| PackFileError::NotAPack)?;

        // The version first, so a newer file says "please update" rather
        // than failing to parse.
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct VersionOnly {
            format_version: u32,
        }
        let VersionOnly { format_version } = serde_json::from_slice(&json)?;
        if format_version > PACK_FORMAT_VERSION {
            return Err(PackFileError::NewerVersion {
                found: format_version,
                supported: PACK_FORMAT_VERSION,
            });
        }
        let document: PackDocument = serde_json::from_slice(&json)?;
        Ok(document.contents)
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
    pub fn copy_media(&mut self, file: &str, out: &mut impl Write) -> Result<u64, PackFileError> {
        if !is_plain_file_name(file) {
            return Err(PackFileError::NotAPack);
        }
        let mut entry = self
            .archive
            .by_name(&format!("{MEDIA_FOLDER}{file}"))
            .map_err(PackFileError::io)?;
        std::io::copy(&mut entry, out).map_err(PackFileError::io)
    }
}

/// Writes a `.swpack` file to `out`, embedding `media`. Returns `out`.
pub fn write_pack_to<W: Write + Seek>(
    out: W,
    contents: &PackContents,
    media: Vec<PackMedia<'_>>,
) -> Result<W, PackFileError> {
    let document = PackDocument {
        format_version: PACK_FORMAT_VERSION,
        generator: format!("SimpleWorship {}", env!("CARGO_PKG_VERSION")),
        contents: contents.clone(),
    };
    // Pretty-printed, so people sharing packs can read what is in one.
    let json = serde_json::to_vec_pretty(&document)?;

    let mut archive = ZipWriter::new(out);
    let compressed = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    archive
        .start_file(PACK_ENTRY, compressed)
        .map_err(PackFileError::io)?;
    archive.write_all(&json).map_err(PackFileError::io)?;

    // Pictures and videos are already compressed: stored as they are.
    let stored = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .large_file(true);
    for mut source in media {
        if !is_plain_file_name(&source.file) {
            continue;
        }
        archive
            .start_file(format!("{MEDIA_FOLDER}{}", source.file), stored)
            .map_err(PackFileError::io)?;
        std::io::copy(&mut source.data, &mut archive).map_err(PackFileError::io)?;
    }
    archive.finish().map_err(PackFileError::io)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use sw_core::lower_thirds::LowerThirdKind;
    use sw_core::{Id, Theme};

    fn sample() -> PackContents {
        PackContents {
            themes: vec![ThemeRecord {
                id: Id::new(),
                theme: Theme {
                    name: "Sunday Blue".into(),
                    ..Theme::default()
                },
                builtin: false,
            }],
            lower_thirds: vec![LowerThird::new(
                LowerThirdKind::Offering,
                "MoMo: 024 123 4567",
                "Account name: Grace Chapel",
            )],
            media: Vec::new(),
        }
    }

    fn archive_with(json: &str) -> Vec<u8> {
        let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
        archive
            .start_file(PACK_ENTRY, SimpleFileOptions::default())
            .unwrap();
        archive.write_all(json.as_bytes()).unwrap();
        archive.finish().unwrap().into_inner()
    }

    #[test]
    fn round_trips_with_media() {
        let contents = sample();
        let picture: &[u8] = b"not really a jpeg";
        let media = vec![PackMedia {
            file: "0192.jpg".into(),
            data: Box::new(picture),
        }];
        let bytes = write_pack_to(Cursor::new(Vec::new()), &contents, media)
            .unwrap()
            .into_inner();

        let mut reader = PackReader::new(Cursor::new(bytes)).unwrap();
        assert_eq!(reader.contents().unwrap(), contents);
        assert_eq!(reader.media_files(), vec!["0192.jpg".to_owned()]);
        let mut copied = Vec::new();
        reader.copy_media("0192.jpg", &mut copied).unwrap();
        assert_eq!(copied, picture);
    }

    #[test]
    fn refuses_a_newer_pack_and_other_files() {
        let newer = archive_with(r#"{"formatVersion": 99, "themes": "something new"}"#);
        let error = PackReader::new(Cursor::new(newer))
            .unwrap()
            .contents()
            .unwrap_err();
        assert!(matches!(
            error,
            PackFileError::NewerVersion { found: 99, .. }
        ));

        assert!(matches!(
            PackReader::new(Cursor::new(b"hello".to_vec())),
            Err(PackFileError::NotAPack)
        ));
        // A zip without pack.json, such as a schedule file.
        let mut renamed = ZipWriter::new(Cursor::new(Vec::new()));
        renamed
            .start_file("schedule.json", SimpleFileOptions::default())
            .unwrap();
        let not_a_pack = renamed.finish().unwrap().into_inner();
        assert!(matches!(
            PackReader::new(Cursor::new(not_a_pack)).unwrap().contents(),
            Err(PackFileError::NotAPack)
        ));
    }

    #[test]
    fn never_reads_media_outside_its_folder() {
        let mut reader =
            PackReader::new(Cursor::new(archive_with(r#"{"formatVersion": 1}"#))).unwrap();
        assert!(matches!(
            reader.copy_media("../secrets.txt", &mut Vec::new()),
            Err(PackFileError::NotAPack)
        ));
    }

    #[test]
    fn a_pack_with_only_lower_thirds_loads() {
        let json = r#"{"formatVersion": 1, "lowerThirds": [{"id": "0192f0e4-0000-7000-8000-000000000001", "title": "@grace"}]}"#;
        let contents = PackReader::new(Cursor::new(archive_with(json)))
            .unwrap()
            .contents()
            .unwrap();
        assert!(contents.themes.is_empty());
        assert_eq!(contents.lower_thirds[0].title, "@grace");
    }
}

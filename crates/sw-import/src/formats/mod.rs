//! One module per source format. Each exposes a `read…` function that
//! returns an [`ImportReport`](crate::ImportReport).

pub mod chordpro;
pub mod easyworship;
pub mod easyworship_media;
pub mod openlp;
pub mod openlyrics;
pub mod plain;
pub mod propresenter;
pub mod songselect;

use std::path::Path;

use crate::ImportReport;

/// Song file extensions [`read_file`] understands (lowercase, no dot).
pub const FILE_EXTENSIONS: &[&str] = &[
    "xml", "pro6", "cho", "chordpro", "chopro", "crd", "usr", "txt",
];

/// Reads one song file, choosing the format from its extension and, for
/// `.xml` and `.txt`, its contents. Libraries (EasyWorship, OpenLP) are read
/// with their own functions instead.
pub fn read_file(path: &Path) -> ImportReport {
    let source = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    let mut report = ImportReport::default();

    let text = match std::fs::read(path) {
        Ok(bytes) => decode(&bytes),
        Err(error) => {
            report.problem(source, format!("could not open the file: {error}"));
            return report;
        }
    };
    let extension = path
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "pro6" => propresenter::read(&source, &text),
        "cho" | "chordpro" | "chopro" | "crd" => chordpro::read(&source, &text),
        "usr" => songselect::read_usr(&source, &text),
        "xml" if openlyrics::is_openlyrics(&text) => openlyrics::read(&source, &text),
        "txt" if songselect::is_songselect_text(&text) => songselect::read_text(&source, &text),
        "txt" => plain::read(&source, &text),
        _ => {
            report.problem(source, "this is not a song file SimpleWorship can read");
            report
        }
    }
}

/// Whether the file at `path` is worth offering as a song: a known song
/// file type, small (songs are a few kilobytes), and, for plain text, not
/// verse-numbered text such as a Bible book.
pub fn looks_like_song_file(path: &Path) -> bool {
    /// Song files are small; a larger file is something else.
    const MAX_BYTES: u64 = 64 * 1024;

    let extension = path
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if !FILE_EXTENSIONS.contains(&extension.as_str()) {
        return false;
    }
    if !std::fs::metadata(path).is_ok_and(|meta| meta.len() <= MAX_BYTES) {
        return false;
    }
    if extension == "txt" {
        let text = std::fs::read(path)
            .map(|bytes| decode(&bytes))
            .unwrap_or_default();
        return songselect::is_songselect_text(&text) || plain::looks_like_song(&text);
    }
    true
}

/// Decodes text as UTF-8 (or UTF-16 with a byte-order mark, which
/// SongSelect `.usr` files sometimes use), dropping any byte-order mark.
fn decode(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let units: Vec<u16> = rest
            .chunks_exact(2)
            .map(|p| u16::from_le_bytes([p[0], p[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let units: Vec<u16> = rest
            .chunks_exact(2)
            .map(|p| u16::from_be_bytes([p[0], p[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    String::from_utf8_lossy(bytes).into_owned()
}

//! Plain text files: the file name is the title, blank lines separate
//! slides, and lines such as "Verse 1" or "Chorus" start sections.

use super::openlyrics::push_if_complete;
use crate::lyrics::tag_sections;
use crate::{ImportReport, ImportedSong};

/// Whether `text` could be a song. A file where many lines start with a
/// number ("1 In the beginning…") is verse-numbered text, such as a Bible
/// book, not lyrics.
pub fn looks_like_song(text: &str) -> bool {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let numbered = lines
        .iter()
        .filter(|line| line.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .count();
    // Lyrics rarely start a line with a number ("10,000 reasons" is an
    // exception); in Bible text most lines do.
    lines.is_empty() || numbered * 2 < lines.len()
}

/// Reads one plain text song.
pub fn read(source: &str, text: &str) -> ImportReport {
    let mut report = ImportReport::default();
    if !looks_like_song(text) {
        report.problem(
            source,
            "this looks like numbered text (a Bible book?), not a song",
        );
        return report;
    }
    let name = source.rsplit(['/', '\\']).next().unwrap_or(source);
    let title = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    push_if_complete(
        &mut report,
        ImportedSong {
            title: title.trim().to_owned(),
            lyrics: tag_sections(text),
            source: source.to_owned(),
            ..ImportedSong::default()
        },
    );
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tells_lyrics_from_numbered_text() {
        assert!(looks_like_song(
            "Verse 1\nAmazing grace\nhow sweet the sound"
        ));
        assert!(looks_like_song(
            "10,000 reasons for my heart to find\nBless the Lord\nO my soul\nworship"
        ));
        assert!(!looks_like_song(
            "Yohane 1\n1 Ansa na wɔrebɛbɔ wiase\n6 Onyankopɔn somaa\n10 Saa bere no"
        ));
    }
}

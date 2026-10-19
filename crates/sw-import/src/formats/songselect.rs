//! CCLI SongSelect downloads: the `.usr` file and the text (`.txt`) file.
//!
//! `.usr` is an INI-style file. Section names and their words are in two
//! parallel lists, separated by `/t`; lines inside a section by `/n`:
//!
//! ```text
//! [S A22025]
//! Title=Amazing Grace
//! Author=John Newton
//! Copyright=Public Domain
//! Fields=Verse 1/tVerse 2
//! Words=Amazing grace/nhow sweet the sound/tTwas grace/n…
//! ```
//!
//! The text download is the lyrics with a footer:
//!
//! ```text
//! Amazing Grace
//!
//! Verse 1
//! Amazing grace …
//!
//! CCLI Song # 22025
//! John Newton
//! © Public Domain
//! For use solely with the SongSelect® Terms of Use. …
//! CCLI License # 12345
//! ```

use super::openlyrics::push_if_complete;
use crate::lyrics::tag_sections;
use crate::{ImportReport, ImportedSong};

/// Whether `text` is a SongSelect text download (it has the CCLI footer).
pub fn is_songselect_text(text: &str) -> bool {
    text.lines()
        .any(|line| line.trim_start().starts_with("CCLI Song #"))
}

/// Reads a `.usr` file.
pub fn read_usr(source: &str, text: &str) -> ImportReport {
    let mut report = ImportReport::default();
    let mut song = ImportedSong {
        source: source.to_owned(),
        ..ImportedSong::default()
    };
    let mut fields: Vec<String> = Vec::new();
    let mut words: Vec<String> = Vec::new();

    for line in text.lines().map(str::trim) {
        if let Some(section) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            // "[S A22025]": the CCLI number is the digits.
            let digits: String = section.chars().filter(char::is_ascii_digit).collect();
            if section.starts_with('S') && !digits.is_empty() {
                song.ccli_number = digits;
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "Title" => song.title = value.trim().to_owned(),
            "Author" => {
                song.author = value
                    .split('|')
                    .map(str::trim)
                    .collect::<Vec<_>>()
                    .join(", ")
            }
            "Copyright" => {
                song.copyright = value
                    .split('|')
                    .map(str::trim)
                    .collect::<Vec<_>>()
                    .join(", ")
            }
            "Fields" => fields = value.split("/t").map(str::to_owned).collect(),
            "Words" => words = value.split("/t").map(str::to_owned).collect(),
            _ => {}
        }
    }

    let sections: Vec<String> = fields
        .iter()
        .zip(&words)
        .map(|(name, text)| format!("[{}]\n{}", name.trim(), text.replace("/n", "\n").trim()))
        .collect();
    song.lyrics = sections.join("\n\n");
    push_if_complete(&mut report, song);
    report
}

/// Reads a SongSelect text download.
pub fn read_text(source: &str, text: &str) -> ImportReport {
    let mut report = ImportReport::default();
    let mut song = ImportedSong {
        source: source.to_owned(),
        ..ImportedSong::default()
    };

    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let footer = lines
        .iter()
        .position(|line| line.starts_with("CCLI Song #"))
        .unwrap_or(lines.len());
    let mut body = lines[..footer].iter().skip_while(|line| line.is_empty());

    song.title = body
        .next()
        .map(|title| (*title).to_owned())
        .unwrap_or_default();
    song.lyrics = tag_sections(&body.copied().collect::<Vec<_>>().join("\n"));

    let footer_lines = &lines[footer.min(lines.len())..];
    if let Some(first) = footer_lines.first() {
        song.ccli_number = first.chars().filter(char::is_ascii_digit).collect();
    }
    for line in footer_lines.iter().skip(1).filter(|line| !line.is_empty()) {
        if line.starts_with('©') || line.to_lowercase().starts_with("copyright") {
            song.copyright = (*line).to_owned();
        } else if line.starts_with("For use solely") || line.starts_with("CCLI License") {
            break;
        } else if song.author.is_empty() {
            song.author = line
                .split('|')
                .map(str::trim)
                .collect::<Vec<_>>()
                .join(", ");
        }
    }

    push_if_complete(&mut report, song);
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_usr_file() {
        let text = "[File]\nType=SongSelect Import File\n[S A22025]\nTitle=Amazing Grace\n\
            Author=John Newton | Edwin Excell\nCopyright=Public Domain\nFields=Verse 1/tVerse 2\n\
            Words=Amazing grace/nhow sweet the sound/tTwas grace/nthat taught\n";
        let song = &read_usr("grace.usr", text).songs[0];
        assert_eq!(song.ccli_number, "22025");
        assert_eq!(song.author, "John Newton, Edwin Excell");
        assert_eq!(
            song.lyrics,
            "[Verse 1]\nAmazing grace\nhow sweet the sound\n\n[Verse 2]\nTwas grace\nthat taught"
        );
    }

    #[test]
    fn reads_a_text_download() {
        let text = "Amazing Grace\n\nVerse 1\nAmazing grace\nhow sweet the sound\n\nVerse 2\nTwas grace\n\n\
            CCLI Song # 22025\nJohn Newton\n© Words: Public Domain\n\
            For use solely with the SongSelect® Terms of Use. All rights reserved. www.ccli.com\n\
            CCLI License # 12345\n";
        assert!(is_songselect_text(text));
        let song = &read_text("grace.txt", text).songs[0];
        assert_eq!(song.title, "Amazing Grace");
        assert_eq!(song.ccli_number, "22025");
        assert_eq!(song.author, "John Newton");
        assert_eq!(song.copyright, "© Words: Public Domain");
        assert_eq!(
            song.lyrics,
            "[Verse 1]\nAmazing grace\nhow sweet the sound\n\n[Verse 2]\nTwas grace"
        );
    }
}

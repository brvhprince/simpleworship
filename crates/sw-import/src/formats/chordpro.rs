//! ChordPro (`.cho`, `.chordpro`, `.chopro`, `.crd`).
//!
//! ```text
//! {title: Amazing Grace}
//! {artist: John Newton}
//! {ccli: 22025}
//! {start_of_verse: Verse 1}
//! [G]Amazing [C]grace how [G]sweet the sound
//! {end_of_verse}
//! {soc}
//! …
//! {eoc}
//! ```
//!
//! Directives give the song details and the sections; chords in square
//! brackets are removed (SimpleWorship does not show chords yet).

use sw_core::lyrics::SectionLabel;

use super::openlyrics::push_if_complete;
use crate::{ImportReport, ImportedSong};

/// Reads one ChordPro file.
pub fn read(source: &str, text: &str) -> ImportReport {
    let mut report = ImportReport::default();
    let mut song = ImportedSong {
        source: source.to_owned(),
        ..ImportedSong::default()
    };
    let mut authors: Vec<String> = Vec::new();
    let mut lines: Vec<String> = Vec::new();
    // Counts unnamed verses, so `{sov}` blocks become Verse 1, Verse 2, …
    let mut verses = 0;

    for line in text.lines().map(str::trim) {
        if line.starts_with('#') {
            continue;
        }
        if let Some((name, value)) = directive(line) {
            match name.as_str() {
                "title" | "t" => song.title = value.to_owned(),
                "artist" | "composer" | "lyricist" | "author" => authors.push(value.to_owned()),
                "copyright" => song.copyright = value.to_owned(),
                "ccli" => song.ccli_number = value.to_owned(),
                "start_of_verse" | "sov" => {
                    verses += 1;
                    let name = named_or(value, &format!("Verse {verses}"));
                    heading(&mut lines, &name);
                }
                "start_of_chorus" | "soc" => heading(&mut lines, &named_or(value, "Chorus")),
                "start_of_bridge" | "sob" => heading(&mut lines, &named_or(value, "Bridge")),
                "comment" | "c" | "comment_italic" | "ci" | "comment_box" | "cb" => {
                    // A comment that names a section ("{c: Verse 2}") starts one.
                    if let Some(label) = SectionLabel::parse_heading(value) {
                        heading(&mut lines, &label.name);
                    }
                }
                _ => {}
            }
            continue;
        }
        let words = strip_chords(line);
        if words.is_empty() {
            if lines
                .last()
                .is_some_and(|last| !last.is_empty() && !last.starts_with('['))
            {
                lines.push(String::new());
            }
        } else {
            lines.push(words);
        }
    }

    song.author = authors.join(", ");
    song.lyrics = lines.join("\n").trim().to_owned();
    push_if_complete(&mut report, song);
    report
}

/// `{name: value}` or `{name}` → (lowercase name, value).
fn directive(line: &str) -> Option<(String, &str)> {
    let inner = line.strip_prefix('{')?.strip_suffix('}')?;
    let (name, value) = inner.split_once([':', ' ']).unwrap_or((inner, ""));
    Some((name.trim().to_lowercase(), value.trim()))
}

fn named_or(value: &str, fallback: &str) -> String {
    if value.is_empty() {
        fallback.to_owned()
    } else {
        value.to_owned()
    }
}

/// Starts a section: a blank line (if needed) and a `[Name]` tag.
fn heading(lines: &mut Vec<String>, name: &str) {
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    if !lines.is_empty() {
        lines.push(String::new());
    }
    lines.push(format!("[{name}]"));
}

/// Removes `[G]`-style chords and tidies the spaces they leave.
fn strip_chords(line: &str) -> String {
    let mut words = String::with_capacity(line.len());
    let mut in_chord = false;
    for c in line.chars() {
        match c {
            '[' => in_chord = true,
            ']' if in_chord => in_chord = false,
            _ if !in_chord => words.push(c),
            _ => {}
        }
    }
    words.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_directives_sections_and_strips_chords() {
        let text = "# A comment\n{title: Amazing Grace}\n{artist: John Newton}\n{ccli: 22025}\n\
            {start_of_verse}\n[G]Amazing [C]grace how [G]sweet the sound\nThat [D]saved a wretch\n\n\
            I once was lost\n{end_of_verse}\n{soc}\nMy chains are gone\n{eoc}\n{c: Verse 2}\nTwas grace\n";
        let report = read("grace.cho", text);
        let song = &report.songs[0];
        assert_eq!(song.title, "Amazing Grace");
        assert_eq!(song.author, "John Newton");
        assert_eq!(
            song.lyrics,
            "[Verse 1]\nAmazing grace how sweet the sound\nThat saved a wretch\n\nI once was lost\n\n[Chorus]\nMy chains are gone\n\n[Verse 2]\nTwas grace"
        );
    }
}

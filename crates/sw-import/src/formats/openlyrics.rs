//! OpenLyrics XML (`.xml`), the open song format, also exported by OpenLP.
//!
//! ```xml
//! <song xmlns="http://openlyrics.info/namespace/2009/song">
//!   <properties>
//!     <titles><title>Amazing Grace</title></titles>
//!     <authors><author>John Newton</author></authors>
//!     <copyright>Public Domain</copyright>
//!     <ccliNo>22025</ccliNo>
//!     <verseOrder>v1 v2 c</verseOrder>
//!   </properties>
//!   <lyrics>
//!     <verse name="v1"><lines>Amazing grace<br/>how sweet</lines></verse>
//!   </lyrics>
//! </song>
//! ```
//!
//! Each `<lines>` block is a slide; `<br/>` is a line break. Chords and
//! comments inside the lyrics are dropped.

use crate::lyrics::{arrangement_from_codes, section_name_for_code};
use crate::xml::{XmlEvent, for_each_event};
use crate::{ImportReport, ImportedSong};

/// Elements whose text is not lyrics.
const SKIPPED: &[&str] = &["comment", "chord"];

/// Whether `text` looks like an OpenLyrics document.
pub fn is_openlyrics(text: &str) -> bool {
    let head: String = text.chars().take(2000).collect();
    head.contains("openlyrics.info") || (head.contains("<song") && head.contains("<lyrics"))
}

/// Reads one OpenLyrics document.
pub fn read(source: &str, xml: &str) -> ImportReport {
    let mut report = ImportReport::default();
    let mut song = ImportedSong {
        source: source.to_owned(),
        ..ImportedSong::default()
    };
    let mut authors: Vec<String> = Vec::new();
    let mut lyrics = String::new();
    let mut verse_order = String::new();
    let mut open: Vec<String> = Vec::new();

    let result = for_each_event(xml, |event| match event {
        XmlEvent::Start {
            name,
            attributes,
            empty,
        } => {
            match name.as_str() {
                "verse" => {
                    let code = attributes.get("name").map_or("v", String::as_str);
                    if !lyrics.is_empty() {
                        lyrics.push_str("\n\n");
                    }
                    lyrics.push_str(&format!("[{}]\n", section_name_for_code(code)));
                }
                "lines" if !lyrics.ends_with('\n') && !lyrics.is_empty() => lyrics.push_str("\n\n"),
                "br" => lyrics.push('\n'),
                _ => {}
            }
            if !empty {
                open.push(name);
            }
        }
        XmlEvent::End { .. } => {
            open.pop();
        }
        XmlEvent::Text(text) => {
            let inside = |tag: &str| open.last().is_some_and(|name| name == tag);
            if open.iter().any(|name| SKIPPED.contains(&name.as_str())) {
                return;
            }
            if inside("title") && song.title.is_empty() {
                song.title = text.trim().to_owned();
            } else if inside("author") {
                authors.push(text.trim().to_owned());
            } else if inside("copyright") {
                song.copyright = text.trim().to_owned();
            } else if inside("cclino") {
                song.ccli_number = text.trim().to_owned();
            } else if inside("verseorder") {
                verse_order = text;
            } else if open.iter().any(|name| name == "lines") {
                // Whitespace between tags is layout, not lyrics.
                lyrics.push_str(text.trim_matches(['\n', '\r', '\t']));
            }
        }
    });

    if let Err(error) = result {
        report.problem(source, format!("this OpenLyrics file is damaged: {error}"));
        return report;
    }
    song.author = authors.join(", ");
    song.lyrics = tidy_lines(&lyrics);
    song.arrangement = arrangement_from_codes(&verse_order);
    push_if_complete(&mut report, song);
    report
}

/// Trims every line and removes blank lines at the very start and end.
pub(crate) fn tidy_lines(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
        .trim_matches('\n')
        .to_owned()
}

/// Adds `song` to the report, or a problem if it has no title or lyrics.
pub(crate) fn push_if_complete(report: &mut ImportReport, song: ImportedSong) {
    if song.title.trim().is_empty() {
        report.problem(
            song.source.clone(),
            "the song has no title, so it was skipped",
        );
    } else if song.lyrics.trim().is_empty() {
        report.problem(
            song.source.clone(),
            "the song has no lyrics, so it was skipped",
        );
    } else {
        report.songs.push(song);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_an_openlyrics_song() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<song xmlns="http://openlyrics.info/namespace/2009/song" version="0.8">
  <properties>
    <titles><title>Amazing Grace</title><title lang="es">Sublime Gracia</title></titles>
    <authors><author>John Newton</author></authors>
    <copyright>Public Domain</copyright>
    <ccliNo>22025</ccliNo>
    <verseOrder>v1 v2 v1</verseOrder>
  </properties>
  <lyrics>
    <verse name="v1">
      <lines>Amazing grace how sweet the sound<br/>That saved a wretch like me</lines>
      <lines><comment>softly</comment>I once was lost</lines>
    </verse>
    <verse name="v2"><lines>&#8217;Twas <chord name="G"/>grace</lines></verse>
  </lyrics>
</song>"#;
        let report = read("grace.xml", xml);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        let song = &report.songs[0];
        assert_eq!(song.title, "Amazing Grace");
        assert_eq!(song.ccli_number, "22025");
        assert_eq!(song.arrangement, "V1 V2 V1");
        assert_eq!(
            song.lyrics,
            "[Verse 1]\nAmazing grace how sweet the sound\nThat saved a wretch like me\n\nI once was lost\n\n[Verse 2]\n\u{2019}Twas grace"
        );
    }
}

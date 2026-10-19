//! ProPresenter 6 documents (`.pro6`).
//!
//! A `.pro6` file is XML. Slides are grouped (`RVSlideGrouping name="Verse 1"`),
//! and each slide's text sits in a text element, base64-encoded: as
//! `PlainText` in newer files, as `RTFData` in all of them. Song details are
//! attributes on the root element (`CCLISongTitle`, `CCLIAuthor`,
//! `CCLICopyrightYear`, `CCLIPublisher`, `CCLISongNumber`).
//!
//! ProPresenter 7 (`.pro`) uses a binary format and is not supported.

use std::collections::HashMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use super::openlyrics::{push_if_complete, tidy_lines};
use crate::xml::{XmlEvent, for_each_event};
use crate::{ImportReport, ImportedSong, rtf};
use sw_core::lyrics::SectionLabel;

/// Reads one ProPresenter 6 document. `source` is its file name; the title
/// falls back to it when the document has none.
pub fn read(source: &str, xml: &str) -> ImportReport {
    let mut report = ImportReport::default();
    let mut song = ImportedSong {
        source: source.to_owned(),
        ..ImportedSong::default()
    };
    let mut lyrics = String::new();
    // The current slide's text, from PlainText (preferred) or RTFData.
    let mut plain: Option<String> = None;
    let mut rich: Option<String> = None;
    let mut field: Option<String> = None;
    let mut root_seen = false;

    let result = for_each_event(xml, |event| match event {
        XmlEvent::Start {
            name, attributes, ..
        } => match name.as_str() {
            "rvpresentationdocument" if !root_seen => {
                root_seen = true;
                read_song_details(&attributes, &mut song);
            }
            "rvslidegrouping" => {
                // Every group becomes a section, whatever its name ("Verse 1",
                // or a custom one such as "Vamp").
                let group = attributes.get("name").map_or("", |name| name.trim());
                if !lyrics.is_empty() {
                    lyrics.push_str("\n\n");
                }
                if !group.is_empty() {
                    lyrics.push_str(&format!("[{}]\n", section_name(group)));
                }
            }
            "rvdisplayslide" => {
                plain = None;
                rich = None;
            }
            "nsstring" => field = attributes.get("rvxmlivarname").cloned(),
            _ => {}
        },
        XmlEvent::End { name } => match name.as_str() {
            "rvdisplayslide" => {
                let text = plain
                    .take()
                    .or_else(|| rich.take().map(|r| rtf::to_text(&r)));
                if let Some(text) = text.filter(|t| !t.trim().is_empty()) {
                    lyrics.push_str(text.trim());
                    lyrics.push_str("\n\n");
                }
            }
            "nsstring" => field = None,
            _ => {}
        },
        XmlEvent::Text(text) => match field.as_deref() {
            Some("PlainText") => plain = decode_base64(&text),
            Some("RTFData") => rich = decode_base64(&text),
            _ => {}
        },
    });

    if let Err(error) = result {
        report.problem(
            source,
            format!("this ProPresenter file is damaged: {error}"),
        );
        return report;
    }
    if song.title.is_empty() {
        song.title = file_stem(source);
    }
    song.lyrics = collapse_blank_lines(&tidy_lines(&lyrics));
    push_if_complete(&mut report, song);
    report
}

fn read_song_details(attributes: &HashMap<String, String>, song: &mut ImportedSong) {
    let get = |key: &str| {
        attributes
            .get(key)
            .map(|v| v.trim().to_owned())
            .unwrap_or_default()
    };
    song.title = get("cclisongtitle");
    song.author = get("ccliauthor");
    // ProPresenter writes 0 for "no number".
    song.ccli_number = Some(get("cclisongnumber"))
        .filter(|n| n != "0")
        .unwrap_or_default();
    let year = get("cclicopyrightyear");
    let publisher = get("cclipublisher");
    song.copyright = match (year.is_empty() || year == "0", publisher.is_empty()) {
        (true, true) => String::new(),
        (true, false) => format!("© {publisher}"),
        (false, true) => format!("© {year}"),
        (false, false) => format!("© {year} {publisher}"),
    };
}

/// A group name as a section name: "VERSE 1" → "Verse 1"; names that are
/// not standard sections are kept as they are.
fn section_name(group: &str) -> String {
    SectionLabel::parse_heading(group).map_or_else(|| group.to_owned(), |label| label.name)
}

/// Turns runs of blank lines into a single one.
fn collapse_blank_lines(text: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();
    for line in text.lines() {
        if !(line.is_empty() && lines.last().is_some_and(|last| last.is_empty())) {
            lines.push(line);
        }
    }
    lines.join("\n")
}

fn decode_base64(text: &str) -> Option<String> {
    let cleaned: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = STANDARD.decode(cleaned).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn file_stem(source: &str) -> String {
    let name = source.rsplit(['/', '\\']).next().unwrap_or(source);
    name.rsplit_once('.')
        .map_or(name, |(stem, _)| stem)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b64(text: &str) -> String {
        STANDARD.encode(text)
    }

    #[test]
    fn reads_groups_slides_and_ccli_details() {
        let xml = format!(
            r#"<RVPresentationDocument CCLISongTitle="Amazing Grace" CCLIAuthor="John Newton"
                 CCLICopyrightYear="1779" CCLIPublisher="Public Domain" CCLISongNumber="22025">
  <array rvXMLIvarName="groups">
    <RVSlideGrouping name="Verse 1">
      <array rvXMLIvarName="slides">
        <RVDisplaySlide><array rvXMLIvarName="displayElements"><RVTextElement>
          <NSString rvXMLIvarName="PlainText">{}</NSString>
        </RVTextElement></array></RVDisplaySlide>
        <RVDisplaySlide><array rvXMLIvarName="displayElements"><RVTextElement>
          <NSString rvXMLIvarName="RTFData">{}</NSString>
        </RVTextElement></array></RVDisplaySlide>
      </array>
    </RVSlideGrouping>
    <RVSlideGrouping name="Chorus"><array rvXMLIvarName="slides">
      <RVDisplaySlide><array><RVTextElement>
        <NSString rvXMLIvarName="PlainText">{}</NSString>
      </RVTextElement></array></RVDisplaySlide>
    </array></RVSlideGrouping>
  </array>
</RVPresentationDocument>"#,
            b64("Amazing grace how sweet the sound\nThat saved a wretch like me"),
            b64(r"{\rtf1\ansi I once was lost\par but now am found}"),
            b64("My chains are gone"),
        );
        let report = read("Amazing Grace.pro6", &xml);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        let song = &report.songs[0];
        assert_eq!(song.copyright, "© 1779 Public Domain");
        assert_eq!(song.ccli_number, "22025");
        assert_eq!(
            song.lyrics,
            "[Verse 1]\nAmazing grace how sweet the sound\nThat saved a wretch like me\n\nI once was lost\nbut now am found\n\n[Chorus]\nMy chains are gone"
        );
    }
}

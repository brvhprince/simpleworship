//! Turning lyrics from other programs into SimpleWorship's tagged form.
//!
//! Most programs write section names as ordinary lines:
//!
//! ```text
//! Verse 1
//! Amazing grace how sweet the sound
//!
//! Chorus
//! …
//! ```
//!
//! [`tag_sections`] turns those heading lines into `[Verse 1]` tags, so the
//! song gets proper sections (and V1, C… jump shortcuts). Everything else is
//! kept as it is: blank lines still mean "new slide".

use sw_core::lyrics::SectionLabel;

/// Converts heading lines to `[Section]` tags and tidies blank lines: no
/// blank lines at the start or end, and no more than one in a row.
pub fn tag_sections(text: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in text.lines().map(str::trim_end) {
        match SectionLabel::parse_heading(line) {
            Some(label) => {
                // A new section starts a new slide anyway; drop the blank
                // line before the heading so sections stay compact.
                while lines.last().is_some_and(|last| last.is_empty()) {
                    lines.pop();
                }
                if !lines.is_empty() {
                    lines.push(String::new());
                }
                lines.push(format!("[{}]", label.name));
            }
            None if line.trim().is_empty() => {
                let after_heading = lines.last().is_some_and(|last| last.starts_with('['));
                if !lines.is_empty()
                    && !after_heading
                    && lines.last().is_some_and(|l| !l.is_empty())
                {
                    lines.push(String::new());
                }
            }
            None => lines.push(line.trim().to_owned()),
        }
    }
    while lines.last().is_some_and(|last| last.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

/// The section name for a short verse code used by OpenLyrics and OpenLP:
/// `"v1"` → "Verse 1", `"c"` → "Chorus", `"p2"` → "Pre-Chorus 2". An
/// unknown code becomes a plain name, e.g. `"o"` → "Other".
pub fn section_name_for_code(code: &str) -> String {
    let code = code.trim();
    // OpenLyrics allows a language or part suffix ("v1a"); keep the number.
    let digits_end = code
        .char_indices()
        .skip_while(|(_, c)| c.is_alphabetic())
        .take_while(|(_, c)| c.is_ascii_digit())
        .last()
        .map(|(i, c)| i + c.len_utf8());
    let core = digits_end.map_or(code, |end| &code[..end]);

    let label = SectionLabel::parse(core);
    if label.kind == sw_core::lyrics::SectionKind::Other {
        let number: String = core.chars().filter(char::is_ascii_digit).collect();
        if number.is_empty() {
            "Other".to_owned()
        } else {
            format!("Other {number}")
        }
    } else {
        label.name
    }
}

/// Converts an OpenLyrics / OpenLP verse order ("v1 c1 v2 c1") into a
/// SimpleWorship arrangement ("V1 C1 V2 C1"), using section shortcuts.
pub fn arrangement_from_codes(order: &str) -> String {
    order
        .split_whitespace()
        .filter_map(|code| SectionLabel::parse(&section_name_for_code(code)).shortcut())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turns_headings_into_tags() {
        let text = "Verse 1\nAmazing grace\nhow sweet\n\nChorus\nMy chains are gone\n\n\n";
        assert_eq!(
            tag_sections(text),
            "[Verse 1]\nAmazing grace\nhow sweet\n\n[Chorus]\nMy chains are gone"
        );
    }

    #[test]
    fn keeps_slide_breaks_inside_sections() {
        let text = "Verse 1\nfirst slide\n\n\nsecond slide";
        assert_eq!(tag_sections(text), "[Verse 1]\nfirst slide\n\nsecond slide");
    }

    #[test]
    fn names_verse_codes() {
        assert_eq!(section_name_for_code("v1"), "Verse 1");
        assert_eq!(section_name_for_code("c"), "Chorus");
        assert_eq!(section_name_for_code("p2"), "Pre-Chorus 2");
        assert_eq!(section_name_for_code("v1a"), "Verse 1");
        assert_eq!(section_name_for_code("o"), "Other");
        assert_eq!(arrangement_from_codes("v1 c v2 c b"), "V1 C V2 C B");
    }

    #[test]
    fn leaves_lyrics_without_headings_alone() {
        assert_eq!(tag_sections("\nline one\nline two\n"), "line one\nline two");
    }
}

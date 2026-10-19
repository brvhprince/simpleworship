//! Parsing of tagged song lyrics.
//!
//! Lyrics are stored as plain text, exactly as an operator types them in the
//! song editor. A line in square brackets starts a new section:
//!
//! ```text
//! [Verse 1]
//! Amazing grace, how sweet the sound
//! That saved a wretch like me
//!
//! [Chorus]
//! My chains are gone, I've been set free
//! ```
//!
//! Section tags are forgiving: `[Verse 1]`, `[verse1]`, `[V1]` and `[v 1]` all
//! mean the same thing. Anything we do not recognise (e.g. `[Spontaneous]`)
//! becomes a section of kind [`SectionKind::Other`] that keeps its name.
//!
//! An *arrangement* is an optional, space- or comma-separated list of section
//! shortcuts (`"V1 C V2 C B C"`) that controls the order sections are shown in.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The kind of a song section. Determines its display name and its keyboard
/// shortcut letter in the live view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum SectionKind {
    /// A verse (`V`).
    Verse,
    /// A pre-chorus (`P`).
    PreChorus,
    /// A chorus (`C`).
    Chorus,
    /// A bridge (`B`).
    Bridge,
    /// A tag, a short repeated line at the end (`T`).
    Tag,
    /// An intro (`I`).
    Intro,
    /// An ending or outro (`E`).
    Ending,
    /// Anything else. These sections have no shortcut.
    Other,
}

impl SectionKind {
    /// The single letter used to jump to this kind of section, if it has one.
    pub fn shortcut_letter(self) -> Option<char> {
        match self {
            SectionKind::Verse => Some('V'),
            SectionKind::PreChorus => Some('P'),
            SectionKind::Chorus => Some('C'),
            SectionKind::Bridge => Some('B'),
            SectionKind::Tag => Some('T'),
            SectionKind::Intro => Some('I'),
            SectionKind::Ending => Some('E'),
            SectionKind::Other => None,
        }
    }

    /// The English display name, e.g. `"Pre-Chorus"`.
    pub fn display_name(self) -> &'static str {
        match self {
            SectionKind::Verse => "Verse",
            SectionKind::PreChorus => "Pre-Chorus",
            SectionKind::Chorus => "Chorus",
            SectionKind::Bridge => "Bridge",
            SectionKind::Tag => "Tag",
            SectionKind::Intro => "Intro",
            SectionKind::Ending => "Ending",
            SectionKind::Other => "Other",
        }
    }

    /// Recognises a section name, ignoring case, spaces and hyphens.
    ///
    /// Accepts full names (`"chorus"`, `"pre-chorus"`), common synonyms
    /// (`"outro"`, `"refrain"`) and single-letter shortcuts (`"c"`).
    fn from_name(name: &str) -> Option<SectionKind> {
        let normalized: String = name
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect();

        let kind = match normalized.as_str() {
            "verse" | "v" => SectionKind::Verse,
            "prechorus" | "p" => SectionKind::PreChorus,
            "chorus" | "refrain" | "c" => SectionKind::Chorus,
            "bridge" | "b" => SectionKind::Bridge,
            "tag" | "t" => SectionKind::Tag,
            "intro" | "i" => SectionKind::Intro,
            "ending" | "outro" | "end" | "e" => SectionKind::Ending,
            _ => return None,
        };
        Some(kind)
    }
}

/// The label of a song section, e.g. "Verse 2" or "Chorus".
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct SectionLabel {
    /// What kind of section this is.
    pub kind: SectionKind,
    /// The section number, e.g. `2` for "Verse 2". `None` for an unnumbered
    /// section such as a single "Chorus".
    pub number: Option<u32>,
    /// The name to display. For known kinds this is normalized ("Verse 2");
    /// for [`SectionKind::Other`] it is the tag text as the operator typed it.
    pub name: String,
}

impl SectionLabel {
    /// Builds the label for a known section kind and optional number.
    pub fn new(kind: SectionKind, number: Option<u32>) -> Self {
        let name = match number {
            Some(n) => format!("{} {n}", kind.display_name()),
            None => kind.display_name().to_owned(),
        };
        Self { kind, number, name }
    }

    /// Parses the text inside a section tag, e.g. `"Verse 1"`, `"v1"`, `"C"`.
    ///
    /// Never fails: unrecognised text becomes a [`SectionKind::Other`] label
    /// that keeps the original text as its name.
    pub fn parse(tag: &str) -> Self {
        let tag = tag.trim();
        match split_trailing_number(tag) {
            Some((kind, number)) => Self::new(kind, number),
            None => Self {
                kind: SectionKind::Other,
                number: None,
                name: tag.to_owned(),
            },
        }
    }

    /// Recognises a whole line that is only a section heading, as found in
    /// lyrics from other programs: `"Verse 1"`, `"CHORUS"`, `"Pre-Chorus 2:"`.
    ///
    /// Stricter than [`SectionLabel::parse`]: the name must be written out
    /// (at least three letters, so "I" or "C" in a lyric is never taken for
    /// a heading) and nothing else may be on the line.
    pub fn parse_heading(line: &str) -> Option<Self> {
        let line = line.trim().trim_end_matches(':').trim();
        if line.is_empty() || line.len() > 24 {
            return None;
        }
        let letters = line.chars().filter(|c| c.is_alphabetic()).count();
        let label = Self::parse(line);
        (letters >= 3 && label.kind != SectionKind::Other).then_some(label)
    }

    /// The keyboard shortcut for this section, e.g. `"V2"` or `"C"`.
    /// Returns `None` for [`SectionKind::Other`] sections.
    pub fn shortcut(&self) -> Option<String> {
        let letter = self.kind.shortcut_letter()?;
        Some(match self.number {
            Some(n) => format!("{letter}{n}"),
            None => letter.to_string(),
        })
    }

    /// Whether an arrangement token or typed shortcut refers to this section.
    ///
    /// Kinds must match. If the token has a number it must match exactly. A
    /// token without a number (`"C"`) matches any section of that kind, so it
    /// still works when the lyrics happen to say "Chorus 1".
    pub fn matches(&self, token: &SectionLabel) -> bool {
        if self.kind != token.kind || self.kind == SectionKind::Other {
            return false;
        }
        token.number.is_none() || token.number == self.number
    }
}

/// Splits `"Verse 12"` / `"v12"` / `"chorus"` into a known kind and an
/// optional number. Returns `None` when the name part is not a known kind.
fn split_trailing_number(tag: &str) -> Option<(SectionKind, Option<u32>)> {
    let digits_start = tag
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_ascii_digit())
        .last()
        .map(|(i, _)| i);

    let (name, number) = match digits_start {
        Some(i) => (&tag[..i], tag[i..].parse().ok()),
        None => (tag, None),
    };

    // Guard against a tag that is *only* a number, like "[2]".
    if name.trim().is_empty() {
        return None;
    }
    SectionKind::from_name(name).map(|kind| (kind, number))
}

/// One section of a song: a label and the lyric text below it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct Section {
    /// The section's label. `None` for text that appears before the first tag.
    pub label: Option<SectionLabel>,
    /// The lyric text, with leading and trailing blank lines removed. Blank
    /// lines *inside* the text are kept: they mark slide breaks.
    pub text: String,
}

/// Parses tagged lyrics into sections, in document order.
///
/// Sections that end up with no text are dropped, so a stray tag at the end
/// of the lyrics does not produce an empty slide.
pub fn parse_sections(lyrics: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut label: Option<SectionLabel> = None;
    let mut lines: Vec<&str> = Vec::new();

    for line in lyrics.lines() {
        if let Some(tag) = parse_tag_line(line) {
            push_section(&mut sections, label.take(), &lines);
            lines.clear();
            label = Some(SectionLabel::parse(tag));
        } else {
            lines.push(line.trim_end());
        }
    }
    push_section(&mut sections, label, &lines);

    sections
}

/// Returns the text inside `[...]` if the whole line is a section tag.
fn parse_tag_line(line: &str) -> Option<&str> {
    line.trim()
        .strip_prefix('[')?
        .strip_suffix(']')
        .filter(|inner| !inner.trim().is_empty())
}

/// Appends a section built from `lines`, unless it has no text.
fn push_section(sections: &mut Vec<Section>, label: Option<SectionLabel>, lines: &[&str]) {
    let text = lines.join("\n").trim_matches('\n').to_owned();
    if !text.trim().is_empty() {
        sections.push(Section { label, text });
    }
}

/// Parses an arrangement string such as `"V1 C V2 C B C"` into labels.
///
/// Tokens may be separated by spaces or commas. Unknown tokens are skipped.
pub fn parse_arrangement(arrangement: &str) -> Vec<SectionLabel> {
    arrangement
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|token| !token.is_empty())
        .map(SectionLabel::parse)
        .filter(|label| label.kind != SectionKind::Other)
        .collect()
}

/// Orders `sections` according to `arrangement`.
///
/// Each arrangement token picks the first section it [matches](SectionLabel::matches).
/// Tokens that match nothing are skipped. An empty arrangement, or one where
/// nothing matches, returns the sections in document order.
pub fn arrange(sections: &[Section], arrangement: &[SectionLabel]) -> Vec<Section> {
    let arranged: Vec<Section> = arrangement
        .iter()
        .filter_map(|token| find_section(sections, token))
        .cloned()
        .collect();

    if arranged.is_empty() {
        sections.to_vec()
    } else {
        arranged
    }
}

/// Finds the section a token refers to, preferring an exact number match
/// ("C" should find "Chorus" before "Chorus 2").
fn find_section<'a>(sections: &'a [Section], token: &SectionLabel) -> Option<&'a Section> {
    let labelled = || sections.iter().filter_map(|s| Some((s, s.label.as_ref()?)));

    labelled()
        .find(|(_, label)| label.kind == token.kind && label.number == token.number)
        .or_else(|| labelled().find(|(_, label)| label.matches(token)))
        .map(|(section, _)| section)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_labels_in_many_spellings() {
        for tag in ["Verse 1", "verse1", "V1", "v 1", " VERSE 1 "] {
            let label = SectionLabel::parse(tag);
            assert_eq!(label.kind, SectionKind::Verse, "tag {tag:?}");
            assert_eq!(label.number, Some(1), "tag {tag:?}");
            assert_eq!(label.name, "Verse 1");
            assert_eq!(label.shortcut().as_deref(), Some("V1"));
        }

        assert_eq!(
            SectionLabel::parse("Pre-Chorus").kind,
            SectionKind::PreChorus
        );
        assert_eq!(SectionLabel::parse("pre chorus 2").number, Some(2));
        assert_eq!(SectionLabel::parse("Outro").kind, SectionKind::Ending);
    }

    #[test]
    fn recognises_heading_lines_strictly() {
        let heading = |line| SectionLabel::parse_heading(line).map(|label| label.name);
        assert_eq!(heading("Verse 1").as_deref(), Some("Verse 1"));
        assert_eq!(heading("  CHORUS: ").as_deref(), Some("Chorus"));
        assert_eq!(heading("Pre-Chorus 2").as_deref(), Some("Pre-Chorus 2"));
        assert_eq!(heading("I"), None, "a lyric line, not an intro");
        assert_eq!(heading("C"), None);
        assert_eq!(heading("Amazing grace how sweet the sound"), None);
        assert_eq!(heading("Lord"), None);
    }

    #[test]
    fn unknown_tags_keep_their_name() {
        let label = SectionLabel::parse("Spontaneous");
        assert_eq!(label.kind, SectionKind::Other);
        assert_eq!(label.name, "Spontaneous");
        assert_eq!(label.shortcut(), None);

        // A bare number is not a section kind.
        assert_eq!(SectionLabel::parse("2").kind, SectionKind::Other);
    }

    #[test]
    fn splits_lyrics_into_sections() {
        let lyrics = "\
Untitled intro line

[Verse 1]
Line one
Line two

Line three

[Chorus]
Chorus line

[Bridge]
";
        let sections = parse_sections(lyrics);
        assert_eq!(sections.len(), 3, "the empty bridge is dropped");

        assert_eq!(sections[0].label, None);
        assert_eq!(sections[0].text, "Untitled intro line");

        assert_eq!(
            sections[1].label,
            Some(SectionLabel::new(SectionKind::Verse, Some(1)))
        );
        assert_eq!(sections[1].text, "Line one\nLine two\n\nLine three");

        assert_eq!(sections[2].text, "Chorus line");
    }

    #[test]
    fn arranges_sections_by_shortcut() {
        let sections = parse_sections("[V1]\nOne\n[Chorus]\nRefrain\n[V2]\nTwo");
        let order = parse_arrangement("V1, C V2 C X9");

        let texts: Vec<_> = arrange(&sections, &order)
            .into_iter()
            .map(|s| s.text)
            .collect();
        assert_eq!(texts, ["One", "Refrain", "Two", "Refrain"]);
    }

    #[test]
    fn empty_arrangement_keeps_document_order() {
        let sections = parse_sections("[V1]\nOne\n[V2]\nTwo");
        assert_eq!(arrange(&sections, &[]), sections);
    }

    #[test]
    fn unnumbered_token_prefers_unnumbered_section() {
        let sections = parse_sections("[Chorus 2]\nSecond\n[Chorus]\nPlain");
        let arranged = arrange(&sections, &parse_arrangement("C"));
        assert_eq!(arranged[0].text, "Plain");

        // ...but still falls back to a numbered one when that is all there is.
        let sections = parse_sections("[Chorus 1]\nOnly");
        assert_eq!(arrange(&sections, &parse_arrangement("C"))[0].text, "Only");
    }
}

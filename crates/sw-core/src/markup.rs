//! Bold, italic and underline in presentation slides.
//!
//! Slide text is stored with a tiny markup, the same that many chat apps
//! use, so it stays readable as plain text and survives copy and paste:
//!
//! | Markup         | Shows as      |
//! | -------------- | ------------- |
//! | `**words**`    | **bold**      |
//! | `*words*`      | *italic*      |
//! | `__words__`    | underlined    |
//! | `\*`           | a plain `*`   |
//!
//! Markers can be combined (`***bold italic***`). A marker that is never
//! closed is shown as it is, so a stray asterisk does no harm.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A piece of text in one style. Line breaks stay in the text as `\n`.
///
/// Colour, size and font are only set where they differ from the text
/// box's own style (`None` follows the box).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct TextRun {
    /// The text.
    pub text: String,
    /// Bold.
    #[serde(default)]
    pub bold: bool,
    /// Italic.
    #[serde(default)]
    pub italic: bool,
    /// Underlined.
    #[serde(default)]
    pub underline: bool,
    /// CSS colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub color: Option<String>,
    /// Font size in reference pixels (scaled with the box when it shrinks
    /// to fit).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub font_size: Option<f32>,
    /// CSS font family list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub font_family: Option<String>,
}

/// The markers, longest first so `**` is not read as two `*`.
const MARKERS: [&str; 3] = ["**", "__", "*"];

/// Splits marked-up text into styled runs. Neighbouring runs always differ
/// in style; empty runs are left out.
pub fn parse(markup: &str) -> Vec<TextRun> {
    let mut runs: Vec<TextRun> = Vec::new();
    let mut style = TextRun::default();
    let mut text = String::new();
    let mut rest = markup;

    while let Some(first) = rest.chars().next() {
        // An escaped character is always plain text.
        if first == '\\'
            && let Some(escaped) = rest[1..].chars().next()
            && "*_\\".contains(escaped)
        {
            text.push(escaped);
            rest = &rest[1 + escaped.len_utf8()..];
            continue;
        }

        let marker = MARKERS.iter().find(|marker| rest.starts_with(**marker));
        if let Some(&marker) = marker {
            let turning_on = !is_on(&style, marker);
            // Only turn a style on if it is closed later; otherwise the
            // marker is just text.
            if !turning_on || rest[marker.len()..].contains(marker) {
                push_run(&mut runs, &style, &mut text);
                toggle(&mut style, marker);
                rest = &rest[marker.len()..];
                continue;
            }
        }

        text.push(first);
        rest = &rest[first.len_utf8()..];
    }
    push_run(&mut runs, &style, &mut text);
    runs
}

/// The text without any markup, for the stage display and thumbnails.
pub fn plain_text(markup: &str) -> String {
    parse(markup).into_iter().map(|run| run.text).collect()
}

/// Whether the text uses any styling at all.
pub fn has_styling(runs: &[TextRun]) -> bool {
    runs.iter()
        .any(|run| run.bold || run.italic || run.underline)
}

fn is_on(style: &TextRun, marker: &str) -> bool {
    match marker {
        "**" => style.bold,
        "__" => style.underline,
        _ => style.italic,
    }
}

fn toggle(style: &mut TextRun, marker: &str) {
    match marker {
        "**" => style.bold = !style.bold,
        "__" => style.underline = !style.underline,
        _ => style.italic = !style.italic,
    }
}

/// Ends the current run (if it has any text), merging it with the previous
/// one when they look the same.
fn push_run(runs: &mut Vec<TextRun>, style: &TextRun, text: &mut String) {
    if text.is_empty() {
        return;
    }
    let text = std::mem::take(text);
    if let Some(last) = runs.last_mut()
        && (last.bold, last.italic, last.underline) == (style.bold, style.italic, style.underline)
    {
        last.text.push_str(&text);
        return;
    }
    runs.push(TextRun {
        text,
        ..style.clone()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn styles(markup: &str) -> Vec<(String, &'static str)> {
        parse(markup)
            .into_iter()
            .map(|run| {
                let style = match (run.bold, run.italic, run.underline) {
                    (false, false, false) => "plain",
                    (true, false, false) => "bold",
                    (false, true, false) => "italic",
                    (false, false, true) => "underline",
                    (true, true, false) => "bold italic",
                    _ => "other",
                };
                (run.text, style)
            })
            .collect()
    }

    fn run(text: &str, style: &'static str) -> (String, &'static str) {
        (text.to_owned(), style)
    }

    #[test]
    fn reads_bold_italic_and_underline() {
        assert_eq!(
            styles("Join us **Sunday** at *10am* for __Harvest__"),
            [
                run("Join us ", "plain"),
                run("Sunday", "bold"),
                run(" at ", "plain"),
                run("10am", "italic"),
                run(" for ", "plain"),
                run("Harvest", "underline"),
            ]
        );
    }

    #[test]
    fn combines_markers() {
        assert_eq!(styles("***All***"), [run("All", "bold italic")]);
    }

    #[test]
    fn keeps_unclosed_markers_and_escapes() {
        assert_eq!(styles("5 * 3 = 15"), [run("5 * 3 = 15", "plain")]);
        assert_eq!(styles(r"\*terms apply"), [run("*terms apply", "plain")]);
        assert_eq!(plain_text("Fill in ___"), "Fill in ___");
    }

    #[test]
    fn keeps_line_breaks_and_strips_markup() {
        assert_eq!(plain_text("**Notices**\nChoir at 4"), "Notices\nChoir at 4");
        assert!(!has_styling(&parse("Just text")));
        assert!(has_styling(&parse("**Bold**")));
    }
}

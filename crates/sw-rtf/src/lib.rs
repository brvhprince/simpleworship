//! Turning RTF into plain lines of text.
//!
//! EasyWorship and ProPresenter store lyrics as RTF, and EasyWorship Bibles
//! keep their copyright notice as RTF too (see [`plain_line`]). We need only the
//! words and where the lines break, so this is a small, forgiving reader
//! rather than a full RTF implementation:
//!
//! - `\par` and `\line` end a line. An empty paragraph is an empty line,
//!   which in lyrics means "new slide".
//! - Font tables, colour tables, style sheets, list numbering and every
//!   `{\*\…}` group (which RTF readers may ignore) are skipped.
//! - `\uN` (Unicode) and `\'hh` (a byte in the document's code page,
//!   Windows-1252 by default) become the right characters.
//! - Unknown control words, including EasyWorship's own (`\sdslidemarker`,
//!   `\fntnamaut`, …), are ignored, so new ones never break an import.

/// Groups whose whole content is not text.
const SKIPPED_DESTINATIONS: &[&str] = &[
    "fonttbl",
    "colortbl",
    "stylesheet",
    "info",
    "pict",
    "header",
    "footer",
    "listtable",
    "listoverridetable",
    "pntext",
    "pntxtb",
    "pntxta",
    "generator",
    "themedata",
    "xmlnstbl",
    "datastore",
    "latentstyles",
];

/// A one-line plain text version of a short note that may be RTF, such as
/// an EasyWorship Bible's copyright notice (shown as one footer line): RTF
/// is read first, then its lines are joined with spaces, whatever ends them
/// (`\n`, `\r\n`, or a lone `\r` as old Mac text has).
pub fn plain_line(text: &str) -> String {
    let trimmed = text.trim();
    let plain = if trimmed.starts_with("{\\rtf") {
        to_text(trimmed)
    } else {
        trimmed.to_owned()
    };
    plain
        .split(['\r', '\n'])
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Converts RTF to plain text, one line per paragraph (`\n` between them).
pub fn to_text(rtf: &str) -> String {
    let mut reader = Reader {
        chars: rtf.chars().collect(),
        position: 0,
        output: String::new(),
        groups: vec![Group::default()],
    };
    reader.run();
    tidy(&reader.output)
}

/// What a group (`{…}`) changes for the text inside it.
#[derive(Clone, Copy)]
struct Group {
    /// Text in this group is not output.
    skip: bool,
    /// How many fallback characters follow each `\uN` (set by `\ucN`).
    unicode_skip: usize,
}

impl Default for Group {
    /// RTF's defaults: text is shown, and one fallback character follows `\uN`.
    fn default() -> Self {
        Self {
            skip: false,
            unicode_skip: 1,
        }
    }
}

struct Reader {
    chars: Vec<char>,
    position: usize,
    output: String,
    groups: Vec<Group>,
}

impl Reader {
    fn run(&mut self) {
        while let Some(c) = self.next() {
            match c {
                '{' => {
                    let mut group = self.current();
                    // `{\*\destination …}` may be ignored by any reader.
                    if self.peek_is("\\*") {
                        group.skip = true;
                    }
                    self.groups.push(group);
                }
                '}' => {
                    if self.groups.len() > 1 {
                        self.groups.pop();
                    }
                }
                '\\' => self.control(),
                // Raw line breaks in RTF source are not text.
                '\r' | '\n' => {}
                c => self.emit(c),
            }
        }
    }

    /// Handles what follows a backslash.
    fn control(&mut self) {
        let Some(c) = self.next() else { return };
        if !c.is_ascii_alphabetic() {
            match c {
                '\'' => {
                    let hex: String = [self.next(), self.next()].into_iter().flatten().collect();
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                        self.emit(windows_1252(byte));
                    }
                }
                '~' => self.emit('\u{a0}'),
                '_' => self.emit('-'),
                '\\' | '{' | '}' => self.emit(c),
                // `\*`, `\-` (optional hyphen) and others print nothing.
                _ => {}
            }
            return;
        }

        let mut word = String::from(c);
        while let Some(&next) = self.chars.get(self.position) {
            if next.is_ascii_alphabetic() {
                word.push(next);
                self.position += 1;
            } else {
                break;
            }
        }
        let number = self.read_number();
        // A single space after a control word is part of the control word.
        if self.chars.get(self.position) == Some(&' ') {
            self.position += 1;
        }
        self.apply(&word, number);
    }

    fn apply(&mut self, word: &str, number: Option<i32>) {
        if SKIPPED_DESTINATIONS.contains(&word) {
            self.groups.last_mut().expect("never empty").skip = true;
            return;
        }
        match word {
            "par" | "line" => self.emit('\n'),
            "tab" => self.emit(' '),
            "emdash" => self.emit('—'),
            "endash" => self.emit('–'),
            "lquote" => self.emit('‘'),
            "rquote" => self.emit('’'),
            "ldblquote" => self.emit('“'),
            "rdblquote" => self.emit('”'),
            "bullet" => self.emit('•'),
            "uc" => {
                self.groups.last_mut().expect("never empty").unicode_skip =
                    usize::try_from(number.unwrap_or(1)).unwrap_or(0);
            }
            "u" => {
                if let Some(code) = number {
                    // RTF writes code points above 32767 as negative numbers.
                    let code = if code < 0 { code + 65_536 } else { code };
                    if let Some(c) = char::from_u32(code as u32) {
                        self.emit(c);
                    }
                    self.skip_fallback();
                }
            }
            _ => {}
        }
    }

    /// Skips the plain characters RTF writes after `\uN` for old readers.
    fn skip_fallback(&mut self) {
        let mut remaining = self.current().unicode_skip;
        while remaining > 0 {
            match self.chars.get(self.position) {
                Some('\\') if self.chars.get(self.position + 1) == Some(&'\'') => {
                    self.position += 4
                }
                Some('{' | '}') | None => break,
                Some(_) => self.position += 1,
            }
            remaining -= 1;
        }
    }

    fn read_number(&mut self) -> Option<i32> {
        let start = self.position;
        if self.chars.get(self.position) == Some(&'-') {
            self.position += 1;
        }
        while self
            .chars
            .get(self.position)
            .is_some_and(char::is_ascii_digit)
        {
            self.position += 1;
        }
        let text: String = self.chars[start..self.position].iter().collect();
        text.parse().ok()
    }

    fn emit(&mut self, c: char) {
        if !self.current().skip {
            self.output.push(c);
        }
    }

    fn current(&self) -> Group {
        *self.groups.last().expect("never empty")
    }

    fn next(&mut self) -> Option<char> {
        let c = self.chars.get(self.position).copied();
        self.position += 1;
        c
    }

    fn peek_is(&self, text: &str) -> bool {
        text.chars()
            .enumerate()
            .all(|(i, c)| self.chars.get(self.position + i) == Some(&c))
    }
}

/// Trims each line and drops blank lines at the start and end.
fn tidy(text: &str) -> String {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let first = lines.iter().position(|line| !line.is_empty());
    let last = lines.iter().rposition(|line| !line.is_empty());
    match (first, last) {
        (Some(first), Some(last)) => lines[first..=last].join("\n"),
        _ => String::new(),
    }
}

/// A Windows-1252 byte as a character. Bytes 0x80–0x9F differ from Latin-1
/// (curly quotes, dashes, the euro sign); everything else matches it.
fn windows_1252(byte: u8) -> char {
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž',
        '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}',
        'ž', 'Ÿ',
    ];
    match byte {
        0x80..=0x9f => HIGH[usize::from(byte - 0x80)],
        _ => char::from(byte),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copyright_notice_in_rtf_becomes_one_plain_line() {
        // As EasyWorship keeps a Bible's copyright (HCSB, from a real .ewb).
        let rtf = r"{\rtf1\ansi\ansicpg1252\deff0\nouicompat{\fonttbl{\f0\fnil\fcharset0 Courier New;}}
{\colortbl ;\red0\green0\blue0;}
{\*\generator Riched20 10.0.19041}\viewkind4\uc1
\pard\cf1\f0\fs20 Holman Christian Standard Bible\'ae\par
Copyright \'a9 1999, 2000, 2002, 2003 by Holman Bible Publishers.\par
}";
        assert_eq!(
            plain_line(rtf),
            "Holman Christian Standard Bible® Copyright © 1999, 2000, 2002, 2003 by Holman Bible Publishers."
        );
    }

    #[test]
    fn a_plain_copyright_notice_stays_as_it_is() {
        assert_eq!(plain_line("  Public Domain "), "Public Domain");
        assert_eq!(plain_line(""), "");
    }

    #[test]
    fn a_plain_notice_on_several_lines_becomes_one() {
        assert_eq!(
            plain_line("Bible Society of Ghana\r\n\r\nAll rights reserved.\n"),
            "Bible Society of Ghana All rights reserved."
        );
        assert_eq!(
            plain_line("Copyright\rAll rights reserved"),
            "Copyright All rights reserved"
        );
    }

    #[test]
    fn reads_easyworship_6_paragraphs() {
        let rtf = r"{\rtf1\ansi\deff0{\fonttbl{\f0\fnil\fcharset0 Arial;}}{\colortbl\red0\green0\blue0;}
{\*\pnseclvl1\pnucrm\pnstart1{\pntxtb}{\pntxta{.}}}
{\pard\li0\fi0 \plain\f1\fntnamaut Verse 1\par
\li0\fi0 \plain\f1\fntnamaut Amazing grace how sweet the sound,\par
\li0\fi0 \par
\li0\fi0 \plain\f1\fntnamaut Verse 2\par}}";
        assert_eq!(
            to_text(rtf),
            "Verse 1\nAmazing grace how sweet the sound,\n\nVerse 2"
        );
    }

    #[test]
    fn reads_easyworship_7_slide_markers() {
        let rtf = r"{\rtf1\ansi\deff0\sdeasyworship2
{\pard\qc\sdewparatemplatestyle102{\*\sdasfactor 1}{\*\sdasbaseline 64}\plain\fs128{\*\sdfsreal 64}\sdfsauto Verse 1\par}
{\pard\qc\plain\fs128{\*\sdfsreal 64}\sdfsauto First line\par}
{\pard\sdslidemarker\qc\plain\fs128{\*\sdfsreal 64}\sdfsauto\par}
{\pard\qc\plain\fs128\sdfsauto Second slide\par}}";
        assert_eq!(to_text(rtf), "Verse 1\nFirst line\n\nSecond slide");
    }

    #[test]
    fn decodes_unicode_and_code_page_characters() {
        assert_eq!(to_text(r"{\rtf1 For\u8217?s sake\par}"), "For’s sake");
        assert_eq!(to_text(r"{\rtf1 caf\'e9 \'93hi\'94\par}"), "café “hi”");
        assert_eq!(to_text(r"{\rtf1\uc0 \u603\u596  ne\par}"), "ɛɔ ne");
        // The single space after a control word belongs to the control word.
        assert_eq!(to_text(r"{\rtf1\uc0 \u596 ne\par}"), "ɔne");
        assert_eq!(to_text(r"{\rtf1 a\u-3913?b\par}"), "a\u{f0b7}b");
    }

    #[test]
    fn keeps_escaped_braces_and_backslashes() {
        assert_eq!(to_text(r"{\rtf1 a \{b\} \\ c\par}"), r"a {b} \ c");
    }
}

//! The books of the Bible.
//!
//! Books are identified by a [`BookId`]: 1–66 for the books every Bible
//! shares, in the usual Protestant order (Genesis = 1, Revelation = 66),
//! and 67 onwards for the deuterocanonical books found in Catholic and
//! Orthodox Bibles.
//!
//! Each book also knows its identifiers in the common file formats (OSIS
//! and USFM codes), its English name, and the abbreviations people type.
//! Names for other languages can be added as further tables later.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Identifies a book: 1 = Genesis … 66 = Revelation, 67+ = deuterocanon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bible.ts", type = "number")]
pub struct BookId(pub u8);

/// Which part of the Bible a book belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Testament {
    /// Genesis to Malachi.
    Old,
    /// Matthew to Revelation.
    New,
    /// Books in Catholic and Orthodox Bibles only (Tobit, Sirach, …).
    Deuterocanon,
}

/// Everything known about one book.
#[derive(Debug)]
pub struct Book {
    /// The book's id.
    pub id: BookId,
    /// The OSIS identifier, e.g. `"1Cor"`.
    pub osis: &'static str,
    /// The USFM / USX code, e.g. `"1CO"`.
    pub usfm: &'static str,
    /// The English name, e.g. `"1 Corinthians"`.
    pub name: &'static str,
    /// Extra spellings and abbreviations people type, already normalized
    /// (lowercase letters and digits only, see [`normalize_name`]).
    pub abbreviations: &'static [&'static str],
    /// Which part of the Bible it is in.
    pub testament: Testament,
}

impl BookId {
    /// The book's details, if the id is known.
    pub fn book(self) -> Option<&'static Book> {
        BOOKS.iter().find(|book| book.id == self)
    }

    /// The English name, or `"Book N"` for an unknown id.
    pub fn name(self) -> String {
        self.book()
            .map(|book| book.name.to_owned())
            .unwrap_or_else(|| format!("Book {}", self.0))
    }
}

/// Finds a book by its OSIS identifier (`"Gen"`), ignoring case.
pub fn by_osis(osis: &str) -> Option<&'static Book> {
    BOOKS
        .iter()
        .find(|book| book.osis.eq_ignore_ascii_case(osis))
}

/// Finds a book by its USFM code (`"GEN"`), ignoring case.
pub fn by_usfm(code: &str) -> Option<&'static Book> {
    BOOKS
        .iter()
        .find(|book| book.usfm.eq_ignore_ascii_case(code))
}

/// Finds a book by an exact name or abbreviation (`"Genesis"`, `"1 Cor"`).
/// For partial names typed by an operator, see [`crate::reference`].
pub fn by_name(name: &str) -> Option<&'static Book> {
    let key = normalize_name(name);
    BOOKS.iter().find(|book| book.matches_exactly(&key))
}

impl Book {
    /// Whether a normalized name is this book's name or one of its abbreviations.
    pub(crate) fn matches_exactly(&self, key: &str) -> bool {
        normalize_name(self.name) == key
            || self.osis.eq_ignore_ascii_case(key)
            || self.abbreviations.contains(&key)
    }

    /// Whether a normalized, partially typed name could be this book.
    pub(crate) fn matches_prefix(&self, key: &str) -> bool {
        normalize_name(self.name).starts_with(key)
            || self.abbreviations.iter().any(|abbr| abbr.starts_with(key))
    }
}

/// Normalizes a book name for matching: lowercase letters and digits only,
/// with a leading Roman numeral or ordinal turned into a digit.
///
/// `"1 Cor."`, `"I Cor"` and `"First Corinthians"` all start with `"1co"`.
pub fn normalize_name(name: &str) -> String {
    let lower = name.trim().to_lowercase();

    // A numeral only counts when it is a separate word ("ii kings"), so
    // that "isa" (Isaiah) is not read as "1 sa" (1 Samuel).
    let numerals = [
        ("iii", "3"),
        ("ii", "2"),
        ("iv", "4"),
        ("i", "1"),
        ("first", "1"),
        ("second", "2"),
        ("third", "3"),
        ("fourth", "4"),
        ("1st", "1"),
        ("2nd", "2"),
        ("3rd", "3"),
        ("4th", "4"),
    ];
    let mut rest = lower.as_str();
    let mut prefix = "";
    for (word, digit) in numerals {
        if let Some(after) = rest.strip_prefix(word)
            && after.starts_with([' ', '.'])
        {
            prefix = digit;
            rest = after;
            break;
        }
    }

    let mut key = prefix.to_owned();
    key.extend(rest.chars().filter(|c| c.is_alphanumeric()));
    key
}

/// Shorthand for the table below.
const fn book(
    id: u8,
    osis: &'static str,
    usfm: &'static str,
    name: &'static str,
    abbreviations: &'static [&'static str],
    testament: Testament,
) -> Book {
    Book {
        id: BookId(id),
        osis,
        usfm,
        name,
        abbreviations,
        testament,
    }
}

use Testament::{Deuterocanon as D, New as N, Old as O};

/// Every known book, in canonical order. When a partly typed name could be
/// several books, the one that comes first here wins.
pub static BOOKS: &[Book] = &[
    book(1, "Gen", "GEN", "Genesis", &["gen", "ge", "gn"], O),
    book(2, "Exod", "EXO", "Exodus", &["exo", "ex", "exod"], O),
    book(3, "Lev", "LEV", "Leviticus", &["lev", "le", "lv"], O),
    book(4, "Num", "NUM", "Numbers", &["num", "nu", "nm", "nb"], O),
    book(
        5,
        "Deut",
        "DEU",
        "Deuteronomy",
        &["deut", "deu", "de", "dt"],
        O,
    ),
    book(6, "Josh", "JOS", "Joshua", &["josh", "jos", "jsh"], O),
    book(
        7,
        "Judg",
        "JDG",
        "Judges",
        &["judg", "jdg", "jg", "jdgs"],
        O,
    ),
    book(8, "Ruth", "RUT", "Ruth", &["ruth", "rut", "rth", "ru"], O),
    book(9, "1Sam", "1SA", "1 Samuel", &["1sam", "1sa", "1sm"], O),
    book(10, "2Sam", "2SA", "2 Samuel", &["2sam", "2sa", "2sm"], O),
    book(
        11,
        "1Kgs",
        "1KI",
        "1 Kings",
        &["1kgs", "1ki", "1kin", "1k"],
        O,
    ),
    book(
        12,
        "2Kgs",
        "2KI",
        "2 Kings",
        &["2kgs", "2ki", "2kin", "2k"],
        O,
    ),
    book(
        13,
        "1Chr",
        "1CH",
        "1 Chronicles",
        &["1chr", "1ch", "1chron"],
        O,
    ),
    book(
        14,
        "2Chr",
        "2CH",
        "2 Chronicles",
        &["2chr", "2ch", "2chron"],
        O,
    ),
    book(15, "Ezra", "EZR", "Ezra", &["ezra", "ezr"], O),
    book(16, "Neh", "NEH", "Nehemiah", &["neh", "ne"], O),
    book(17, "Esth", "EST", "Esther", &["esth", "est", "es"], O),
    book(18, "Job", "JOB", "Job", &["job", "jb"], O),
    book(
        19,
        "Ps",
        "PSA",
        "Psalms",
        &["ps", "psa", "psalm", "pss", "psm"],
        O,
    ),
    book(
        20,
        "Prov",
        "PRO",
        "Proverbs",
        &["prov", "pro", "pr", "prv"],
        O,
    ),
    book(
        21,
        "Eccl",
        "ECC",
        "Ecclesiastes",
        &["eccl", "ecc", "ec", "qoh"],
        O,
    ),
    book(
        22,
        "Song",
        "SNG",
        "Song of Songs",
        &["song", "sng", "sos", "songofsolomon", "canticles", "cant"],
        O,
    ),
    book(23, "Isa", "ISA", "Isaiah", &["isa", "is"], O),
    book(24, "Jer", "JER", "Jeremiah", &["jer", "je", "jr"], O),
    book(25, "Lam", "LAM", "Lamentations", &["lam", "la"], O),
    book(26, "Ezek", "EZK", "Ezekiel", &["ezek", "eze", "ezk"], O),
    book(27, "Dan", "DAN", "Daniel", &["dan", "da", "dn"], O),
    book(28, "Hos", "HOS", "Hosea", &["hos", "ho"], O),
    book(29, "Joel", "JOL", "Joel", &["joel", "jol", "jl"], O),
    book(30, "Amos", "AMO", "Amos", &["amos", "amo", "am"], O),
    book(31, "Obad", "OBA", "Obadiah", &["obad", "oba", "ob"], O),
    book(32, "Jonah", "JON", "Jonah", &["jonah", "jon", "jnh"], O),
    book(33, "Mic", "MIC", "Micah", &["mic", "mc"], O),
    book(34, "Nah", "NAM", "Nahum", &["nah", "nam", "na"], O),
    book(35, "Hab", "HAB", "Habakkuk", &["hab", "hb"], O),
    book(36, "Zeph", "ZEP", "Zephaniah", &["zeph", "zep", "zp"], O),
    book(37, "Hag", "HAG", "Haggai", &["hag", "hg"], O),
    book(38, "Zech", "ZEC", "Zechariah", &["zech", "zec", "zc"], O),
    book(39, "Mal", "MAL", "Malachi", &["mal", "ml"], O),
    book(40, "Matt", "MAT", "Matthew", &["matt", "mat", "mt"], N),
    book(41, "Mark", "MRK", "Mark", &["mark", "mrk", "mk", "mr"], N),
    book(42, "Luke", "LUK", "Luke", &["luke", "luk", "lk"], N),
    book(43, "John", "JHN", "John", &["john", "jhn", "jn", "joh"], N),
    book(44, "Acts", "ACT", "Acts", &["acts", "act", "ac"], N),
    book(45, "Rom", "ROM", "Romans", &["rom", "ro", "rm"], N),
    book(46, "1Cor", "1CO", "1 Corinthians", &["1cor", "1co"], N),
    book(47, "2Cor", "2CO", "2 Corinthians", &["2cor", "2co"], N),
    book(48, "Gal", "GAL", "Galatians", &["gal", "ga"], N),
    book(49, "Eph", "EPH", "Ephesians", &["eph", "ephes"], N),
    book(50, "Phil", "PHP", "Philippians", &["phil", "php", "pp"], N),
    book(51, "Col", "COL", "Colossians", &["col"], N),
    book(
        52,
        "1Thess",
        "1TH",
        "1 Thessalonians",
        &["1thess", "1thes", "1th"],
        N,
    ),
    book(
        53,
        "2Thess",
        "2TH",
        "2 Thessalonians",
        &["2thess", "2thes", "2th"],
        N,
    ),
    book(54, "1Tim", "1TI", "1 Timothy", &["1tim", "1ti", "1tm"], N),
    book(55, "2Tim", "2TI", "2 Timothy", &["2tim", "2ti", "2tm"], N),
    book(56, "Titus", "TIT", "Titus", &["titus", "tit"], N),
    book(
        57,
        "Phlm",
        "PHM",
        "Philemon",
        &["phlm", "phm", "philem", "pm"],
        N,
    ),
    book(58, "Heb", "HEB", "Hebrews", &["heb"], N),
    book(59, "Jas", "JAS", "James", &["jas", "jam", "jm"], N),
    book(
        60,
        "1Pet",
        "1PE",
        "1 Peter",
        &["1pet", "1pe", "1pt", "1p"],
        N,
    ),
    book(
        61,
        "2Pet",
        "2PE",
        "2 Peter",
        &["2pet", "2pe", "2pt", "2p"],
        N,
    ),
    book(
        62,
        "1John",
        "1JN",
        "1 John",
        &["1john", "1jn", "1jo", "1jhn"],
        N,
    ),
    book(
        63,
        "2John",
        "2JN",
        "2 John",
        &["2john", "2jn", "2jo", "2jhn"],
        N,
    ),
    book(
        64,
        "3John",
        "3JN",
        "3 John",
        &["3john", "3jn", "3jo", "3jhn"],
        N,
    ),
    book(65, "Jude", "JUD", "Jude", &["jude", "jud", "jd"], N),
    book(
        66,
        "Rev",
        "REV",
        "Revelation",
        &["rev", "re", "rv", "revelations", "apocalypse"],
        N,
    ),
    // Deuterocanonical books.
    book(67, "Tob", "TOB", "Tobit", &["tob", "tb"], D),
    book(68, "Jdt", "JDT", "Judith", &["jdt", "jdth"], D),
    book(
        69,
        "AddEsth",
        "ESG",
        "Esther (Greek)",
        &["esg", "addesth", "gkesth"],
        D,
    ),
    book(
        70,
        "Wis",
        "WIS",
        "Wisdom of Solomon",
        &["wis", "wisdom", "ws"],
        D,
    ),
    book(
        71,
        "Sir",
        "SIR",
        "Sirach",
        &["sir", "ecclus", "ecclesiasticus"],
        D,
    ),
    book(72, "Bar", "BAR", "Baruch", &["bar"], D),
    book(
        73,
        "EpJer",
        "LJE",
        "Letter of Jeremiah",
        &["lje", "epjer"],
        D,
    ),
    book(
        74,
        "PrAzar",
        "S3Y",
        "Prayer of Azariah",
        &["s3y", "prazar"],
        D,
    ),
    book(75, "Sus", "SUS", "Susanna", &["sus"], D),
    book(76, "Bel", "BEL", "Bel and the Dragon", &["bel"], D),
    book(
        77,
        "1Macc",
        "1MA",
        "1 Maccabees",
        &["1macc", "1mac", "1ma"],
        D,
    ),
    book(
        78,
        "2Macc",
        "2MA",
        "2 Maccabees",
        &["2macc", "2mac", "2ma"],
        D,
    ),
    book(
        79,
        "3Macc",
        "3MA",
        "3 Maccabees",
        &["3macc", "3mac", "3ma"],
        D,
    ),
    book(
        80,
        "4Macc",
        "4MA",
        "4 Maccabees",
        &["4macc", "4mac", "4ma"],
        D,
    ),
    book(81, "1Esd", "1ES", "1 Esdras", &["1esd", "1es"], D),
    book(82, "2Esd", "2ES", "2 Esdras", &["2esd", "2es"], D),
    book(
        83,
        "PrMan",
        "MAN",
        "Prayer of Manasseh",
        &["man", "prman"],
        D,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_in_order() {
        for (index, book) in BOOKS.iter().enumerate() {
            assert_eq!(usize::from(book.id.0), index + 1, "{}", book.name);
        }
    }

    #[test]
    fn abbreviations_are_normalized_and_unambiguous() {
        for book in BOOKS {
            for abbreviation in book.abbreviations {
                assert_eq!(&normalize_name(abbreviation), abbreviation, "{}", book.name);
                let owners: Vec<_> = BOOKS
                    .iter()
                    .filter(|other| other.matches_exactly(abbreviation))
                    .map(|other| other.name)
                    .collect();
                assert_eq!(
                    owners,
                    [book.name],
                    "{abbreviation:?} must belong to one book"
                );
            }
        }
    }

    #[test]
    fn normalizes_numerals_and_punctuation() {
        assert_eq!(normalize_name("1 Cor."), "1cor");
        assert_eq!(normalize_name("I Cor"), "1cor");
        assert_eq!(normalize_name("II Kings"), "2kings");
        assert_eq!(normalize_name("First John"), "1john");
        assert_eq!(normalize_name("Song of Songs"), "songofsongs");
        assert_eq!(normalize_name("Isa"), "isa", "not 1 Samuel");
    }

    #[test]
    fn finds_books_by_code_and_name() {
        assert_eq!(by_osis("gen").unwrap().name, "Genesis");
        assert_eq!(by_usfm("1CO").unwrap().name, "1 Corinthians");
        assert_eq!(by_name("Psalm").unwrap().name, "Psalms");
        assert_eq!(by_name("Song of Solomon").unwrap().name, "Song of Songs");
        assert!(by_name("Hezekiah").is_none());
    }
}

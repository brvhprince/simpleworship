//! Every supported format, fed the same tiny Bible, must give the same
//! verses. The fixtures include the traps real files have: footnotes,
//! Strong's numbers, headings, entities, line breaks inside verses, and both
//! styles of verse markers.

use sw_bible::BookId;
use sw_bible::import::{BibleData, import};

/// What every fixture contains.
const EXPECTED: &[(u8, u16, u16, &str)] = &[
    (
        1,
        1,
        1,
        "In the beginning God created the heavens and the earth.",
    ),
    (1, 1, 2, "The earth was formless and empty."),
    (
        1,
        1,
        3,
        "God said, “Let there be light,” and there was light.",
    ),
    (43, 3, 16, "For God so loved the world & gave his Son."),
];

fn verses(bible: &BibleData) -> Vec<(u8, u16, u16, &str)> {
    bible
        .verses
        .iter()
        .map(|v| (v.book.0, v.chapter, v.verse, v.text.as_str()))
        .collect()
}

fn import_fixture(name: &str, bytes: &[u8]) -> BibleData {
    import(name, bytes).unwrap_or_else(|error| panic!("{name}: {error}"))
}

#[test]
fn osis() {
    let bible = import_fixture("osis.xml", include_bytes!("fixtures/osis.xml"));
    assert_eq!(verses(&bible), EXPECTED);
    assert_eq!(bible.name, "Test Bible");
    assert_eq!(bible.abbreviation, "TEST");
    assert_eq!(bible.copyright, "Public Domain");
}

#[test]
fn zefania() {
    let bible = import_fixture("zefania.xml", include_bytes!("fixtures/zefania.xml"));
    assert_eq!(verses(&bible), EXPECTED);
    assert_eq!(bible.name, "Test Bible");
    assert_eq!(bible.language, "en");
}

#[test]
fn opensong() {
    let bible = import_fixture("Test.xmm", include_bytes!("fixtures/opensong.xmm"));
    assert_eq!(verses(&bible), EXPECTED);
    assert_eq!(
        bible.name, "Test",
        "named after the file when it has no title"
    );
}

#[test]
fn usx_zip() {
    let bible = import_fixture("usx.zip", include_bytes!("fixtures/usx.zip"));
    assert_eq!(verses(&bible), EXPECTED);
}

#[test]
fn usfm_zip() {
    let bible = import_fixture("usfm.zip", include_bytes!("fixtures/usfm.zip"));
    assert_eq!(verses(&bible), EXPECTED);
    assert_eq!(bible.name, "Test Bible");
    assert_eq!(
        bible.verses[0].book,
        BookId(1),
        "sorted, though John came first in the zip"
    );
}

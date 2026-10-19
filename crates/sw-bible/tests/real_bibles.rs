//! Imports the Bibles bundled in `resources/bibles` and checks them.
//!
//! These files are large, so the test is ignored by default. Run it after
//! changing an importer or updating the bundled files:
//!
//! ```sh
//! cargo test --release -p sw-bible --test real_bibles -- --ignored
//! ```

use sw_bible::import::{BibleData, import};

fn import_bundled(file: &str) -> BibleData {
    let path = format!(
        "{}/../../resources/bibles/{file}",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    import(file, &bytes).unwrap_or_else(|error| panic!("{file}: {error}"))
}

fn text(bible: &BibleData, book: u8, chapter: u16, verse: u16) -> &str {
    bible
        .verses
        .iter()
        .find(|v| v.book.0 == book && v.chapter == chapter && v.verse == verse)
        .map_or("", |v| v.text.as_str())
}

/// No USFM markup may survive into verse text.
fn assert_clean(bible: &BibleData) {
    let dirty: Vec<_> = bible
        .verses
        .iter()
        .filter(|v| v.text.contains(['\\', '|', '¶']) || v.text.contains("strong="))
        .take(3)
        .collect();
    assert!(dirty.is_empty(), "markup left in {dirty:?}");
}

#[test]
#[ignore = "large files; run with --ignored"]
fn king_james_version() {
    let bible = import_bundled("eng-kjv2006_usfm.zip");
    assert_eq!(bible.verses.len(), 31_102, "the standard KJV verse count");
    assert_eq!(
        text(&bible, 43, 3, 16),
        "For God so loved the world, that he gave his only begotten Son, that whosoever \
         believeth in him should not perish, but have everlasting life."
    );
    assert_clean(&bible);
}

#[test]
#[ignore = "large files; run with --ignored"]
fn world_english_bible() {
    let bible = import_bundled("engwebp_usfm.zip");
    // WEB leaves out a few verses that are missing from the oldest manuscripts.
    assert_eq!(bible.verses.len(), 31_098);
    assert_eq!(bible.name, "World English Bible (WEB)");
    assert_eq!(
        text(&bible, 19, 23, 1),
        "The LORD is my shepherd; I shall lack nothing."
    );
    assert_clean(&bible);
}

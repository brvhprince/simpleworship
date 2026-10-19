//! The same public-domain hymn, in every song file format, read through
//! `read_file`: each must give the same title and lyrics.

use std::path::Path;

use sw_import::formats::read_file;

const LYRICS: &str = "[Verse 1]\nAmazing grace how sweet the sound\nThat saved a wretch like me\n\n\
    [Verse 2]\n'Twas grace that taught my heart to fear\nAnd grace my fears relieved";

fn read(name: &str) -> sw_import::ImportedSong {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let mut report = read_file(&path);
    assert!(report.problems.is_empty(), "{name}: {:?}", report.problems);
    assert_eq!(report.songs.len(), 1, "{name}");
    report.songs.remove(0)
}

#[test]
fn every_format_gives_the_same_song() {
    for name in [
        "amazing-grace.xml",
        "amazing-grace.pro6",
        "amazing-grace.cho",
        "amazing-grace.usr",
        "amazing-grace-songselect.txt",
        "Amazing Grace.txt",
    ] {
        let song = read(name);
        assert_eq!(song.title, "Amazing Grace", "{name}");
        assert_eq!(song.lyrics, LYRICS, "{name}");
    }
}

#[test]
fn ccli_details_come_across() {
    for name in [
        "amazing-grace.xml",
        "amazing-grace.pro6",
        "amazing-grace.usr",
        "amazing-grace-songselect.txt",
    ] {
        let song = read(name);
        assert_eq!(song.ccli_number, "22025", "{name}");
        assert_eq!(song.author, "John Newton", "{name}");
    }
}

#[test]
fn other_files_are_reported_not_fatal() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let report = read_file(&path);
    assert!(report.songs.is_empty());
    assert_eq!(report.problems.len(), 1);
}

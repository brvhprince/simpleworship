//! Files written by earlier versions must keep opening.
//!
//! `fixtures/sunday-v1.sws` was written by hand (with Python's `zipfile`),
//! not by this crate, so it also checks that the format is what the docs say.
//! Never regenerate it: add a new fixture when the format changes.

use sw_core::ItemContent;
use sw_files::read_schedule;

#[test]
fn opens_a_version_1_schedule() {
    let bytes = include_bytes!("fixtures/sunday-v1.sws");
    let schedule = read_schedule(bytes).expect("fixture opens");

    let titles: Vec<_> = schedule.items.iter().map(|item| item.title()).collect();
    assert_eq!(titles, ["Welcome", "Amazing Grace"]);
    assert_eq!(schedule.items[0].notes, "Before the countdown");

    let ItemContent::Song { song } = &schedule.items[1].content else {
        panic!("the second item is a song");
    };
    assert_eq!(song.ccli_number, "22025");
    assert!(song.lyrics.starts_with("[Verse 1]"));
}

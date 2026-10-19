//! Imports a real library from another program, if one is provided.
//!
//! Real libraries contain copyrighted lyrics, so they never go into the
//! repository. To run against your own copy:
//!
//! ```sh
//! SW_EASYWORSHIP_DATA="/path/to/EasyWorship/Data" \
//!   cargo test -p sw-import --test real_libraries -- --ignored --nocapture
//! ```

use sw_import::formats::easyworship;

#[test]
#[ignore = "needs a real EasyWorship library; see the module docs"]
fn easyworship_library() {
    let Ok(folder) = std::env::var("SW_EASYWORSHIP_DATA") else {
        eprintln!("SW_EASYWORSHIP_DATA is not set; nothing to test");
        return;
    };
    let report = easyworship::read_library(std::path::Path::new(&folder));

    let with_sections = report
        .songs
        .iter()
        .filter(|s| s.lyrics.contains('['))
        .count();
    let with_author = report.songs.iter().filter(|s| !s.author.is_empty()).count();
    let leftovers = report
        .songs
        .iter()
        .filter(|s| s.lyrics.contains('\\') || s.lyrics.contains('{'))
        .count();
    println!(
        "songs: {} · with sections: {} · with author: {} · RTF left in lyrics: {} · problems: {}",
        report.songs.len(),
        with_sections,
        with_author,
        leftovers,
        report.problems.len()
    );
    for problem in &report.problems {
        println!("  problem: {}: {}", problem.source, problem.message);
    }
    assert_eq!(leftovers, 0, "no RTF may survive into lyrics");
}

/// Lists the section names found, and short lines that might be headings
/// the importer did not recognise, to check heading detection by eye.
#[test]
#[ignore = "needs a real EasyWorship library; see the module docs"]
fn easyworship_headings() {
    let Ok(folder) = std::env::var("SW_EASYWORSHIP_DATA") else {
        return;
    };
    let report = easyworship::read_library(std::path::Path::new(&folder));
    let mut tags = std::collections::BTreeMap::<String, usize>::new();
    let mut suspects = std::collections::BTreeMap::<String, usize>::new();
    let looks_like_heading = |line: &str| {
        let lower = line.to_lowercase();
        [
            "verse",
            "chorus",
            "bridge",
            "tag",
            "intro",
            "ending",
            "refrain",
            "pre",
            "repeat",
            "x2",
            "outro",
            "vamp",
            "interlude",
        ]
        .iter()
        .any(|word| lower.contains(word))
            && line.split_whitespace().count() <= 4
    };
    for song in &report.songs {
        for line in song.lyrics.lines() {
            if line.starts_with('[') {
                *tags.entry(line.to_owned()).or_default() += 1;
            } else if looks_like_heading(line) {
                *suspects.entry(line.to_owned()).or_default() += 1;
            }
        }
    }
    println!("tags: {tags:?}");
    println!("possible missed headings: {suspects:?}");
}

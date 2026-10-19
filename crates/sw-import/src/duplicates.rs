//! Spotting songs that are already in the library.
//!
//! Two songs are duplicates when their titles and the start of their
//! lyrics match after ignoring case, punctuation, spacing and section tags.
//! Importing the same EasyWorship library twice therefore finds nothing
//! new, while two different songs that share a title both come in.

/// How much of the lyrics is compared. Enough to tell songs apart, little
/// enough that a fixed typo near the end still counts as the same song.
const LYRICS_COMPARED: usize = 200;

/// A key that is equal for duplicate songs.
pub fn key(title: &str, lyrics: &str) -> String {
    let words: String = lyrics
        .lines()
        .filter(|line| !(line.trim_start().starts_with('[') && line.trim_end().ends_with(']')))
        .collect::<Vec<_>>()
        .join(" ");
    let lyrics = normalize(&words);
    let lyrics: String = lyrics.chars().take(LYRICS_COMPARED).collect();
    format!("{}|{}", normalize(title), lyrics)
}

/// Lowercase letters and digits, single spaces between words.
fn normalize(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .flat_map(char::to_lowercase)
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_case_punctuation_and_tags() {
        assert_eq!(
            key("Amazing Grace", "[Verse 1]\nAmazing grace, how sweet!"),
            key("amazing grace", "Amazing Grace how   sweet")
        );
    }

    #[test]
    fn different_lyrics_are_different_songs() {
        assert_ne!(
            key("Holy", "Holy holy holy"),
            key("Holy", "Holy is the Lamb")
        );
    }
}

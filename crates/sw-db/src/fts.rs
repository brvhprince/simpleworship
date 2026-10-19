//! Helpers for SQLite full-text search (FTS5), shared by songs and Bibles.

/// Turns what the operator typed into an FTS5 query: every word must match
/// as a prefix. Returns `None` if there are no searchable words.
///
/// Each word is quoted, so characters that mean something in FTS5 syntax
/// (`-`, `*`, `"`, `AND`) are treated as plain text.
pub(crate) fn to_fts_query(input: &str) -> Option<String> {
    let terms: Vec<String> = input
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .map(|word| word.trim_matches('\''))
        .filter(|word| !word.is_empty())
        .map(|word| format!("\"{}\"*", word.replace('"', "")))
        .collect();

    (!terms.is_empty()).then(|| terms.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fts_query_quotes_every_word() {
        assert_eq!(
            to_fts_query("amazing grace").unwrap(),
            "\"amazing\"* \"grace\"*"
        );
        assert_eq!(to_fts_query("don't").unwrap(), "\"don't\"*");
        assert_eq!(to_fts_query(" - * "), None);
    }
}

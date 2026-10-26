//! Stable identifiers for every stored record.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// A globally unique, time-ordered identifier (UUID v7).
///
/// UUID v7 sorts by creation time and can be generated on any machine without
/// coordination, which keeps the door open for syncing libraries between
/// several operator computers later on.
///
/// Serialized as the usual hyphenated string, e.g.
/// `"01920c7e-5a4b-7c3d-9e8f-0a1b2c3d4e5f"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings.ts", type = "string")]
pub struct Id(Uuid);

impl Id {
    /// Creates a new, unique identifier.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// The all-zero identifier, for something there is exactly one of
    /// (such as [`crate::engine::LOCAL_OPERATOR`]).
    pub const fn nil() -> Self {
        Self(Uuid::nil())
    }

    /// Wraps an existing UUID, e.g. one derived from older, non-UUID data.
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Id {
    type Err = uuid::Error;

    /// Parses the hyphenated form. Anything else is an error, so a mistyped
    /// or corrupt id is caught instead of quietly naming nothing.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_prints_the_hyphenated_form() {
        let text = "01920c7e-5a4b-7c3d-9e8f-0a1b2c3d4e5f";
        let id: Id = serde_json::from_str(&format!("\"{text}\"")).unwrap();
        assert_eq!(id.to_string(), text);
        assert_eq!(text.parse::<Id>().unwrap(), id);
    }

    #[test]
    fn rejects_anything_that_is_not_a_uuid() {
        assert!("title".parse::<Id>().is_err());
        assert!("".parse::<Id>().is_err());
        assert!(serde_json::from_str::<Id>("\"title\"").is_err());
        assert!(serde_json::from_str::<Id>("\"\"").is_err());
    }
}

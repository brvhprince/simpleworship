//! Stable identifiers for every stored record.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize};
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, TS)]
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
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().is_empty() {
            return Ok(Self::nil());
        }
        let uuid =
            Uuid::parse_str(s).unwrap_or_else(|_| Uuid::new_v5(&Uuid::NAMESPACE_OID, s.as_bytes()));
        Ok(Self(uuid))
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(s.parse::<Self>().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_uuid_and_arbitrary_string() {
        let valid_str = "\"01920c7e-5a4b-7c3d-9e8f-0a1b2c3d4e5f\"";
        let parsed: Id = serde_json::from_str(valid_str).expect("valid uuid");
        assert_eq!(parsed.to_string(), "01920c7e-5a4b-7c3d-9e8f-0a1b2c3d4e5f");

        // Arbitrary strings like "title" or "body" must deserialize without error
        let title_str = "\"title\"";
        let parsed_title: Id = serde_json::from_str(title_str).expect("arbitrary string title");
        assert_ne!(parsed_title, Id::nil());

        // Must be deterministic
        let parsed_title_again: Id = serde_json::from_str(title_str).unwrap();
        assert_eq!(parsed_title, parsed_title_again);

        // Empty string deserializes to nil
        let empty_str = "\"\"";
        let parsed_empty: Id = serde_json::from_str(empty_str).expect("empty string");
        assert_eq!(parsed_empty, Id::nil());
    }
}

//! Timestamp helpers.
//!
//! Timestamps are stored as milliseconds since the Unix epoch (UTC). A plain
//! integer is trivial to store in SQLite, compare, and send to JavaScript,
//! where `new Date(ms)` turns it back into a date.

use std::time::{SystemTime, UNIX_EPOCH};

/// Milliseconds since the Unix epoch.
pub type Timestamp = i64;

/// Returns the current time as a [`Timestamp`].
pub fn now() -> Timestamp {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is set before 1970");

    // i64 milliseconds covers roughly 292 million years, so this cannot overflow.
    elapsed.as_millis() as Timestamp
}

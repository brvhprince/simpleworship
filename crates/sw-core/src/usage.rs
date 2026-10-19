//! Song usage, for CCLI reporting.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One song's line in a usage report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct SongUsage {
    /// Song title.
    pub title: String,
    /// Author(s).
    pub author: String,
    /// Copyright notice.
    pub copyright: String,
    /// CCLI song number (empty if unknown).
    pub ccli_number: String,
    /// On how many days it was used in the period.
    pub times: u32,
    /// The last day it was used, `YYYY-MM-DD`.
    pub last_used: String,
}

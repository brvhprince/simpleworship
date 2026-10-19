//! Countdowns: "Service starts in 4:59". A countdown is a schedule item that
//! counts down either for a set time or to a time of day, drawn with a
//! theme like any slide.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::time::Timestamp;

/// Milliseconds in a day.
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// What a countdown counts down to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "bindings.ts")]
pub enum CountdownTarget {
    /// A set length, starting when the countdown goes live.
    Duration {
        /// How long, in seconds.
        seconds: u32,
    },
    /// A time of day on this computer's clock, e.g. 10:30.
    TimeOfDay {
        /// Hour, 0–23.
        hour: u8,
        /// Minute, 0–59.
        minute: u8,
    },
}

/// What happens when a countdown reaches zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum CountdownEnd {
    /// Stay on 0:00.
    #[default]
    Stay,
    /// Go on to the next schedule item.
    NextItem,
}

/// A countdown on a slide, before it goes live.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct CountdownSlide {
    /// Text above the time, e.g. "Service starts in".
    pub message: String,
    /// What it counts down to.
    pub target: CountdownTarget,
    /// What happens at zero.
    pub at_end: CountdownEnd,
}

/// A live countdown, as renderers draw it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct CountdownFrame {
    /// Text above the time.
    pub message: String,
    /// When it reaches zero (milliseconds since the Unix epoch).
    #[ts(type = "number")]
    pub ends_at: Timestamp,
}

impl CountdownTarget {
    /// When a countdown that goes live at `now` reaches zero.
    /// `utc_offset_minutes` is this computer's time zone (e.g. 60 for UTC+1).
    /// A time of day already passed today gives that past time, so the
    /// countdown shows 0:00 rather than counting to tomorrow.
    pub fn ends_at(self, now: Timestamp, utc_offset_minutes: i32) -> Timestamp {
        match self {
            CountdownTarget::Duration { seconds } => now + i64::from(seconds) * 1000,
            CountdownTarget::TimeOfDay { hour, minute } => {
                let offset = i64::from(utc_offset_minutes) * 60_000;
                let local_now = now + offset;
                let local_midnight = local_now.div_euclid(DAY_MS) * DAY_MS;
                let local_target =
                    local_midnight + (i64::from(hour) * 60 + i64::from(minute)) * 60_000;
                local_target - offset
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_count_from_going_live() {
        let target = CountdownTarget::Duration { seconds: 300 };
        assert_eq!(target.ends_at(1_000, 0), 301_000);
    }

    #[test]
    fn times_of_day_use_the_local_clock() {
        // 2024-01-07 09:00 UTC, in a UTC+1 time zone it is 10:00 local.
        let now = 1_704_618_000_000;
        let target = CountdownTarget::TimeOfDay {
            hour: 10,
            minute: 30,
        };
        let ends = target.ends_at(now, 60);
        assert_eq!(ends - now, 30 * 60_000, "10:30 local is half an hour away");

        // In UTC (offset 0) 10:30 is an hour and a half away.
        assert_eq!(target.ends_at(now, 0) - now, 90 * 60_000);
    }

    #[test]
    fn a_time_already_passed_ends_immediately() {
        let now = 1_704_618_000_000; // 09:00 UTC
        let target = CountdownTarget::TimeOfDay { hour: 8, minute: 0 };
        assert!(target.ends_at(now, 0) < now);
    }
}

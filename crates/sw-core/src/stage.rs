//! What only the stage display shows: a timer for the speaker and messages
//! from the operator ("Wrap up", "5 min"). The congregation never sees them.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::time::Timestamp;

/// Which way a stage timer counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum TimerMode {
    /// Counts up from zero (how long the sermon has run).
    CountUp,
    /// Counts down from a duration, then into overtime (shown negative).
    CountDown,
}

/// A timer on the stage display. Like video, it is an anchor plus a clock:
/// every screen works out the current value from `elapsed_at`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct StageTimer {
    /// Up or down.
    pub mode: TimerMode,
    /// For a count-down, how long it runs, in milliseconds.
    #[ts(type = "number")]
    pub duration_ms: u64,
    /// Time counted so far, at `anchor`.
    #[ts(type = "number")]
    pub elapsed_ms: u64,
    /// When `elapsed_ms` was measured.
    #[ts(type = "number")]
    pub anchor: Timestamp,
    /// Whether it is running.
    pub running: bool,
}

impl StageTimer {
    /// Milliseconds counted at `now`.
    pub fn elapsed_at(&self, now: Timestamp) -> u64 {
        let since = if self.running {
            u64::try_from(now - self.anchor).unwrap_or(0)
        } else {
            0
        };
        self.elapsed_ms + since
    }

    /// What the timer shows at `now`, in milliseconds: elapsed time when
    /// counting up; remaining time when counting down (negative once over).
    pub fn display_ms(&self, now: Timestamp) -> i64 {
        let elapsed = i64::try_from(self.elapsed_at(now)).unwrap_or(i64::MAX);
        match self.mode {
            TimerMode::CountUp => elapsed,
            TimerMode::CountDown => i64::try_from(self.duration_ms).unwrap_or(i64::MAX) - elapsed,
        }
    }
}

/// Everything the stage display shows beyond the slides.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct StageState {
    /// The speaker's timer, if one is set.
    pub timer: Option<StageTimer>,
    /// A message for the people on stage, if any.
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_up_down_and_into_overtime() {
        let timer = StageTimer {
            mode: TimerMode::CountDown,
            duration_ms: 60_000,
            elapsed_ms: 0,
            anchor: 0,
            running: true,
        };
        assert_eq!(timer.display_ms(15_000), 45_000);
        assert_eq!(timer.display_ms(70_000), -10_000, "overtime is negative");

        let paused = StageTimer {
            running: false,
            elapsed_ms: 5_000,
            mode: TimerMode::CountUp,
            ..timer
        };
        assert_eq!(paused.display_ms(999_999), 5_000);
    }
}

//! Alerts: short messages over the slides, such as a nursery call
//! ("Parent of child #42 to the nursery") or a car to be moved.
//!
//! An alert is a banner (still) or a ticker (scrolling across), at the top
//! or bottom of the screen. It sits above the slides and the logo, and
//! below Black. It ends when the operator clears it, after some seconds, or
//! (for a ticker) after it has scrolled across a number of times.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::time::Timestamp;

/// How an alert looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum AlertStyle {
    /// Still text in a band.
    Banner,
    /// Text scrolling from right to left.
    Ticker,
}

/// Where an alert appears.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum AlertPosition {
    /// Along the top of the screen.
    Top,
    /// Along the bottom of the screen.
    Bottom,
}

/// When an alert goes away by itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "bindings.ts")]
pub enum AlertEnd {
    /// Only when the operator clears it.
    Manual,
    /// After this many seconds.
    AfterSeconds {
        /// How long it stays, in seconds.
        seconds: u32,
    },
    /// For a ticker: after scrolling across this many times. (The pass time
    /// depends on the text width, which only the renderer knows.)
    AfterPasses {
        /// How many times it scrolls across.
        passes: u32,
    },
}

/// What the operator asks to show.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct AlertRequest {
    /// The message.
    pub text: String,
    /// Banner or ticker.
    pub style: AlertStyle,
    /// Top or bottom.
    pub position: AlertPosition,
    /// When it goes away by itself.
    pub end: AlertEnd,
    /// The bar's colour, any CSS colour (see-through ones too).
    #[serde(default = "default_background")]
    pub background: String,
    /// The text's colour.
    #[serde(default = "default_color")]
    pub color: String,
}

/// The usual bar: dark and a little see-through.
pub const DEFAULT_BACKGROUND: &str = "rgba(0, 0, 0, 0.82)";
/// The usual text: white.
pub const DEFAULT_COLOR: &str = "#ffffff";

fn default_background() -> String {
    DEFAULT_BACKGROUND.to_owned()
}

fn default_color() -> String {
    DEFAULT_COLOR.to_owned()
}

/// An alert on screen.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct Alert {
    /// What was asked for.
    #[serde(flatten)]
    pub request: AlertRequest,
    /// When it appeared (milliseconds since the Unix epoch), so every screen
    /// scrolls it in step and knows when it ends.
    #[ts(type = "number")]
    pub started_at: Timestamp,
}

impl Alert {
    /// Whether the alert has run its time at `now`. Pass-limited tickers
    /// are ended by the renderer instead, so they report `false` here.
    pub fn has_expired(&self, now: Timestamp) -> bool {
        match self.request.end {
            AlertEnd::AfterSeconds { seconds } => {
                now >= self.started_at + i64::from(seconds) * 1000
            }
            AlertEnd::Manual | AlertEnd::AfterPasses { .. } => false,
        }
    }
}

/// A saved alert the operator can show in one click. `___` in the text is
/// replaced with what the operator types (a child number, a plate).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct AlertTemplate {
    /// Button label, e.g. "Nursery".
    pub name: String,
    /// The alert, with `___` where the operator's input goes.
    #[serde(flatten)]
    pub request: AlertRequest,
}

/// The templates a new installation starts with.
pub fn starter_templates() -> Vec<AlertTemplate> {
    let ticker = |name: &str, text: &str| AlertTemplate {
        name: name.into(),
        request: AlertRequest {
            text: text.into(),
            style: AlertStyle::Ticker,
            position: AlertPosition::Bottom,
            end: AlertEnd::AfterPasses { passes: 3 },
            background: default_background(),
            color: default_color(),
        },
    };
    vec![
        ticker(
            "Nursery",
            "Parent of child #___, please come to the nursery.",
        ),
        ticker(
            "Car",
            "Would the owner of the car with plate ___ please move it.",
        ),
    ]
}

/// Fills a template's `___` with `value`.
pub fn fill_template(template: &AlertTemplate, value: &str) -> AlertRequest {
    AlertRequest {
        text: template.request.text.replace("___", value.trim()),
        ..template.request.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_alerts_without_colours_get_the_usual_ones() {
        let saved = r#"{"text":"Hi","style":"banner","position":"top","end":{"type":"manual"}}"#;
        let request: AlertRequest = serde_json::from_str(saved).unwrap();
        assert_eq!(request.background, DEFAULT_BACKGROUND);
        assert_eq!(request.color, DEFAULT_COLOR);
    }

    #[test]
    fn timed_alerts_expire_and_others_do_not() {
        let alert = |end| Alert {
            request: AlertRequest {
                text: "Hi".into(),
                style: AlertStyle::Banner,
                position: AlertPosition::Top,
                end,
                background: default_background(),
                color: default_color(),
            },
            started_at: 1_000,
        };
        assert!(!alert(AlertEnd::AfterSeconds { seconds: 5 }).has_expired(5_999));
        assert!(alert(AlertEnd::AfterSeconds { seconds: 5 }).has_expired(6_000));
        assert!(!alert(AlertEnd::Manual).has_expired(i64::MAX));
    }

    #[test]
    fn templates_fill_in_the_blank() {
        let nursery = &starter_templates()[0];
        assert_eq!(
            fill_template(nursery, " 42 ").text,
            "Parent of child #42, please come to the nursery."
        );
    }
}

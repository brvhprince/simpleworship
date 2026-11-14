//! Lower thirds: a name, a title or a short message in an animated strap at
//! the bottom of the screen, such as "Pastor Kwame Mensah · Senior Pastor",
//! the sermon's title, the offering's MoMo number or the church's social
//! handles.
//!
//! The operator keeps a list of them (saved with the settings) and shows one
//! with a click, on the screens, the stream or both. It sits above the
//! slides and the logo, below alerts and Black, animates in with its style,
//! and goes away when hidden or after some seconds. Every screen animates it
//! from its start time, so they stay in step and one opened later joins
//! part-way through.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::id::Id;
use crate::time::Timestamp;

/// How a lower third looks and moves in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum LowerThirdStyle {
    /// An accent bar grows, and the text slides out from it.
    #[default]
    Bar,
    /// The box is wiped on from the side.
    Wipe,
    /// The text rises, and an accent line draws under it.
    Underline,
    /// The whole card rises a little into place.
    Rise,
    /// Two blocks: the first line on the accent colour, the second below.
    Block,
    /// A full-width strap, the first line on an accent tag at its start.
    Strap,
}

/// What a lower third is for: sets the labels and examples in the editor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum LowerThirdKind {
    /// A person: their name, then their title.
    #[default]
    Name,
    /// The sermon: its title, then the series or the preacher.
    Sermon,
    /// A Bible reference.
    Scripture,
    /// The offering: the MoMo number, then the account name.
    Offering,
    /// Social media: the handle, then where to find it.
    Social,
    /// Anything else.
    Announcement,
}

/// Which outputs show a lower third.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum LowerThirdTarget {
    /// The screens in the room and the stream.
    #[default]
    Both,
    /// Only the screens in the room.
    Screens,
    /// Only the stream (OBS, vMix, NDI, the stream window).
    Stream,
}

impl LowerThirdTarget {
    /// Whether the screens in the room show it.
    pub fn on_screens(self) -> bool {
        matches!(self, Self::Both | Self::Screens)
    }

    /// Whether the stream shows it.
    pub fn on_stream(self) -> bool {
        matches!(self, Self::Both | Self::Stream)
    }
}

/// Which side of the screen a lower third sits on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum LowerThirdSide {
    /// Bottom left, where names usually go.
    #[default]
    Left,
    /// Bottom centre.
    Centre,
    /// Bottom right.
    Right,
}

/// A small picture beside a lower third's title.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum LowerThirdIcon {
    /// No picture.
    #[default]
    None,
    /// The church logo (Settings › Logo); nothing if none is set.
    Logo,
    /// Facebook.
    Facebook,
    /// YouTube.
    Youtube,
    /// Instagram.
    Instagram,
    /// A chat bubble, for WhatsApp.
    Whatsapp,
    /// A phone, for MoMo numbers and calls.
    Phone,
    /// A globe, for a website.
    Web,
    /// An envelope, for an email address.
    Email,
    /// A map pin, for an address.
    Location,
    /// An open book, for scripture.
    Bible,
    /// A cross.
    Cross,
}

/// How big a lower third is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum LowerThirdSize {
    /// Smaller, for a busy camera shot.
    Small,
    /// The usual size.
    #[default]
    Medium,
    /// Bigger, for the back of a large room.
    Large,
}

/// How fast a lower third moves in and out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum LowerThirdSpeed {
    /// Gentle, for quiet moments.
    Slow,
    /// The usual speed.
    #[default]
    Normal,
    /// Snappy.
    Fast,
}

/// The colours, font and place of a lower third.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "bindings.ts")]
pub struct LowerThirdLook {
    /// How it looks and moves.
    pub style: LowerThirdStyle,
    /// The accent (bar, line, tag), any CSS colour.
    pub accent: String,
    /// The box behind the text, any CSS colour (see-through ones too).
    pub background: String,
    /// The text.
    pub color: String,
    /// The font family, or empty for the app's default.
    pub font_family: String,
    /// Which side it sits on.
    pub side: LowerThirdSide,
    /// A picture beside the title.
    pub icon: LowerThirdIcon,
    /// How round its corners are, in pixels on a 1920 × 1080 screen (0 for
    /// square).
    pub rounding: u8,
    /// How big it is.
    pub size: LowerThirdSize,
    /// How fast it moves in and out.
    pub speed: LowerThirdSpeed,
}

impl Default for LowerThirdLook {
    /// A blue accent and white text on a dark box, readable over any picture.
    fn default() -> Self {
        Self {
            style: LowerThirdStyle::default(),
            accent: "#4c6ef5".into(),
            background: "rgba(10, 12, 18, 0.86)".into(),
            color: "#ffffff".into(),
            font_family: String::new(),
            side: LowerThirdSide::default(),
            icon: LowerThirdIcon::default(),
            rounding: 0,
            size: LowerThirdSize::default(),
            speed: LowerThirdSpeed::default(),
        }
    }
}

/// A saved lower third.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct LowerThird {
    /// Identifies it in the list.
    pub id: Id,
    /// What it is for.
    #[serde(default)]
    pub kind: LowerThirdKind,
    /// The main line, e.g. the person's name.
    pub title: String,
    /// The second line, e.g. their title, or empty for none.
    #[serde(default)]
    pub subtitle: String,
    /// How it looks.
    #[serde(default)]
    pub look: LowerThirdLook,
    /// Where it shows.
    #[serde(default)]
    pub target: LowerThirdTarget,
    /// Hides itself after this many seconds, or stays until hidden (`None`).
    #[serde(default)]
    pub hide_after_seconds: Option<u32>,
}

impl LowerThird {
    /// A new lower third of `kind`, with the usual look.
    pub fn new(kind: LowerThirdKind, title: &str, subtitle: &str) -> Self {
        Self {
            id: Id::new(),
            kind,
            title: title.into(),
            subtitle: subtitle.into(),
            look: LowerThirdLook::default(),
            target: LowerThirdTarget::Both,
            hide_after_seconds: None,
        }
    }
}

/// A lower third on screen.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct LowerThirdShown {
    /// This showing of it: new each time one is shown (even the same item,
    /// even in the same millisecond), so a timer can say exactly which
    /// showing ran out.
    pub show_id: Id,
    /// What is showing (a copy, so editing the saved one changes nothing on
    /// screen until it is shown again).
    pub item: LowerThird,
    /// When it appeared (milliseconds since the Unix epoch), so every screen
    /// animates it in step and knows when it ends.
    #[ts(type = "number")]
    pub started_at: Timestamp,
}

impl LowerThirdShown {
    /// Whether it has run its time at `now` (never, without a time).
    pub fn has_expired(&self, now: Timestamp) -> bool {
        self.item
            .hide_after_seconds
            .is_some_and(|seconds| now >= self.started_at + i64::from(seconds) * 1000)
    }
}

/// The lower thirds a new installation starts with: one of each kind, with
/// words to replace.
pub fn starter_lower_thirds() -> Vec<LowerThird> {
    let mut offering = LowerThird::new(
        LowerThirdKind::Offering,
        "MoMo: 024 000 0000",
        "Account name: Your Church",
    );
    offering.look.style = LowerThirdStyle::Block;
    offering.look.icon = LowerThirdIcon::Phone;
    let mut social = LowerThird::new(
        LowerThirdKind::Social,
        "@yourchurch",
        "Facebook · YouTube · Instagram",
    );
    social.look.style = LowerThirdStyle::Underline;
    social.look.icon = LowerThirdIcon::Facebook;
    social.target = LowerThirdTarget::Stream;
    let mut sermon = LowerThird::new(LowerThirdKind::Sermon, "Sermon title", "Series · Part 1");
    sermon.look.style = LowerThirdStyle::Wipe;
    vec![
        LowerThird::new(LowerThirdKind::Name, "Pastor's name", "Senior Pastor"),
        sermon,
        offering,
        social,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expires_only_with_a_time() {
        let mut item = LowerThird::new(LowerThirdKind::Name, "A", "B");
        let shown = |item: &LowerThird| LowerThirdShown {
            show_id: Id::new(),
            item: item.clone(),
            started_at: 1_000,
        };
        assert!(!shown(&item).has_expired(i64::MAX));
        item.hide_after_seconds = Some(8);
        assert!(!shown(&item).has_expired(8_999));
        assert!(shown(&item).has_expired(9_000));
    }

    #[test]
    fn targets_say_where_they_show() {
        assert!(LowerThirdTarget::Both.on_screens() && LowerThirdTarget::Both.on_stream());
        assert!(LowerThirdTarget::Screens.on_screens() && !LowerThirdTarget::Screens.on_stream());
        assert!(!LowerThirdTarget::Stream.on_screens() && LowerThirdTarget::Stream.on_stream());
    }

    #[test]
    fn a_saved_item_from_before_new_fields_still_loads() {
        let item: LowerThird =
            serde_json::from_str(r#"{"id":"0192f0e4-0000-7000-8000-000000000001","title":"Name"}"#)
                .unwrap();
        assert_eq!(item.look, LowerThirdLook::default());
        assert_eq!(item.target, LowerThirdTarget::Both);
        assert_eq!(item.subtitle, "");
    }

    #[test]
    fn a_look_saved_before_the_designer_keeps_its_old_shape() {
        let look: LowerThirdLook =
            serde_json::from_str(r##"{"style":"wipe","accent":"#ff0000"}"##).unwrap();
        assert_eq!(look.style, LowerThirdStyle::Wipe);
        assert_eq!(look.accent, "#ff0000");
        assert_eq!(look.icon, LowerThirdIcon::None);
        assert_eq!(look.rounding, 0);
        assert_eq!(look.size, LowerThirdSize::Medium);
        assert_eq!(look.speed, LowerThirdSpeed::Normal);
    }
}

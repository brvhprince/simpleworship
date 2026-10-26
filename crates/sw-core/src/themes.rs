//! The theme library, and deciding which theme a slide is drawn with.
//!
//! The rule, from most to least specific:
//!
//! 1. A **background** set on the schedule item replaces the theme's
//!    background (the rest of the theme still applies).
//! 2. A **theme** set on the schedule item.
//! 3. The song's own theme, chosen in the song editor.
//! 4. The **default theme** for that kind of content (songs, scripture, text).
//! 5. If none of those exist, the first theme in the library, or
//!    [`Theme::default`] for an empty library.
//!
//! A schedule item stores a *copy* of the theme it uses, so a saved schedule
//! looks the same on a computer that lacks the theme. If the library does
//! have it, the library's version wins, so later edits show up.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::id::Id;
use crate::theme::{
    Background, FooterStyle, HorizontalAlign, Outline, Placement, Shadow, TextBox, TextStyle,
    Theme, VerticalAlign,
};

/// A theme in the library.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct ThemeRecord {
    /// Unique identifier.
    pub id: Id,
    /// The theme (including its name).
    pub theme: Theme,
    /// One of the starter themes that ship with SimpleWorship.
    pub builtin: bool,
}

/// Which theme each kind of content uses by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "bindings.ts")]
pub struct ThemeDefaults {
    /// Theme for songs.
    pub song: Option<Id>,
    /// Theme for scripture.
    pub scripture: Option<Id>,
    /// Theme for free text.
    pub text: Option<Id>,
    /// Theme for presentations and slideshows. When unset, they use the
    /// free-text theme.
    #[serde(default)]
    pub presentation: Option<Id>,
}

/// The kinds of content that have their own default theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentKind {
    /// Songs.
    Song,
    /// Bible passages.
    Scripture,
    /// Free text and everything else.
    Text,
    /// Presentations and slideshows.
    Presentation,
}

/// Every theme, plus the defaults. The host sends this to the engine at
/// startup and whenever a theme changes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct ThemeLibrary {
    /// The themes, in the order the operator sees them.
    pub themes: Vec<ThemeRecord>,
    /// The default theme for each kind of content.
    pub defaults: ThemeDefaults,
}

/// How a schedule item overrides the normal look.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "bindings.ts")]
pub struct ItemLook {
    /// A theme for this item only (a copy; see the [module docs](self)).
    pub theme: Option<ThemeRecord>,
    /// A background for this item only, replacing the theme's.
    pub background: Option<Background>,
}

/// The outcome of resolving a look: the theme to draw with, and what it
/// came from, so it can be refreshed when the library changes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct Look {
    /// The library theme used, if any. `None` for an embedded copy that the
    /// library does not have, or when the library is empty.
    pub theme_id: Option<Id>,
    /// The theme, with any background override already applied.
    pub theme: Theme,
    /// The background override, kept so it can be re-applied when the
    /// theme is refreshed.
    pub background_override: Option<Background>,
}

impl From<Theme> for Look {
    /// A look that is just this theme, from no library.
    fn from(theme: Theme) -> Self {
        Self {
            theme_id: None,
            theme,
            background_override: None,
        }
    }
}

impl Look {
    /// Re-applies this look on top of a newer version of its theme.
    pub fn restyled(&self, theme: &Theme) -> Self {
        let mut theme = theme.clone();
        if let Some(background) = &self.background_override {
            theme.background = background.clone();
        }
        Self {
            theme_id: self.theme_id,
            theme,
            background_override: self.background_override.clone(),
        }
    }

    /// Applies a background override, if there is one.
    fn with_background(mut self, background: Option<Background>) -> Self {
        if let Some(background) = &background {
            self.theme.background = background.clone();
        }
        self.background_override = background;
        self
    }
}

impl ThemeLibrary {
    /// The theme with `id`, if the library has it.
    pub fn get(&self, id: Id) -> Option<&ThemeRecord> {
        self.themes.iter().find(|record| record.id == id)
    }

    /// Resolves the look of a piece of content. `own_theme` is the song's
    /// own theme, if it has one. See the [module docs](self) for the rule.
    pub fn resolve(&self, kind: ContentKind, own_theme: Option<Id>, look: &ItemLook) -> Look {
        let (theme_id, theme) = match &look.theme {
            // An item theme: the library's version if there is one.
            Some(record) => match self.get(record.id) {
                Some(library) => (Some(library.id), library.theme.clone()),
                None => (None, record.theme.clone()),
            },
            None => match own_theme
                .and_then(|id| self.get(id))
                .or_else(|| self.default_for(kind))
            {
                Some(record) => (Some(record.id), record.theme.clone()),
                None => (None, Theme::default()),
            },
        };

        Look {
            theme_id,
            theme,
            background_override: None,
        }
        .with_background(look.background.clone())
    }

    /// The default theme for `kind`, falling back to the first theme.
    fn default_for(&self, kind: ContentKind) -> Option<&ThemeRecord> {
        let id = match kind {
            ContentKind::Song => self.defaults.song,
            ContentKind::Scripture => self.defaults.scripture,
            ContentKind::Text => self.defaults.text,
            ContentKind::Presentation => self.defaults.presentation.or(self.defaults.text),
        };
        id.and_then(|id| self.get(id))
            .or_else(|| self.themes.first())
    }
}

// ── Starter themes ───────────────────────────────────────────────────────

/// Ids of the starter themes. Fixed, so that every installation shares
/// them and a schedule that uses one finds it on any computer.
pub mod starter_ids {
    /// "Default".
    pub const DEFAULT: &str = "00000000-0000-7000-8000-000000000001";
    /// "Clean".
    pub const CLEAN: &str = "00000000-0000-7000-8000-000000000002";
    /// "Scripture".
    pub const SCRIPTURE: &str = "00000000-0000-7000-8000-000000000003";
    /// "Lower Third".
    pub const LOWER_THIRD: &str = "00000000-0000-7000-8000-000000000004";
}

/// The themes a new library starts with, and the defaults that go with them.
pub fn starter_library() -> ThemeLibrary {
    let id = |text: &str| text.parse::<Id>().expect("starter ids are valid UUIDs");
    let record = |text: &str, theme: Theme| ThemeRecord {
        id: id(text),
        theme,
        builtin: true,
    };

    ThemeLibrary {
        themes: vec![
            record(starter_ids::DEFAULT, Theme::default()),
            record(starter_ids::CLEAN, clean()),
            record(starter_ids::SCRIPTURE, scripture()),
            record(starter_ids::LOWER_THIRD, lower_third()),
        ],
        defaults: ThemeDefaults {
            song: Some(id(starter_ids::DEFAULT)),
            scripture: Some(id(starter_ids::SCRIPTURE)),
            text: Some(id(starter_ids::DEFAULT)),
            presentation: None,
        },
    }
}

/// White text on black, no outline: crisp on any projector.
fn clean() -> Theme {
    Theme {
        name: "Clean".into(),
        background: Background::Color {
            color: "#000000".into(),
        },
        text: TextStyle {
            font_family: "Montserrat, \"Open Sans\", sans-serif".into(),
            outline: None,
            shadow: Some(Shadow {
                color: "rgba(0, 0, 0, 0.6)".into(),
                offset_x: 0.0,
                offset_y: 2.0,
                blur: 8.0,
            }),
            ..TextStyle::default()
        },
        ..Theme::default()
    }
}

/// A calm, readable serif for longer passages.
fn scripture() -> Theme {
    Theme {
        name: "Scripture".into(),
        background: Background::LinearGradient {
            from: "#1b2430".into(),
            to: "#0b0f14".into(),
            angle_deg: 160.0,
        },
        text: TextStyle {
            font_family: "Merriweather, Georgia, serif".into(),
            font_size: 64.0,
            font_weight: 400,
            line_height: 1.4,
            align: HorizontalAlign::Left,
            outline: None,
            ..TextStyle::default()
        },
        text_box: TextBox {
            top: 90.0,
            right: 140.0,
            bottom: 170.0,
            left: 140.0,
            ..TextBox::default()
        },
        ..Theme::default()
    }
}

/// Text in the bottom third, for a live stream or a camera picture.
fn lower_third() -> Theme {
    Theme {
        name: "Lower Third".into(),
        background: Background::Color {
            color: "#000000".into(),
        },
        text: TextStyle {
            font_family: "Lato, \"Open Sans\", sans-serif".into(),
            font_size: 56.0,
            font_weight: 700,
            outline: Some(Outline {
                color: "#000000".into(),
                width: 3.0,
            }),
            ..TextStyle::default()
        },
        text_box: TextBox {
            top: 760.0,
            right: 120.0,
            bottom: 80.0,
            left: 120.0,
            vertical_align: VerticalAlign::Bottom,
            auto_fit: true,
            scale_up: false,
        },
        // The text sits low already; the reference follows it.
        caption: FooterStyle {
            placement: Placement::AfterText,
            ..FooterStyle::caption()
        },
        ..Theme::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library() -> ThemeLibrary {
        starter_library()
    }

    fn name(look: &Look) -> &str {
        &look.theme.name
    }

    #[test]
    fn defaults_apply_per_kind() {
        let library = library();
        let none = ItemLook::default();
        assert_eq!(
            name(&library.resolve(ContentKind::Song, None, &none)),
            "Default"
        );
        assert_eq!(
            name(&library.resolve(ContentKind::Scripture, None, &none)),
            "Scripture"
        );
    }

    #[test]
    fn more_specific_choices_win() {
        let library = library();
        let clean = library.themes[1].clone();
        let lower = library.themes[3].clone();

        // The song's own theme beats the default.
        let look = library.resolve(ContentKind::Song, Some(clean.id), &ItemLook::default());
        assert_eq!(name(&look), "Clean");

        // An item theme beats the song's own theme.
        let item = ItemLook {
            theme: Some(lower.clone()),
            background: None,
        };
        assert_eq!(
            name(&library.resolve(ContentKind::Song, Some(clean.id), &item)),
            "Lower Third"
        );

        // An item background replaces just the background.
        let background = Background::Color {
            color: "#ff0000".into(),
        };
        let item = ItemLook {
            theme: Some(lower),
            background: Some(background.clone()),
        };
        let look = library.resolve(ContentKind::Song, None, &item);
        assert_eq!(name(&look), "Lower Third");
        assert_eq!(look.theme.background, background);
    }

    #[test]
    fn the_library_version_of_an_item_theme_wins() {
        let mut library = library();
        let stale_copy = library.themes[1].clone();
        library.themes[1].theme.text.font_size = 99.0;

        let item = ItemLook {
            theme: Some(stale_copy),
            background: None,
        };
        let look = library.resolve(ContentKind::Song, None, &item);
        assert_eq!(look.theme.text.font_size, 99.0);
    }

    #[test]
    fn an_unknown_item_theme_uses_its_own_copy() {
        let mut copy = library().themes[1].clone();
        copy.id = Id::new();
        copy.theme.name = "From another computer".into();

        let item = ItemLook {
            theme: Some(copy),
            background: None,
        };
        let look = library().resolve(ContentKind::Song, None, &item);
        assert_eq!(name(&look), "From another computer");
        assert_eq!(look.theme_id, None);
    }

    #[test]
    fn an_empty_library_still_resolves() {
        let look = ThemeLibrary::default().resolve(ContentKind::Text, None, &ItemLook::default());
        assert_eq!(look.theme, Theme::default());
    }

    #[test]
    fn old_themes_missing_new_fields_still_load() {
        // A theme saved before `footer` existed.
        let json = r##"{"name":"Old","background":{"type":"color","color":"#000"}}"##;
        let theme: Theme = serde_json::from_str(json).unwrap();
        assert_eq!(theme.name, "Old");
        assert_eq!(theme.footer, FooterStyle::default());
    }
}

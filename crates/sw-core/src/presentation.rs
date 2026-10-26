//! Presentations: a ready-to-show list of slides with a theme.
//!
//! Anything the operator can put on screen (a song, a block of typed text and,
//! in later milestones, scripture, images or PDF pages) is first turned into
//! a [`Presentation`]. From then on the engine does not care where the slides
//! came from.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use sw_bible::Passage;

use crate::countdown::CountdownSlide;
use crate::deck::{AutoAdvance, SlideDeck, SlideStyle};
use crate::id::Id;
use crate::markup;
use crate::media::{CameraRef, MediaItem, MediaKind};
use crate::output::RenderFrame;
use crate::scripture::{self, ScriptureOptions, TranslationText};
use crate::slides::{self, Slide, SlideOptions};
use crate::song::Song;
use crate::theme::{Background, MediaFit, Theme};
use crate::themes::{Look, ThemeLibrary};

/// Where a presentation's slides came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "bindings.ts")]
pub enum PresentationSource {
    /// A song from the library.
    Song {
        /// The song's id.
        song_id: Id,
    },
    /// A countdown.
    Countdown,
    /// The pages of a presentation (an imported PDF or PowerPoint), from a
    /// schedule saved before decks could be edited.
    Slides,
    /// A slide deck: a presentation, imported PDF or slideshow.
    Deck {
        /// The deck's id.
        deck_id: Id,
    },
    /// A background-music playlist (it has no slides of its own to show).
    Playlist,
    /// Free text typed by the operator.
    Text,
    /// A Bible passage.
    Scripture {
        /// The passage.
        passage: Passage,
    },
    /// An image or video from the media library, shown on its own.
    Media {
        /// The media item's id.
        media_id: Id,
    },
    /// A camera, shown on its own.
    Camera,
}

/// A titled list of slides, styled with one theme.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct Presentation {
    /// Identifies this particular presentation instance.
    pub id: Id,
    /// Where the slides came from.
    pub source: PresentationSource,
    /// Title shown in the operator UI, e.g. the song title.
    pub title: String,
    /// The slides, in the order they are shown.
    pub slides: Vec<Slide>,
    /// The theme every slide is drawn with (with any background override
    /// already applied).
    pub theme: Theme,
    /// The library theme `theme` came from, so edits to it can be shown
    /// straight away. `None` if it did not come from the library.
    pub theme_id: Option<Id>,
    /// A background chosen for this item only, re-applied when the theme
    /// is refreshed.
    pub background_override: Option<Background>,
    /// Small print shown under the lyrics, e.g. copyright and CCLI number.
    pub footer: Option<String>,
    /// Move to the next slide by itself (the operator window keeps time).
    #[serde(default)]
    pub auto_advance: Option<AutoAdvance>,
}

impl Presentation {
    /// Builds a presentation from a song, following its arrangement.
    pub fn from_song(song: &Song, look: impl Into<Look>, options: SlideOptions) -> Self {
        Self::new(
            PresentationSource::Song { song_id: song.id },
            song.title.clone(),
            slides::slides_from_sections(&song.ordered_sections(), options),
            look.into(),
            song.footer(),
        )
    }

    /// Builds a presentation from free text. Blank lines separate slides.
    pub fn from_text(title: impl Into<String>, text: &str, look: impl Into<Look>) -> Self {
        Self::new(
            PresentationSource::Text,
            title.into(),
            slides::slides_from_text(text, SlideOptions::default()),
            look.into(),
            None,
        )
    }

    /// Builds a one-slide presentation that shows an image or video on its
    /// own, fitted to the screen with black bars rather than cropped.
    pub fn from_media(media: &MediaItem, look: impl Into<Look>) -> Self {
        let mut look: Look = look.into();
        if let Some(background) = shown_on_its_own(media) {
            // Black around the picture, whatever the theme says.
            look.background_override = Some(background.clone());
            look.theme.background = background;
        }
        Self::new(
            PresentationSource::Media { media_id: media.id },
            media.name.clone(),
            vec![Slide::text("")],
            look,
            None,
        )
    }

    /// Builds a one-slide presentation of a camera's live picture, filling
    /// the screen.
    pub fn from_camera(camera: &CameraRef, look: impl Into<Look>) -> Self {
        let mut look: Look = look.into();
        let background = Background::Camera {
            camera: camera.clone(),
            fit: MediaFit::Cover,
            opacity: 1.0,
        };
        look.background_override = Some(background.clone());
        look.theme.background = background;
        Self::new(
            PresentationSource::Camera,
            camera.label.clone(),
            vec![Slide::text("")],
            look,
            None,
        )
    }

    /// Builds a one-slide countdown, e.g. "Service starts in 4:59".
    pub fn from_countdown(
        title: impl Into<String>,
        countdown: CountdownSlide,
        look: impl Into<Look>,
    ) -> Self {
        let slide = Slide {
            countdown: Some(countdown.clone()),
            ..Slide::text(countdown.message)
        };
        Self::new(
            PresentationSource::Countdown,
            title.into(),
            vec![slide],
            look.into(),
            None,
        )
    }

    /// Builds a presentation from imported pages, one picture per slide.
    pub fn from_pages(name: impl Into<String>, pages: &[MediaItem], look: impl Into<Look>) -> Self {
        let slides = pages
            .iter()
            .map(|page| Slide {
                background: shown_on_its_own(page),
                ..Slide::text("")
            })
            .collect();
        Self::new(
            PresentationSource::Slides,
            name.into(),
            slides,
            look.into(),
            None,
        )
    }

    /// Builds a presentation from a slide deck. `themes` provides the
    /// themes that single slides use instead of the deck's.
    pub fn from_deck(deck: &SlideDeck, look: impl Into<Look>, themes: &ThemeLibrary) -> Self {
        let slides = deck
            .slides
            .iter()
            .map(|slide| {
                let runs = markup::parse(&slide.text);
                let elements = (!slide.elements.is_empty()).then(|| slide.elements.clone());
                Slide {
                    text: slide.plain_text(),
                    elements,
                    rich: markup::has_styling(&runs).then(|| runs.clone()),
                    background: slide.background.clone(),
                    look: slide.theme_id.and_then(|id| themes.get(id)).map(|record| {
                        Box::new(Look {
                            theme_id: Some(record.id),
                            theme: record.theme.clone(),
                            background_override: None,
                        })
                    }),
                    style: (!slide.style.is_empty()).then(|| slide.style.clone()),
                    ..Slide::text("")
                }
            })
            .collect();
        Self {
            auto_advance: deck.auto_advance,
            ..Self::new(
                PresentationSource::Deck { deck_id: deck.id },
                deck.name.clone(),
                slides,
                look.into(),
                None,
            )
        }
    }

    /// Builds the preview of a playlist: one slide naming its tracks. A
    /// playlist does not change the screen; sending it live plays it.
    pub fn from_playlist(
        name: impl Into<String>,
        tracks: &[MediaItem],
        look: impl Into<Look>,
    ) -> Self {
        let list = tracks
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let slide = Slide {
            caption: Some(format!("{} tracks", tracks.len())),
            ..Slide::text(list)
        };
        Self::new(
            PresentationSource::Playlist,
            name.into(),
            vec![slide],
            look.into(),
            None,
        )
    }

    fn new(
        source: PresentationSource,
        title: String,
        slides: Vec<Slide>,
        look: Look,
        footer: Option<String>,
    ) -> Self {
        Self {
            id: Id::new(),
            source,
            title,
            slides,
            theme: look.theme,
            theme_id: look.theme_id,
            background_override: look.background_override,
            footer,
            auto_advance: None,
        }
    }

    /// The look this presentation was built with.
    pub fn look(&self) -> Look {
        Look {
            theme_id: self.theme_id,
            theme: self.theme.clone(),
            background_override: self.background_override.clone(),
        }
    }

    /// Picks up newer versions of the library themes this presentation
    /// uses (its own, and any single slide's). Returns whether anything
    /// changed.
    pub fn restyle(&mut self, library: &ThemeLibrary) -> bool {
        let mut changed = false;
        if let Some(record) = self.theme_id.and_then(|id| library.get(id)) {
            let look = self.look().restyled(&record.theme);
            if look.theme != self.theme {
                self.theme = look.theme;
                changed = true;
            }
        }
        for slide in &mut self.slides {
            let Some(look) = &mut slide.look else {
                continue;
            };
            if let Some(record) = look.theme_id.and_then(|id| library.get(id))
                && record.theme != look.theme
            {
                **look = look.restyled(&record.theme);
                changed = true;
            }
        }
        changed
    }

    /// Records each picture's and video's own fit from `fits` in every
    /// background the slides can show (see [`Background::apply_file_fits`]).
    pub fn apply_file_fits(&mut self, fits: &crate::FileFits) {
        let mut backgrounds: Vec<&mut Background> = vec![&mut self.theme.background];
        backgrounds.extend(self.background_override.as_mut());
        for slide in &mut self.slides {
            backgrounds.extend(slide.background.as_mut());
            if let Some(look) = &mut slide.look {
                let look = &mut **look;
                backgrounds.push(&mut look.theme.background);
                backgrounds.extend(look.background_override.as_mut());
            }
        }
        for background in backgrounds {
            background.apply_file_fits(fits);
        }
    }

    /// Builds a presentation from a Bible passage in one or more
    /// translations (shown side by side).
    pub fn from_scripture(
        passage: &Passage,
        translations: &[TranslationText],
        options: ScriptureOptions,
        look: impl Into<Look>,
    ) -> Self {
        Self::new(
            PresentationSource::Scripture {
                passage: passage.clone(),
            },
            scripture::title(passage, translations),
            scripture::slides(passage, translations, options),
            look.into(),
            scripture::footer(translations),
        )
    }

    /// The fully resolved frame for the slide at `index`, if it exists.
    ///
    /// The theme is the slide's own if it has one, then this item's
    /// background override, then the slide's own background, then the
    /// slide's text adjustments. Mirrored in `apps/desktop/src/lib/frames.ts`.
    pub fn frame(&self, index: usize) -> Option<RenderFrame> {
        let slide = self.slides.get(index)?;
        let mut theme = match &slide.look {
            Some(look) => look.theme.clone(),
            None => self.theme.clone(),
        };
        if let Some(background) = &self.background_override {
            theme.background = background.clone();
        }
        if let Some(background) = &slide.background {
            theme.background = background.clone();
        }
        if let Some(style) = &slide.style {
            apply_style(&mut theme, style);
        }

        // The reference can be shown on the first slide of a passage only.
        let caption = if slide.scripture.is_some()
            && index > 0
            && !theme.scripture.reference_on_every_slide
        {
            None
        } else {
            slide.caption.clone()
        };

        Some(RenderFrame {
            text: slide.text.clone(),
            scripture: slide.scripture.clone(),
            caption,
            rich: slide.rich.clone(),
            elements: slide.elements.clone(),
            // The engine fills this in for a live countdown (it knows when
            // the countdown started).
            countdown: None,
            footer: self.footer.clone(),
            theme,
        })
    }

    /// Index of the first slide of the section a shortcut such as `"V2"` or
    /// `"C"` refers to.
    pub fn find_section(&self, shortcut: &str) -> Option<usize> {
        let wanted = crate::lyrics::SectionLabel::parse(shortcut);
        self.slides.iter().position(|slide| {
            slide.starts_section && slide.section.as_ref().is_some_and(|l| l.matches(&wanted))
        })
    }
}

/// Applies one slide's text adjustments to its theme.
fn apply_style(theme: &mut Theme, style: &SlideStyle) {
    if let Some(size) = style.font_size {
        theme.text.font_size = size;
    }
    if let Some(color) = &style.color {
        theme.text.color = color.clone();
    }
    if let Some(align) = style.align {
        theme.text.align = align;
    }
}

/// How an image or video looks shown on its own: the whole picture, with
/// black bars if needed, unless the file's own scaling says otherwise.
/// Sound files have no picture.
pub(crate) fn shown_on_its_own(media: &MediaItem) -> Option<Background> {
    match media.kind {
        MediaKind::Image => Some(Background::Image {
            media: media.as_ref(),
            fit: MediaFit::Contain,
            opacity: 1.0,
            // The engine records the file's own fit (see `Engine::finish`).
            file_fit: None,
        }),
        MediaKind::Video => Some(Background::Video {
            media: media.as_ref(),
            fit: MediaFit::Contain,
            looping: false,
            play_audio: true,
            opacity: 1.0,
            // The engine records the file's own fit (see `Engine::finish`).
            file_fit: None,
        }),
        MediaKind::Audio => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_camera_fills_the_screen_with_no_text() {
        let camera = CameraRef {
            device_id: "abc".into(),
            label: "FaceTime HD Camera".into(),
        };
        let presentation = Presentation::from_camera(&camera, Theme::default());
        assert_eq!(presentation.title, "FaceTime HD Camera");
        assert_eq!(presentation.slides.len(), 1);
        let frame = presentation.frame(0).expect("one slide");
        assert_eq!(frame.text, "");
        assert!(matches!(
            frame.theme.background,
            Background::Camera { ref camera, fit: MediaFit::Cover, .. } if camera.label == "FaceTime HD Camera"
        ));
    }

    fn song() -> Song {
        let mut song = Song::new("Test Song");
        song.lyrics = "[Verse 1]\nOne\n\nOne b\n[Chorus]\nRefrain\n[Verse 2]\nTwo".into();
        song.copyright = "Public Domain".into();
        song
    }

    #[test]
    fn song_presentations_carry_slides_and_footer() {
        let p = Presentation::from_song(&song(), Theme::default(), SlideOptions::default());

        assert_eq!(p.title, "Test Song");
        assert_eq!(p.slides.len(), 4);
        assert_eq!(p.footer.as_deref(), Some("Public Domain"));
        assert_eq!(p.frame(1).unwrap().text, "One b");
        assert_eq!(p.frame(99), None);
    }

    #[test]
    fn finds_sections_by_shortcut() {
        let p = Presentation::from_song(&song(), Theme::default(), SlideOptions::default());

        assert_eq!(p.find_section("V1"), Some(0));
        assert_eq!(p.find_section("c"), Some(2));
        assert_eq!(p.find_section("V2"), Some(3));
        assert_eq!(
            p.find_section("V"),
            Some(0),
            "unnumbered picks the first verse"
        );
        assert_eq!(p.find_section("B"), None);
    }

    fn library_with(theme: Theme) -> (ThemeLibrary, Id) {
        let mut library = crate::themes::starter_library();
        let id = Id::new();
        library.themes.push(crate::themes::ThemeRecord {
            id,
            theme,
            builtin: false,
        });
        (library, id)
    }

    #[test]
    fn deck_slides_carry_styling_their_own_theme_and_adjustments() {
        let (library, red_id) = library_with(Theme {
            name: "Red".into(),
            ..Theme::default()
        });
        let mut deck = SlideDeck::new("Notices");
        let mut special = crate::deck::DeckSlide::text("**Harvest** Sunday");
        special.theme_id = Some(red_id);
        special.style.font_size = Some(120.0);
        deck.slides = vec![crate::deck::DeckSlide::text("Plain"), special];

        let presentation = Presentation::from_deck(&deck, Theme::default(), &library);
        let plain = presentation.frame(0).unwrap();
        assert_eq!(plain.rich, None, "no styling, no runs");
        assert_eq!(plain.theme.name, "Default");

        let styled = presentation.frame(1).unwrap();
        assert_eq!(styled.text, "Harvest Sunday");
        assert!(styled.rich.unwrap()[0].bold);
        assert_eq!(styled.theme.name, "Red");
        assert_eq!(styled.theme.text.font_size, 120.0);
    }

    #[test]
    fn slide_themes_follow_library_edits() {
        let (mut library, id) = library_with(Theme::default());
        let mut deck = SlideDeck::new("Notices");
        let mut slide = crate::deck::DeckSlide::text("Hi");
        slide.theme_id = Some(id);
        deck.slides = vec![slide];
        let mut presentation = Presentation::from_deck(&deck, Theme::default(), &library);

        library.themes.last_mut().unwrap().theme.text.color = "#ff0000".into();
        assert!(presentation.restyle(&library));
        assert_eq!(presentation.frame(0).unwrap().theme.text.color, "#ff0000");
    }

    #[test]
    fn the_reference_can_be_shown_on_the_first_slide_only() {
        let mut theme = Theme::default();
        theme.scripture.reference_on_every_slide = false;
        let scripture = |caption: &str| Slide {
            caption: Some(caption.into()),
            scripture: Some(crate::scripture::ScriptureSlide {
                columns: Vec::new(),
                verse_numbers: None,
            }),
            ..Slide::text("")
        };
        let mut presentation = Presentation::from_text("x", "a", theme);
        presentation.slides = vec![scripture("John 3:16"), scripture("John 3:17")];

        assert_eq!(
            presentation.frame(0).unwrap().caption.as_deref(),
            Some("John 3:16")
        );
        assert_eq!(presentation.frame(1).unwrap().caption, None);
    }
}

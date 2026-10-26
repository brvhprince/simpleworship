//! What a joined computer may ask this one (the host) to do: run one of
//! the operator window's commands, by name, with the same arguments the
//! window would give it (see `link.rs` on the joined computer, and
//! `apps/desktop/src/lib/api.ts`).
//!
//! Only the commands an operator needs during a service are here: finding
//! and showing songs, scripture, media and presentations, adding them to
//! the schedule, editing songs and themes. Anything else (importing,
//! deleting media, managing Bibles, files on this computer) is refused.
//!
//! The command runs as the joined computer's operator, so whatever it puts
//! in a preview goes to that computer's own preview pane (see
//! `commands::engine::as_operator`).

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use tauri::{AppHandle, Manager};

use crate::commands::{media, presentations, schedule, scripture, settings, songs, text, themes};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Runs `method` (a command's name, e.g. `"songs_search"`) with `params`
/// (its arguments by the names the window uses, e.g. `{ "query": "grace" }`).
pub fn run(app: &AppHandle, method: &str, params: Value) -> AppResult<Value> {
    let params = Params(match params {
        Value::Object(map) => map,
        Value::Null => Map::new(),
        _ => return Err(AppError::Conversion("arguments must be an object".into())),
    });
    let app = app.clone();
    let state = || app.state::<AppState>();

    let value = match method {
        // Songs
        "songs_search" => json(songs::songs_search(state(), params.get("query")?)?),
        "song_get" => json(songs::song_get(state(), params.get("id")?)?),
        "song_save" => json(songs::song_save(app.clone(), state(), params.get("song")?)?),
        "song_presentation" => json(songs::song_presentation(state(), params.get("song")?)),
        "schedule_item_thumbnail" => json(schedule::schedule_item_thumbnail(
            state(),
            params.get("id")?,
        )),
        "song_show_in_preview" => json(songs::song_show_in_preview(
            app.clone(),
            state(),
            params.get("id")?,
        )?),
        "schedule_add_song" => json(schedule::schedule_add_song(
            app.clone(),
            state(),
            params.get("id")?,
            params.get("index")?,
        )?),

        // Scripture
        "bibles_list" => json(scripture::bibles_list(state())?),
        "bible_outline" => json(scripture::bible_outline(state(), params.get("id")?)?),
        "bible_chapter" => json(scripture::bible_chapter(
            state(),
            params.get("id")?,
            params.get("book")?,
            params.get("chapter")?,
        )?),
        "bible_search" => json(scripture::bible_search(
            state(),
            params.get("id")?,
            params.get("query")?,
        )?),
        "scripture_parse" => json(scripture::scripture_parse(params.get("text")?)),
        "scripture_text" => json(scripture::scripture_text(
            state(),
            params.get("passage")?,
            params.get("bibleId")?,
        )?),
        "scripture_show_in_preview" => json(scripture::scripture_show_in_preview(
            app.clone(),
            state(),
            params.get("passage")?,
            params.get("bibleIds")?,
            params.get("options")?,
        )?),
        "schedule_add_scripture" => json(scripture::schedule_add_scripture(
            app.clone(),
            state(),
            params.get("passage")?,
            params.get("bibleIds")?,
            params.get("options")?,
            params.get("index")?,
        )?),

        // Text and countdowns
        "text_show_in_preview" => json(text::text_show_in_preview(
            app.clone(),
            state(),
            params.get("title")?,
            params.get("text")?,
        )?),
        "schedule_add_text" => json(schedule::schedule_add_text(
            app.clone(),
            state(),
            params.get("title")?,
            params.get("text")?,
            params.get("index")?,
        )?),
        "schedule_add_countdown" => json(schedule::schedule_add_countdown(
            app.clone(),
            state(),
            params.get("title")?,
            params.get("countdown")?,
            params.get("index")?,
        )?),

        // Media
        "media_list" => json(media::media_list(state())?),
        "media_show_in_preview" => json(media::media_show_in_preview(
            app.clone(),
            state(),
            params.get("id")?,
        )?),
        "schedule_add_media" => json(media::schedule_add_media(
            app.clone(),
            state(),
            params.get("id")?,
            params.get("index")?,
        )?),
        "schedule_add_slideshow" => json(presentations::schedule_add_slideshow(
            app.clone(),
            state(),
            params.get("mediaIds")?,
            params.get("index")?,
        )?),
        "schedule_add_playlist" => json(schedule::schedule_add_playlist(
            app.clone(),
            state(),
            params.get("name")?,
            params.get("trackIds")?,
            params.get("index")?,
        )?),
        "schedule_append_media" => json(presentations::schedule_append_media(
            app.clone(),
            state(),
            params.get("id")?,
            params.get("mediaIds")?,
        )?),
        "logo_set" => json(themes::logo_set(
            app.clone(),
            state(),
            params.get("mediaId")?,
        )?),

        // Presentations
        "presentations_list" => json(presentations::presentations_list(state())?),
        "presentation_get" => json(presentations::presentation_get(state(), params.get("id")?)?),
        "presentation_show_in_preview" => json(presentations::presentation_show_in_preview(
            app.clone(),
            state(),
            params.get("id")?,
        )?),
        "schedule_add_presentation" => json(presentations::schedule_add_presentation(
            app.clone(),
            state(),
            params.get("id")?,
            params.get("index")?,
        )?),
        "schedule_append_presentation" => json(presentations::schedule_append_presentation(
            app.clone(),
            state(),
            params.get("id")?,
            params.get("deckId")?,
        )?),
        "deck_presentation" => json(presentations::deck_presentation(
            state(),
            params.get("deck")?,
        )),

        // Themes and settings shared by everyone running the service
        "theme_save" => json(themes::theme_save(
            app.clone(),
            state(),
            params.get("theme")?,
        )?),
        "theme_delete" => json(themes::theme_delete(
            app.clone(),
            state(),
            params.get("id")?,
        )?),
        "theme_set_defaults" => json(themes::theme_set_defaults(
            app.clone(),
            state(),
            params.get("defaults")?,
        )?),
        "fonts_list" => json(tauri::async_runtime::block_on(themes::fonts_list())),
        "settings_get" => json(settings::settings_get(state())?),
        "schedule_file_status" => json(schedule::schedule_file_status(state())),
        "transitions_save" => json(settings::transitions_save(
            app.clone(),
            state(),
            params.get("transitions")?,
        )?),
        "stream_save" => json(settings::stream_save(
            app.clone(),
            state(),
            params.get("settings")?,
        )?),

        _ => {
            return Err(AppError::Conversion(format!(
                "{method} is not available from another computer"
            )));
        }
    };
    Ok(value)
}

/// A command's arguments, by name.
struct Params(Map<String, Value>);

impl Params {
    /// The argument `name`. A missing argument counts as `null` (for the
    /// optional ones, such as `index`).
    fn get<T: DeserializeOwned>(&self, name: &str) -> AppResult<T> {
        let value = self.0.get(name).cloned().unwrap_or(Value::Null);
        serde_json::from_value(value)
            .map_err(|error| AppError::Conversion(format!("argument {name}: {error}")))
    }
}

fn json(value: impl serde::Serialize) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

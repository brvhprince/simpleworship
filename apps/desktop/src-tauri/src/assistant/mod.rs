//! The assistant: optional help from an AI service, with the church's own
//! API key: Claude (Anthropic), ChatGPT (OpenAI) or Gemini (Google). See
//! [`providers`] for how each is talked to.
//!
//! It is off until the operator turns it on in Settings and gives a key,
//! which is kept in the computer's keychain (macOS Keychain, Windows
//! Credential Manager, the Secret Service on Linux), never in the
//! library. Nothing is sent unless the operator clicks one of its buttons,
//! and then only what that button needs:
//!
//! - [`assistant_find_songs`]: songs that fit a description ("the song
//!   about the blood that never loses its power"). Only the description is
//!   sent; the window looks the ideas up in the library itself.
//! - [`assistant_find_verses`]: passages on a theme ("God's faithfulness
//!   in trouble").
//! - [`assistant_tidy_lyrics`]: pasted lyrics, set out in sections.
//! - [`assistant_translate`]: a song's lyrics in another language, as a
//!   draft to correct.
//! - [`assistant_sermon_slides`]: a preacher's notes as slides and
//!   scripture for the schedule.
//!
//! Every task has a fixed shape for its answer (a JSON schema), which each
//! service is made to follow, so the answer can be read reliably.

mod providers;

use std::collections::HashMap;
use std::sync::Mutex;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Settings key for whether the assistant is on, and with what.
const SETTINGS_KEY: &str = "assistant";
/// Settings key (followed by the service) for the models chosen for it.
const MODELS_KEY: &str = "assistant-models-";
/// Where keys are kept in the keychain (one account per service).
const KEYCHAIN_SERVICE: &str = "SimpleWorship";

/// The AI service.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export, export_to = "desktop.ts")]
pub enum AssistantProvider {
    /// Claude, from Anthropic.
    #[default]
    #[serde(rename = "anthropic")]
    Anthropic,
    /// ChatGPT, from OpenAI.
    #[serde(rename = "openai")]
    OpenAi,
    /// Gemini, from Google.
    #[serde(rename = "gemini")]
    Gemini,
}

impl AssistantProvider {
    /// Its key's name in the keychain.
    fn keychain_account(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic-api-key",
            Self::OpenAi => "openai-api-key",
            Self::Gemini => "gemini-api-key",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
            Self::Gemini => "gemini",
        }
    }
}

/// How capable (and costly) a model to use.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub enum AssistantModel {
    /// Quick and inexpensive: right for most tasks.
    #[default]
    Fast,
    /// Better writing (translations), slower and dearer.
    Better,
}

/// What the operator chose (saved).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "desktop.ts")]
pub struct AssistantSettings {
    /// The assistant's buttons are shown.
    pub enabled: bool,
    /// Which service answers.
    pub provider: AssistantProvider,
    /// Its quick model, or its better one.
    pub model: AssistantModel,
}

/// The models chosen from a service's list when its key was saved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct ChosenModels {
    /// Its newest quick model.
    pub fast: String,
    /// Its newest main model.
    pub better: String,
}

/// Whether the assistant can be used.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct AssistantStatus {
    /// What was chosen.
    pub settings: AssistantSettings,
    /// A key for the chosen service is saved on this computer.
    pub has_key: bool,
    /// The chosen service's models, once its key was checked.
    pub models: Option<ChosenModels>,
}

/// A song that might be the one described.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct SongIdea {
    /// Its usual title.
    pub title: String,
    /// A line from it that is likely in its lyrics, to find it by.
    pub line: String,
}

/// A passage on the theme asked about.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct VerseIdea {
    /// The reference, e.g. "Lamentations 3:22-23".
    pub reference: String,
    /// Why it fits, in a few words.
    pub why: String,
}

/// Lyrics set out for SimpleWorship, and any song details found in them.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct TidiedSong {
    /// The lyrics, with `[Verse 1]`-style section tags.
    pub lyrics: String,
    /// The order to sing the sections in ("V1 C V2 C B C"), if clear.
    pub arrangement: String,
    /// Details found in the pasted text (empty if none).
    pub title: String,
    pub author: String,
    pub copyright: String,
    pub ccli_number: String,
}

/// A song in another language.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct TranslatedSong {
    /// The title, translated.
    pub title: String,
    /// The lyrics, translated, with the same section tags.
    pub lyrics: String,
}

/// A language to translate into.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub enum TranslateTo {
    Twi,
    French,
    English,
}

/// One slide or scripture from a preacher's notes, in order.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", tag = "kind")]
#[ts(export, export_to = "desktop.ts")]
pub enum SermonItem {
    /// A point, as a text slide.
    Point {
        /// A short heading ("1. He sees you"). Only `kind` is required of
        /// the assistant, so a point may come with a heading alone (often
        /// the sermon title) or text alone.
        #[serde(default)]
        title: String,
        /// The words on the slide.
        #[serde(default)]
        text: String,
    },
    /// A passage the notes refer to.
    Scripture {
        /// The reference, e.g. "Romans 8:28-30". One left out is dropped.
        #[serde(default)]
        reference: String,
    },
}

/// The keys, once read from the keychain (reading one can ask the
/// operator for permission, so each is read once).
#[derive(Default)]
pub struct AssistantState {
    keys: Mutex<HashMap<AssistantProvider, String>>,
}

fn settings(app: &AppHandle) -> AssistantSettings {
    let state = app.state::<AppState>();
    let database = state.database();
    database
        .settings()
        .get::<AssistantSettings>(SETTINGS_KEY)
        .ok()
        .flatten()
        .unwrap_or_default()
}

fn chosen_models(app: &AppHandle, provider: AssistantProvider) -> Option<ChosenModels> {
    let state = app.state::<AppState>();
    let database = state.database();
    database
        .settings()
        .get::<ChosenModels>(&format!("{MODELS_KEY}{}", provider.name()))
        .ok()
        .flatten()
        // One chosen before the app knew better (0.6.0 could choose
        // Gemini's "customtools" variant) is chosen again.
        .filter(|models| {
            providers::usable(provider, &models.fast) && providers::usable(provider, &models.better)
        })
}

fn keychain(provider: AssistantProvider) -> AppResult<keyring::Entry> {
    keyring::Entry::new(KEYCHAIN_SERVICE, provider.keychain_account())
        .map_err(|error| AppError::Assistant(format!("the keychain is not available: {error}")))
}

fn keys(app: &AppHandle) -> std::sync::MutexGuard<'_, HashMap<AssistantProvider, String>> {
    app.state::<AssistantState>()
        .inner()
        .keys
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The saved key for a service, if any. Reads the keychain, so call it off
/// the main thread.
fn saved_key(app: &AppHandle, provider: AssistantProvider) -> Option<String> {
    if let Some(key) = keys(app).get(&provider) {
        return Some(key.clone());
    }
    let key = keychain(provider).ok()?.get_password().ok()?;
    keys(app).insert(provider, key.clone());
    Some(key)
}

async fn key_for(app: &AppHandle, provider: AssistantProvider) -> Option<String> {
    let reader = app.clone();
    tauri::async_runtime::spawn_blocking(move || saved_key(&reader, provider))
        .await
        .ok()
        .flatten()
}

async fn status(app: &AppHandle) -> AssistantStatus {
    let settings = settings(app);
    AssistantStatus {
        has_key: key_for(app, settings.provider).await.is_some(),
        models: chosen_models(app, settings.provider),
        settings,
    }
}

/// Whether the assistant is on, and has a key.
#[tauri::command]
pub async fn assistant_status(app: AppHandle) -> AssistantStatus {
    status(&app).await
}

/// Turns the assistant on or off, or changes its service or model.
#[tauri::command]
pub async fn assistant_configure(
    app: AppHandle,
    settings: AssistantSettings,
) -> AppResult<AssistantStatus> {
    {
        let state = app.state::<AppState>();
        let database = state.database();
        database.settings().set(SETTINGS_KEY, &settings)?;
    }
    Ok(status(&app).await)
}

/// Checks a key with the chosen service (by reading its list of models,
/// from which the models to use are chosen), then keeps it in the
/// keychain.
#[tauri::command]
pub async fn assistant_set_key(app: AppHandle, key: String) -> AppResult<AssistantStatus> {
    let key = key.trim().to_owned();
    if key.is_empty() {
        return Err(AppError::Assistant("paste the API key first".into()));
    }
    let provider = settings(&app).provider;
    let offered = providers::models(provider, &key).await?;
    let chosen = providers::choose(provider, &offered);
    log::info!(
        "assistant: {provider:?} key accepted; using {} and {}",
        chosen.fast,
        chosen.better
    );

    let saving = key.clone();
    tauri::async_runtime::spawn_blocking(move || {
        keychain(provider)?
            .set_password(&saving)
            .map_err(|error| AppError::Assistant(format!("the key could not be saved: {error}")))
    })
    .await
    .map_err(|error| AppError::Assistant(error.to_string()))??;
    keys(&app).insert(provider, key);
    {
        let state = app.state::<AppState>();
        let database = state.database();
        database
            .settings()
            .set(&format!("{MODELS_KEY}{}", provider.name()), &chosen)?;
    }
    Ok(status(&app).await)
}

/// Removes the chosen service's key from this computer.
#[tauri::command]
pub async fn assistant_forget_key(app: AppHandle) -> AppResult<AssistantStatus> {
    let provider = settings(&app).provider;
    tauri::async_runtime::spawn_blocking(move || match keychain(provider)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(AppError::Assistant(format!(
            "the key could not be removed: {error}"
        ))),
    })
    .await
    .map_err(|error| AppError::Assistant(error.to_string()))??;
    keys(&app).remove(&provider);
    Ok(status(&app).await)
}

// ── The tasks ─────────────────────────────────────────────────────────────

/// Songs that might be the one described.
#[tauri::command]
pub async fn assistant_find_songs(app: AppHandle, description: String) -> AppResult<Vec<SongIdea>> {
    #[derive(Deserialize)]
    struct Answer {
        songs: Vec<SongIdea>,
    }
    let answer: Answer = ask(
        &app,
        Task {
            system: "You help a church's projection operator find worship songs. From a \
                     description (a theme, a remembered phrase, a half-remembered title), \
                     suggest up to 8 well-known hymns and worship songs that fit, best \
                     first. Include Ghanaian gospel songs where they fit. For each give its \
                     usual title and one line that is really in its lyrics, exactly as \
                     sung, so it can be found in the church's library. Never invent songs.",
            prompt: description,
            tool: "suggest_songs",
            description: "The songs that fit the description, best first.",
            schema: json!({
                "type": "object",
                "properties": {
                    "songs": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": { "type": "string" },
                                "line": { "type": "string", "description": "A line from its lyrics." }
                            },
                            "required": ["title", "line"]
                        }
                    }
                },
                "required": ["songs"]
            }),
            max_tokens: 1200,
        },
    )
    .await?;
    Ok(answer.songs)
}

/// Passages on a theme.
#[tauri::command]
pub async fn assistant_find_verses(app: AppHandle, theme: String) -> AppResult<Vec<VerseIdea>> {
    #[derive(Deserialize)]
    struct Answer {
        passages: Vec<VerseIdea>,
    }
    let answer: Answer = ask(
        &app,
        Task {
            system: "You help a church's projection operator find Bible passages. For the \
                     theme or description given, suggest up to 8 passages that speak to it \
                     directly, best first. Use references that certainly exist, in the \
                     usual English form with the book's full name, such as \"Lamentations \
                     3:22-23\" or \"Psalm 46:1-3\": short passages, not whole chapters. For \
                     each, say in at most twelve words why it fits.",
            prompt: theme,
            tool: "suggest_passages",
            description: "The passages that fit, best first.",
            schema: json!({
                "type": "object",
                "properties": {
                    "passages": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "reference": { "type": "string" },
                                "why": { "type": "string" }
                            },
                            "required": ["reference", "why"]
                        }
                    }
                },
                "required": ["passages"]
            }),
            max_tokens: 1000,
        },
    )
    .await?;
    Ok(answer.passages)
}

/// Pasted lyrics, set out in sections.
#[tauri::command]
pub async fn assistant_tidy_lyrics(app: AppHandle, text: String) -> AppResult<TidiedSong> {
    ask(
        &app,
        Task {
            system: "You prepare song lyrics for SimpleWorship, a church projection program. \
                     Set out the pasted text as sections, each starting with a tag on its own \
                     line: [Verse 1], [Verse 2]…, [Pre-Chorus], [Chorus], [Bridge], [Tag], \
                     [Intro] or [Ending]. Write each section once, even if it is sung many \
                     times, and give the order it is sung in as an arrangement such as \"V1 C \
                     V2 C B C\" (V, P, C, B, T, I, E with numbers), or an empty string if \
                     the order is not clear. A blank line inside a section starts a new \
                     slide: keep slides to four lines or fewer. Remove chords, chord charts, \
                     repeat marks (\"x2\", \"Repeat\"), page headers, copyright footers and \
                     anything that is not sung. Keep the words exactly as written, only \
                     tidying capital letters and spacing. If the text gives the title, \
                     writer, copyright or CCLI song number, return them; otherwise return \
                     empty strings.",
            prompt: text,
            tool: "song",
            description: "The lyrics set out in tagged sections, and any song details.",
            schema: json!({
                "type": "object",
                "properties": {
                    "lyrics": { "type": "string" },
                    "arrangement": { "type": "string" },
                    "title": { "type": "string" },
                    "author": { "type": "string" },
                    "copyright": { "type": "string" },
                    "ccliNumber": { "type": "string" }
                },
                "required": ["lyrics", "arrangement", "title", "author", "copyright", "ccliNumber"]
            }),
            max_tokens: 4000,
        },
    )
    .await
}

/// A song's lyrics in another language: a draft for the operator to correct.
#[tauri::command]
pub async fn assistant_translate(
    app: AppHandle,
    title: String,
    lyrics: String,
    language: TranslateTo,
) -> AppResult<TranslatedSong> {
    let into = match language {
        TranslateTo::Twi => {
            "Asante Twi, as sung in Ghanaian churches, in the standard spelling (ɛ, ɔ). Use \
             the words of the Twi Bible for God and the faith (Onyankopɔn, Awurade, Yesu \
             Kristo, Honhom Kronkron, adom, nkwa)"
        }
        TranslateTo::French => "French, as sung in francophone churches",
        TranslateTo::English => "English",
    };
    ask(
        &app,
        Task {
            system: &format!(
                "You translate worship songs for a church to sing and show on screen. \
                 Translate the song into {into}. Keep its meaning and its faith exactly; \
                 make the lines natural and singable rather than word for word, with about \
                 the same number of lines. Keep every section tag ([Verse 1], [Chorus]…) \
                 exactly as it is, in English, and keep the blank lines where they are. \
                 Translate the title too."
            ),
            prompt: format!("Title: {title}\n\n{lyrics}"),
            tool: "translated_song",
            description: "The translated title and lyrics.",
            schema: json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string" },
                    "lyrics": { "type": "string" }
                },
                "required": ["title", "lyrics"]
            }),
            max_tokens: 4000,
        },
    )
    .await
}

/// A preacher's notes as slides and scripture, in order.
#[tauri::command]
pub async fn assistant_sermon_slides(app: AppHandle, notes: String) -> AppResult<Vec<SermonItem>> {
    #[derive(Deserialize)]
    struct Answer {
        items: Vec<SermonItem>,
    }
    let answer: Answer = ask(
        &app,
        Task {
            system: "You turn a preacher's sermon notes into what a church shows on screen, \
                     in the order of the sermon: the main points as text slides, and every \
                     Bible passage the notes refer to. A point has a short heading (the \
                     sermon title, or \"1. …\" for numbered points) and one to four short \
                     lines of text taken from the notes. A passage is its reference only, in \
                     the usual English form with the book's full name, such as \"Romans \
                     8:28-30\". Use only what is in the notes: add nothing, and keep the \
                     preacher's words.",
            prompt: notes,
            tool: "sermon_slides",
            description: "The slides and passages, in order.",
            schema: json!({
                "type": "object",
                "properties": {
                    "items": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "kind": { "type": "string", "enum": ["point", "scripture"] },
                                "title": { "type": "string", "description": "For a point." },
                                "text": { "type": "string", "description": "For a point." },
                                "reference": { "type": "string", "description": "For a passage." }
                            },
                            "required": ["kind"]
                        }
                    }
                },
                "required": ["items"]
            }),
            max_tokens: 4000,
        },
    )
    .await?;
    // Leave out anything with nothing to show.
    Ok(answer
        .items
        .into_iter()
        .filter(|item| match item {
            SermonItem::Point { title, text } => {
                !(title.trim().is_empty() && text.trim().is_empty())
            }
            SermonItem::Scripture { reference } => !reference.trim().is_empty(),
        })
        .collect())
}

// ── Asking ────────────────────────────────────────────────────────────────

/// One request: instructions, the operator's text, and the shape of the
/// answer (named, so services that use tools or functions can call it).
struct Task<'a> {
    system: &'a str,
    prompt: String,
    tool: &'a str,
    description: &'a str,
    schema: Value,
    max_tokens: u32,
}

/// Sends a task to the chosen service and reads the answer.
async fn ask<T: DeserializeOwned>(app: &AppHandle, task: Task<'_>) -> AppResult<T> {
    let settings = settings(app);
    if !settings.enabled {
        return Err(AppError::Assistant(
            "turn on the assistant in Settings first".into(),
        ));
    }
    let provider = settings.provider;
    let key = key_for(app, provider)
        .await
        .ok_or_else(|| AppError::Assistant("add an API key in Settings first".into()))?;
    if task.prompt.trim().is_empty() {
        return Err(AppError::Assistant("type or paste something first".into()));
    }
    let models = match chosen_models(app, provider) {
        Some(models) => models,
        None => {
            let chosen = providers::choose(provider, &providers::models(provider, &key).await?);
            // Remembered, so the list is not fetched again next time.
            let state = app.state::<AppState>();
            let saved = state
                .database()
                .settings()
                .set(&format!("{MODELS_KEY}{}", provider.name()), &chosen);
            if let Err(error) = saved {
                log::warn!("assistant: the chosen models could not be saved: {error}");
            }
            chosen
        }
    };
    let model = match settings.model {
        AssistantModel::Fast => models.fast,
        AssistantModel::Better => models.better,
    };

    let answer = providers::ask(provider, &key, &model, &task).await?;
    serde_json::from_value(answer).map_err(|error| {
        log::warn!("assistant: unexpected answer from {model}: {error}");
        AppError::Assistant("the assistant's answer could not be read; try again".into())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_sermon_items() {
        let items: Vec<SermonItem> = serde_json::from_value(json!([
            {"kind":"point","title":"1. He sees you","text":"Even in the valley"},
            {"kind":"scripture","reference":"Psalm 23:4"}
        ]))
        .unwrap();
        assert!(
            matches!(&items[1], SermonItem::Scripture { reference } if reference == "Psalm 23:4")
        );
    }

    #[test]
    fn reads_a_point_with_a_heading_alone() {
        // As gemini-3.1-pro-preview-customtools sent it: only `kind` is required.
        let items: Vec<SermonItem> = serde_json::from_value(json!([
            {"kind":"point","title":"He sees you"},
            {"kind":"point","text":"Even in the valley"},
            {"kind":"scripture"}
        ]))
        .unwrap();
        assert!(
            matches!(&items[0], SermonItem::Point { title, text } if title == "He sees you" && text.is_empty())
        );
        assert!(matches!(&items[2], SermonItem::Scripture { reference } if reference.is_empty()));
    }

    #[test]
    fn services_are_saved_by_name() {
        assert_eq!(
            serde_json::to_value(AssistantProvider::OpenAi).unwrap(),
            "openai"
        );
        let settings: AssistantSettings = serde_json::from_value(json!({"enabled": true})).unwrap();
        assert_eq!(
            settings.provider,
            AssistantProvider::Anthropic,
            "saved before there was a choice"
        );
    }
}

//! The three AI services the assistant can use, each with its own way of
//! checking a key, listing its models and answering in a fixed shape:
//!
//! | Service             | Key header           | Fixed-shape answer            |
//! | ------------------- | -------------------- | ----------------------------- |
//! | Claude (Anthropic)  | `x-api-key`          | a tool it is told to use      |
//! | ChatGPT (OpenAI)    | `Authorization`      | a function it must call       |
//! | Gemini (Google)     | `x-goog-api-key`     | JSON matching a schema        |
//!
//! Model names change every few months, so none is fixed here: when a key
//! is saved, the service's own list of models is read and the newest quick
//! one ("Fast") and newest main one ("Better") are chosen from it
//! ([`choose`]). The fallbacks are only for a list that cannot be read.

use std::time::Duration;

use serde_json::{Value, json};

use super::{AssistantProvider, ChosenModels, Task};
use crate::error::{AppError, AppResult};

/// The longest an answer may take.
/// How long to wait for an answer. Reasoning models can think for a
/// minute or more before answering, and church connections can be slow;
/// the assistant is used while preparing, not during a service.
const TIMEOUT: Duration = Duration::from_secs(180);
const ANTHROPIC: &str = "https://api.anthropic.com/v1";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const OPENAI: &str = "https://api.openai.com/v1";
const GEMINI: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Reasoning models count their thinking against the answer's length, so
/// they are given this many times what the answer itself needs.
const THINKING_ROOM: u32 = 4;

/// A model a service offers.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Offered {
    pub id: String,
    /// When it came out (any increasing number), if the service says.
    pub created: i64,
}

/// Checks the key by listing the service's models.
pub(super) async fn models(provider: AssistantProvider, key: &str) -> AppResult<Vec<Offered>> {
    let client = client()?;
    let request = match provider {
        AssistantProvider::Anthropic => client
            .get(format!("{ANTHROPIC}/models?limit=100"))
            .header("x-api-key", key)
            .header("anthropic-version", ANTHROPIC_VERSION),
        AssistantProvider::OpenAi => client.get(format!("{OPENAI}/models")).bearer_auth(key),
        AssistantProvider::Gemini => client
            .get(format!("{GEMINI}/models?pageSize=1000"))
            .header("x-goog-api-key", key),
    };
    let reply = read(provider, request).await?;

    let offered = match provider {
        // Newest first.
        AssistantProvider::Anthropic => list(&reply["data"], |model| {
            Some((model["id"].as_str()?.to_owned(), 0))
        }),
        AssistantProvider::OpenAi => list(&reply["data"], |model| {
            Some((
                model["id"].as_str()?.to_owned(),
                model["created"].as_i64().unwrap_or(0),
            ))
        }),
        AssistantProvider::Gemini => list(&reply["models"], |model| {
            let methods = model["supportedGenerationMethods"].as_array()?;
            if !methods.iter().any(|method| method == "generateContent") {
                return None;
            }
            let id = model["name"].as_str()?.trim_start_matches("models/");
            Some((id.to_owned(), 0))
        }),
    };
    // Anthropic lists the newest first: count down so it sorts that way.
    Ok(match provider {
        AssistantProvider::Anthropic => offered
            .into_iter()
            .enumerate()
            .map(|(index, model)| Offered {
                created: -(index as i64),
                ..model
            })
            .collect(),
        _ => offered,
    })
}

fn list(items: &Value, read: impl Fn(&Value) -> Option<(String, i64)>) -> Vec<Offered> {
    items
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(&read)
                .map(|(id, created)| Offered { id, created })
                .collect()
        })
        .unwrap_or_default()
}

/// The newest quick model and newest main model in a service's list.
pub(super) fn choose(provider: AssistantProvider, offered: &[Offered]) -> ChosenModels {
    let newest = |accept: &dyn Fn(&str) -> bool| -> Option<String> {
        offered
            .iter()
            .filter(|model| accept(&model.id))
            .max_by(|a, b| rank(provider, a).cmp(&rank(provider, b)))
            .map(|model| model.id.clone())
    };
    let (fast, better, fallback) = match provider {
        AssistantProvider::Anthropic => (
            newest(&|id| id.contains("haiku")),
            newest(&|id| id.contains("sonnet")),
            ("claude-haiku-4-5", "claude-sonnet-5"),
        ),
        AssistantProvider::OpenAi => (
            newest(&|id| openai_chat(id) && id.contains("-mini")),
            newest(&|id| {
                openai_chat(id) && !["-mini", "-nano", "-pro"].iter().any(|s| id.contains(s))
            }),
            ("gpt-5-mini", "gpt-5"),
        ),
        AssistantProvider::Gemini => (
            newest(&|id| gemini_tier(id) == Some("flash")),
            newest(&|id| gemini_tier(id) == Some("pro")),
            ("gemini-2.5-flash", "gemini-2.5-pro"),
        ),
    };
    ChosenModels {
        fast: fast.unwrap_or_else(|| fallback.0.into()),
        better: better.unwrap_or_else(|| fallback.1.into()),
    }
}

/// Orders a service's models from oldest to newest, preferring released
/// ones over previews of the same age.
fn rank(provider: AssistantProvider, model: &Offered) -> (Vec<u32>, i64, bool) {
    let released = !model.id.contains("preview") && !model.id.contains("exp");
    match provider {
        // Gemini's list has no dates: the version in the name tells.
        AssistantProvider::Gemini => (version(&model.id), 0, released),
        _ => (Vec::new(), model.created, released),
    }
}

/// `"gemini-3.8-flash"` → `[3, 8]`.
fn version(id: &str) -> Vec<u32> {
    id.split('-')
        .nth(1)
        .map(|part| part.split('.').filter_map(|n| n.parse().ok()).collect())
        .unwrap_or_default()
}

/// An OpenAI model for writing text (not speech, pictures or code tools),
/// and not a dated snapshot of one.
fn openai_chat(id: &str) -> bool {
    const NOT: [&str; 11] = [
        "audio",
        "realtime",
        "image",
        "transcribe",
        "tts",
        "search",
        "codex",
        "instruct",
        "oss",
        "chat-latest",
        "embedding",
    ];
    id.starts_with("gpt-") && !NOT.iter().any(|word| id.contains(word)) && !dated(id)
}

/// The tier of a plain Gemini text model, `"flash"` or `"pro"`: named
/// `gemini-<version>-<tier>`, perhaps followed by `preview`, `exp`,
/// `latest` or a date. Anything else is a variant for another job ("lite",
/// "image", "tts", "customtools", which answers in its own way), or not a
/// Gemini text model at all.
fn gemini_tier(id: &str) -> Option<&str> {
    let mut parts = id.strip_prefix("gemini-")?.split('-');
    let version = parts.next()?;
    if version.is_empty() || !version.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    let tier = parts
        .next()
        .filter(|tier| ["flash", "pro"].contains(tier))?;
    parts
        .all(|part| {
            ["preview", "exp", "latest"].contains(&part) || part.chars().all(|c| c.is_ascii_digit())
        })
        .then_some(tier)
}

/// Whether a model the app chose before is one it would still choose: a
/// plain text model (Gemini's list also offers variants for other jobs).
pub(super) fn usable(provider: AssistantProvider, id: &str) -> bool {
    match provider {
        AssistantProvider::Gemini => gemini_tier(id).is_some(),
        _ => true,
    }
}

/// Ends in a date, as snapshots do ("gpt-5-mini-2025-08-07").
fn dated(id: &str) -> bool {
    let parts: Vec<&str> = id.rsplitn(4, '-').collect();
    parts.len() == 4
        && parts[..3]
            .iter()
            .all(|part| part.chars().all(|c| c.is_ascii_digit()))
}

/// Sends a task and returns the answer, in the task's shape.
pub(super) async fn ask(
    provider: AssistantProvider,
    key: &str,
    model: &str,
    task: &Task<'_>,
) -> AppResult<Value> {
    let client = client()?;
    let request = match provider {
        // Newer Claude models do not let a request force a tool
        // ("tool_choice": "tool"), so it is offered and asked for instead;
        // an answer written as plain JSON is read too.
        AssistantProvider::Anthropic => client
            .post(format!("{ANTHROPIC}/messages"))
            .header("x-api-key", key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&json!({
                "model": model,
                "max_tokens": task.max_tokens * THINKING_ROOM,
                "system": format!(
                    "{}\n\nAlways answer by calling the {} tool, once.",
                    task.system, task.tool
                ),
                "messages": [{ "role": "user", "content": task.prompt }],
                "tools": [{
                    "name": task.tool,
                    "description": task.description,
                    "input_schema": task.schema,
                }],
                "tool_choice": { "type": "auto" },
            })),
        AssistantProvider::OpenAi => client
            .post(format!("{OPENAI}/chat/completions"))
            .bearer_auth(key)
            .json(&json!({
                "model": model,
                "max_completion_tokens": task.max_tokens * THINKING_ROOM,
                "messages": [
                    { "role": "system", "content": task.system },
                    { "role": "user", "content": task.prompt },
                ],
                "tools": [{
                    "type": "function",
                    "function": {
                        "name": task.tool,
                        "description": task.description,
                        "parameters": task.schema,
                    },
                }],
                "tool_choice": { "type": "function", "function": { "name": task.tool } },
            })),
        AssistantProvider::Gemini => {
            let gemini = |thinking: Option<Value>| {
                let mut config = json!({
                    "responseMimeType": "application/json",
                    "responseJsonSchema": task.schema,
                    "maxOutputTokens": task.max_tokens * THINKING_ROOM,
                });
                if let Some(thinking) = thinking {
                    config["thinkingConfig"] = thinking;
                }
                client
                    .post(format!("{GEMINI}/models/{model}:generateContent"))
                    .header("x-goog-api-key", key)
                    .json(&json!({
                        "systemInstruction": { "parts": [{ "text": task.system }] },
                        "contents": [{ "role": "user", "parts": [{ "text": task.prompt }] }],
                        "generationConfig": config,
                    }))
            };
            let reply = match read(provider, gemini(Some(gemini_thinking(model)))).await {
                // A model that does not take this way of setting its
                // thinking: ask again, letting it think as it likes.
                Err(AppError::Assistant(message))
                    if message.to_lowercase().contains("thinking") =>
                {
                    log::warn!("assistant: {model} refused the thinking setting: {message}");
                    read(provider, gemini(None)).await?
                }
                result => result?,
            };
            return answer(provider, &reply, task.tool);
        }
    };
    let reply = read(provider, request).await?;
    answer(provider, &reply, task.tool)
}

/// How much a Gemini model thinks before answering. Left to itself, it
/// can think for over a minute, longer than the app waits; these tasks
/// (reading notes, tidying lyrics) need little. Gemini 3 and later take a
/// level; 2.5 takes a budget in tokens (Pro cannot think less than 128).
fn gemini_thinking(model: &str) -> Value {
    match version(model).first() {
        Some(major) if *major >= 3 => json!({ "thinkingLevel": "low" }),
        _ => json!({ "thinkingBudget": 1024 }),
    }
}

/// The answer in a service's reply.
pub(super) fn answer(provider: AssistantProvider, reply: &Value, tool: &str) -> AppResult<Value> {
    let unreadable = || why_no_answer(provider, reply);
    match provider {
        AssistantProvider::Anthropic => {
            let blocks = reply["content"].as_array().ok_or_else(unreadable)?;
            if let Some(call) = blocks
                .iter()
                .find(|block| block["type"] == "tool_use" && block["name"] == tool)
            {
                return Ok(call["input"].clone());
            }
            // No tool call: perhaps the answer was written out as JSON.
            let text: String = blocks
                .iter()
                .filter(|block| block["type"] == "text")
                .filter_map(|block| block["text"].as_str())
                .collect();
            json_in(&text).ok_or_else(unreadable)
        }
        AssistantProvider::OpenAi => {
            let message = &reply["choices"][0]["message"];
            let arguments = message["tool_calls"]
                .as_array()
                .and_then(|calls| calls.iter().find(|call| call["function"]["name"] == tool))
                .and_then(|call| call["function"]["arguments"].as_str());
            match arguments {
                Some(arguments) => serde_json::from_str(arguments).map_err(|_| unreadable()),
                // No function call: perhaps the answer was written out as JSON.
                None => json_in(message["content"].as_str().unwrap_or("")).ok_or_else(unreadable),
            }
        }
        AssistantProvider::Gemini => {
            // The answer's text, leaving out any thinking it shows.
            let text: String = reply["candidates"][0]["content"]["parts"]
                .as_array()
                .ok_or_else(unreadable)?
                .iter()
                .filter(|part| part["thought"] != true)
                .filter_map(|part| part["text"].as_str())
                .collect();
            json_in(&text).ok_or_else(unreadable)
        }
    }
}

/// Why a reply holds no answer, from the reason the service gives for
/// stopping: it ran out of room (often while thinking), its filters
/// declined, or (Gemini) it would not repeat text it recognises, such as
/// published song lyrics. The reason goes to the log, so a failure on a
/// church's computer can be explained.
fn why_no_answer(provider: AssistantProvider, reply: &Value) -> AppError {
    let reason = match provider {
        AssistantProvider::Anthropic => reply["stop_reason"].as_str(),
        AssistantProvider::OpenAi => {
            let choice = &reply["choices"][0];
            if choice["message"]["refusal"].is_string() {
                Some("refusal")
            } else {
                choice["finish_reason"].as_str()
            }
        }
        AssistantProvider::Gemini => reply["promptFeedback"]["blockReason"]
            .as_str()
            .or_else(|| reply["candidates"][0]["finishReason"].as_str()),
    }
    .unwrap_or("none given");
    log::warn!("assistant: {provider:?} gave no readable answer (reason: {reason})");

    let message = match reason {
        "max_tokens" | "length" | "MAX_TOKENS" => {
            "the answer was longer than the assistant may write; try a shorter text, \
             or the other model in Settings"
                .into()
        }
        "RECITATION" => "Gemini will not repeat text it recognises, such as published song \
                         lyrics; type or paste them yourself, or use Claude or ChatGPT for this"
            .into(),
        "refusal" | "content_filter" | "SAFETY" | "PROHIBITED_CONTENT" | "BLOCKLIST" | "SPII"
        | "OTHER" => format!("the AI service declined to answer this ({reason})"),
        _ => "the assistant's answer could not be read; try again".into(),
    };
    AppError::Assistant(message)
}

/// JSON written as text, perhaps in a code block or after a sentence.
fn json_in(text: &str) -> Option<Value> {
    let text = text.trim();
    let start = text.find(['{', '['])?;
    let end = text.rfind(['}', ']'])?;
    serde_json::from_str(text.get(start..=end)?).ok()
}

/// Sends a request and reads its JSON, turning refusals into messages.
async fn read(provider: AssistantProvider, request: reqwest::RequestBuilder) -> AppResult<Value> {
    let response = request.send().await.map_err(|error| network(&error))?;
    let status = response.status().as_u16();
    let body = response.text().await.map_err(|error| network(&error))?;
    if !(200..300).contains(&status) {
        return Err(refusal(provider, status, &body));
    }
    serde_json::from_str(&body).map_err(|_| {
        let start: String = body.chars().take(200).collect();
        log::warn!("assistant: {provider:?} sent a reply that is not JSON: {start}");
        AppError::Assistant("the assistant's answer could not be read; try again".into())
    })
}

fn client() -> AppResult<reqwest::Client> {
    // The HTTPS library leaves the choice of encryption code to the app (as
    // the updater does too).
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    reqwest::Client::builder()
        .user_agent(concat!("SimpleWorship/", env!("CARGO_PKG_VERSION")))
        .timeout(TIMEOUT)
        .build()
        .map_err(|error| network(&error))
}

fn network(error: &reqwest::Error) -> AppError {
    log::warn!("assistant: {error}");
    if error.is_timeout() {
        AppError::Assistant("the assistant took too long to answer; try again".into())
    } else {
        AppError::Assistant(
            "the assistant could not be reached; check the internet connection".into(),
        )
    }
}

/// What to tell the operator when a service says no.
pub(super) fn refusal(provider: AssistantProvider, status: u16, body: &str) -> AppError {
    log::warn!("assistant: {provider:?} refused ({status}): {body}");
    let lower = body.to_lowercase();
    let message = if lower.contains("api_key_invalid") || lower.contains("api key not valid") {
        "the API key was not accepted; check it in Settings"
    } else if lower.contains("credit balance") || lower.contains("insufficient_quota") {
        "the account needs credit or billing set up with the AI service"
    } else {
        match status {
            401 | 403 => "the API key was not accepted; check it in Settings",
            402 => "the account needs credit or billing set up with the AI service",
            404 => "that model is not available to this key; choose the other model in Settings",
            429 => "the assistant is busy or the account's limit was reached; wait a moment",
            500..=599 => "the assistant is having problems; try again in a few minutes",
            // Anything else: say what the service said.
            _ => {
                let reason = serde_json::from_str::<Value>(body)
                    .ok()
                    .and_then(|reply| reply["error"]["message"].as_str().map(str::to_owned));
                return AppError::Assistant(match reason {
                    Some(reason) => format!("the assistant could not do that: {reason}"),
                    None => "the assistant could not do that".into(),
                });
            }
        }
    };
    AppError::Assistant(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offered(ids: &[(&str, i64)]) -> Vec<Offered> {
        ids.iter()
            .map(|&(id, created)| Offered {
                id: id.into(),
                created,
            })
            .collect()
    }

    #[test]
    fn chooses_the_newest_openai_models() {
        let list = offered(&[
            ("gpt-5", 1),
            ("gpt-5-mini", 1),
            ("gpt-5-mini-2025-08-07", 1),
            ("gpt-5.4-mini", 5),
            ("gpt-5.5-pro", 6),
            ("gpt-6-astra", 7),
            ("gpt-realtime-2.1", 8),
            ("gpt-image-2", 9),
        ]);
        let chosen = choose(AssistantProvider::OpenAi, &list);
        assert_eq!(chosen.fast, "gpt-5.4-mini");
        assert_eq!(chosen.better, "gpt-6-astra");
    }

    #[test]
    fn chooses_the_newest_gemini_models() {
        let list = offered(&[
            ("gemini-2.5-flash", 0),
            ("gemini-3.8-flash", 0),
            ("gemini-3.8-flash-lite", 0),
            ("gemini-3.9-flash-preview", 0),
            ("gemini-3.1-pro", 0),
            ("gemini-3.8-flash-image", 0),
            ("gemini-3.2-pro-preview-customtools", 0),
            ("gemini-3.9-flash-preview-tts", 0),
        ]);
        let chosen = choose(AssistantProvider::Gemini, &list);
        // A newer preview is still newer.
        assert_eq!(chosen.fast, "gemini-3.9-flash-preview");
        assert_eq!(chosen.better, "gemini-3.1-pro");
    }

    #[test]
    fn keeps_gemini_thinking_low() {
        assert_eq!(gemini_thinking("gemini-3.8-flash")["thinkingLevel"], "low");
        assert_eq!(
            gemini_thinking("gemini-3-pro-preview")["thinkingLevel"],
            "low"
        );
        assert_eq!(gemini_thinking("gemini-2.5-pro")["thinkingBudget"], 1024);
    }

    #[test]
    fn knows_gemini_text_models_from_variants() {
        for id in [
            "gemini-2.5-flash",
            "gemini-3.1-pro-preview",
            "gemini-2.5-flash-preview-05-20",
            "gemini-3-pro-latest",
        ] {
            assert!(usable(AssistantProvider::Gemini, id), "{id}");
        }
        for id in [
            "gemini-3.1-pro-preview-customtools",
            "gemini-2.5-flash-lite",
            "gemini-2.5-flash-image",
            "gemini-2.5-flash-preview-tts",
            "gemini-embedding-001",
            "gemini-flash-latest",
        ] {
            assert!(!usable(AssistantProvider::Gemini, id), "{id}");
        }
    }

    #[test]
    fn chooses_the_first_listed_claude_models() {
        // Anthropic lists newest first (models() turns that into ranks).
        let list = offered(&[
            ("claude-sonnet-5", 0),
            ("claude-haiku-4-5-20251001", -1),
            ("claude-sonnet-4-5", -2),
        ]);
        let chosen = choose(AssistantProvider::Anthropic, &list);
        assert_eq!(chosen.fast, "claude-haiku-4-5-20251001");
        assert_eq!(chosen.better, "claude-sonnet-5");
    }

    #[test]
    fn falls_back_when_the_list_is_empty() {
        assert_eq!(
            choose(AssistantProvider::Gemini, &[]).fast,
            "gemini-2.5-flash"
        );
    }

    #[test]
    fn reads_each_services_answer() {
        let claude = json!({"content":[{"type":"tool_use","name":"song","input":{"lyrics":"x"}}]});
        let openai = json!({"choices":[{"message":{"tool_calls":[
            {"function":{"name":"song","arguments":"{\"lyrics\":\"x\"}"}}
        ]}}]});
        let gemini = json!({"candidates":[{"content":{"parts":[
            {"text":"thinking…","thought":true},
            {"text":"```json\n{\"lyrics\":\"x\"}\n```"}
        ]}}]});
        for (provider, reply) in [
            (AssistantProvider::Anthropic, claude),
            (AssistantProvider::OpenAi, openai),
            (AssistantProvider::Gemini, gemini),
        ] {
            let answer = answer(provider, &reply, "song").unwrap();
            assert_eq!(answer["lyrics"], "x", "{provider:?}");
        }
    }

    #[test]
    fn reads_claude_answers_written_as_text() {
        let reply = json!({"content":[
            {"type":"thinking","thinking":"…"},
            {"type":"text","text":"Here it is:\n```json\n{\"lyrics\":\"x\"}\n```"}
        ]});
        assert_eq!(
            answer(AssistantProvider::Anthropic, &reply, "song").unwrap()["lyrics"],
            "x"
        );
    }

    #[test]
    fn passes_on_the_services_reason() {
        let error = refusal(
            AssistantProvider::Anthropic,
            400,
            r#"{"type":"error","error":{"type":"invalid_request_error","message":"something new"}}"#,
        );
        assert!(error.to_string().ends_with("something new"), "{error}");
    }

    #[test]
    fn a_reply_without_an_answer_is_unreadable() {
        let reply = json!({"content":[{"type":"text","text":"I can't help with that"}]});
        assert!(answer(AssistantProvider::Anthropic, &reply, "song").is_err());
    }

    #[test]
    fn says_why_a_reply_has_no_answer() {
        let cases = [
            (
                AssistantProvider::Gemini,
                json!({"candidates":[{"finishReason":"MAX_TOKENS","content":{"parts":[
                    {"text":"…","thought":true}
                ]}}]}),
                "longer than",
            ),
            (
                AssistantProvider::Gemini,
                json!({"candidates":[{"finishReason":"RECITATION"}]}),
                "will not repeat",
            ),
            (
                AssistantProvider::Gemini,
                json!({"promptFeedback":{"blockReason":"SAFETY"}}),
                "declined to answer this (SAFETY)",
            ),
            (
                AssistantProvider::OpenAi,
                json!({"choices":[{"finish_reason":"length","message":{"content":""}}]}),
                "longer than",
            ),
            (
                AssistantProvider::OpenAi,
                json!({"choices":[{"finish_reason":"stop","message":{"refusal":"No."}}]}),
                "declined",
            ),
            (
                AssistantProvider::Anthropic,
                json!({"stop_reason":"max_tokens","content":[{"type":"text","text":"{\"ly"}]}),
                "longer than",
            ),
            (
                AssistantProvider::Gemini,
                json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"text":"hi"}]}}]}),
                "could not be read",
            ),
        ];
        for (provider, reply, expected) in cases {
            let error = answer(provider, &reply, "song").unwrap_err().to_string();
            assert!(error.contains(expected), "{provider:?}: {error}");
        }
    }

    #[test]
    fn explains_refusals() {
        let gemini = refusal(
            AssistantProvider::Gemini,
            400,
            r#"{"error":{"status":"INVALID_ARGUMENT","details":[{"reason":"API_KEY_INVALID"}]}}"#,
        );
        assert!(gemini.to_string().contains("not accepted"));
        let openai = refusal(
            AssistantProvider::OpenAi,
            429,
            r#"{"error":{"code":"insufficient_quota"}}"#,
        );
        assert!(openai.to_string().contains("credit"));
        let claude = refusal(
            AssistantProvider::Anthropic,
            400,
            "Your credit balance is too low",
        );
        assert!(claude.to_string().contains("credit"));
    }

    #[test]
    fn recognises_dated_snapshots() {
        assert!(dated("gpt-5-mini-2025-08-07"));
        assert!(!dated("gpt-5.4-mini"));
    }
}

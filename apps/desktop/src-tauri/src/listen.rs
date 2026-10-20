//! Listen: hears the preacher and offers the scripture they read.
//!
//! The operator turns it on from the toolbar and chooses a microphone (the
//! mixer's feed, usually). What is said is written down on this computer
//! by Whisper ([`sw_listen`]); every reference in it ([`sw_bible::spoken`])
//! is sent to the windows as [`HEARD_EVENT`], and the Scripture tab offers
//! it. Nothing goes live by itself.
//!
//! The speech model is downloaded once, the first time it is needed, from
//! whisper.cpp's published models, and checked against its known SHA-256
//! before it is used. Three sizes are offered ([`SpeechModel`]): bigger is
//! more accurate, and slower on older computers.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sw_bible::spoken::Spotter;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Settings key for the chosen microphone and model.
const SETTINGS_KEY: &str = "listen";
/// Sent with a [`ListenStatus`] whenever it changes.
pub const STATUS_EVENT: &str = "listen://status";
/// Sent with a [`HeardWords`] for every sentence heard.
pub const HEARD_EVENT: &str = "listen://heard";
/// Sent with the input level (0 to 1), about ten times a second.
pub const LEVEL_EVENT: &str = "listen://level";
/// How often download progress is reported.
const PROGRESS_EVERY: Duration = Duration::from_millis(250);

/// A Whisper model size. All three are the English-only models, which hear
/// English better than the multilingual ones of the same size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub enum SpeechModel {
    /// For older computers (78 MB).
    Fast,
    /// Right for most computers (148 MB).
    #[default]
    Standard,
    /// Best with accents and a noisy room, on a recent computer (488 MB).
    Accurate,
}

impl SpeechModel {
    const ALL: [Self; 3] = [Self::Fast, Self::Standard, Self::Accurate];

    /// The model's file name, as whisper.cpp publishes it.
    fn file(self) -> &'static str {
        match self {
            Self::Fast => "ggml-tiny.en.bin",
            Self::Standard => "ggml-base.en.bin",
            Self::Accurate => "ggml-small.en.bin",
        }
    }

    /// Its size in bytes.
    fn size(self) -> u64 {
        match self {
            Self::Fast => 77_704_715,
            Self::Standard => 147_964_211,
            Self::Accurate => 487_614_201,
        }
    }

    /// Its SHA-256, as published with it.
    fn sha256(self) -> &'static str {
        match self {
            Self::Fast => "921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f",
            Self::Standard => "a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002",
            Self::Accurate => "c6138d6d58ecc8322097e0f987c32f1be8bb0a18532a3f88f734d1bbf9c41e5d",
        }
    }

    fn url(self) -> String {
        format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
            self.file()
        )
    }
}

/// What the operator chose (saved).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "desktop.ts")]
pub struct ListenSettings {
    /// The microphone ([`AudioInput::id`]), or the system's default.
    pub device: Option<String>,
    /// The speech model.
    pub model: SpeechModel,
}

/// A microphone or audio input to listen to.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct AudioInput {
    /// Identifies it in [`ListenSettings::device`].
    pub id: String,
    /// Its name, as the system shows it.
    pub name: String,
    /// Whether it is the system's default input.
    pub default: bool,
}

/// A speech model, and whether it is on this computer.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct ModelInfo {
    /// Which one.
    pub model: SpeechModel,
    /// Its download size in bytes.
    #[ts(type = "number")]
    pub size_bytes: u64,
    /// Whether it is downloaded.
    pub ready: bool,
}

/// A model on its way.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct ModelDownload {
    /// Which one.
    pub model: SpeechModel,
    /// How far along, 0 to 1.
    pub progress: f32,
}

/// Whether this computer is listening, and with what.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct ListenStatus {
    /// Listening now.
    pub listening: bool,
    /// Loading the model and opening the microphone.
    pub starting: bool,
    /// What was chosen.
    pub settings: ListenSettings,
    /// The models, and which are downloaded.
    pub models: Vec<ModelInfo>,
    /// A model being downloaded.
    pub download: Option<ModelDownload>,
    /// Why listening stopped by itself, or a download failed.
    pub problem: Option<String>,
}

/// A reference heard.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct HeardReference {
    /// The passage.
    pub passage: sw_bible::Passage,
    /// How it is written ("John 3:16").
    pub reference: String,
    /// The words it was found in ("John chapter three, verse sixteen").
    pub said: String,
}

/// A sentence heard, and the references in it.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct HeardWords {
    /// What was said.
    pub words: String,
    /// The references in it (often none).
    pub references: Vec<HeardReference>,
}

/// Listen's state, managed by Tauri.
#[derive(Default)]
pub struct ListenState {
    inner: Mutex<Listening>,
}

#[derive(Default)]
struct Listening {
    listener: Option<sw_listen::Listener>,
    starting: bool,
    spotter: Spotter,
    download: Option<ModelDownload>,
    problem: Option<String>,
}

fn lock(app: &AppHandle) -> std::sync::MutexGuard<'_, Listening> {
    app.state::<ListenState>()
        .inner()
        .inner
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Where the models are kept.
fn models_folder(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app.path().app_data_dir()?.join("speech-models"))
}

fn model_path(app: &AppHandle, model: SpeechModel) -> AppResult<PathBuf> {
    Ok(models_folder(app)?.join(model.file()))
}

/// Whether a model is downloaded (and complete).
fn is_ready(app: &AppHandle, model: SpeechModel) -> bool {
    model_path(app, model)
        .ok()
        .and_then(|path| std::fs::metadata(path).ok())
        .is_some_and(|meta| meta.len() == model.size())
}

fn settings(app: &AppHandle) -> ListenSettings {
    let state = app.state::<AppState>();
    let database = state.database();
    database
        .settings()
        .get::<ListenSettings>(SETTINGS_KEY)
        .ok()
        .flatten()
        .unwrap_or_default()
}

fn status(app: &AppHandle) -> ListenStatus {
    let listening = lock(app);
    ListenStatus {
        listening: listening.listener.is_some(),
        starting: listening.starting,
        settings: settings(app),
        models: SpeechModel::ALL
            .into_iter()
            .map(|model| ModelInfo {
                model,
                size_bytes: model.size(),
                ready: is_ready(app, model),
            })
            .collect(),
        download: listening.download.clone(),
        problem: listening.problem.clone(),
    }
}

/// Tells the windows what changed.
fn announce(app: &AppHandle) {
    let _ = app.emit(STATUS_EVENT, status(app));
}

/// Whether Listen is on, and with what.
#[tauri::command]
pub fn listen_status(app: AppHandle) -> ListenStatus {
    status(&app)
}

/// The microphones and audio inputs on this computer.
#[tauri::command]
pub async fn listen_devices() -> Vec<AudioInput> {
    tauri::async_runtime::spawn_blocking(sw_listen::input_devices)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|device| AudioInput {
            id: device.id,
            name: device.name,
            default: device.default,
        })
        .collect()
}

/// Chooses the microphone and model. If listening, starts again with them.
#[tauri::command]
pub async fn listen_configure(app: AppHandle, settings: ListenSettings) -> AppResult<ListenStatus> {
    {
        let state = app.state::<AppState>();
        let database = state.database();
        database.settings().set(SETTINGS_KEY, &settings)?;
    }
    let was_listening = lock(&app).listener.take().is_some();
    if was_listening && is_ready(&app, settings.model) {
        start(&app).await?;
    }
    announce(&app);
    Ok(status(&app))
}

/// Starts listening, with the chosen microphone and model (which must be
/// downloaded: see [`listen_download`]).
#[tauri::command]
pub async fn listen_start(app: AppHandle) -> AppResult<ListenStatus> {
    start(&app).await?;
    Ok(status(&app))
}

/// Stops listening.
#[tauri::command]
pub fn listen_stop(app: AppHandle) -> ListenStatus {
    {
        let mut listening = lock(&app);
        listening.listener = None;
        listening.spotter.forget();
    }
    announce(&app);
    status(&app)
}

async fn start(app: &AppHandle) -> AppResult<()> {
    let settings = settings(app);
    if !is_ready(app, settings.model) {
        return Err(AppError::Listen("download the speech model first".into()));
    }
    {
        let mut listening = lock(app);
        if listening.starting {
            return Ok(());
        }
        listening.listener = None;
        listening.starting = true;
        listening.problem = None;
    }
    announce(app);

    let path = model_path(app, settings.model)?;
    let events = app.clone();
    let started = tauri::async_runtime::spawn_blocking(move || {
        sw_listen::Listener::start(settings.device.as_deref(), &path, move |event| {
            on_event(&events, event);
        })
    })
    .await
    .map_err(|error| AppError::Listen(error.to_string()))?;

    let result = {
        let mut listening = lock(app);
        listening.starting = false;
        match started {
            Ok(listener) => {
                listening.listener = Some(listener);
                log::info!("listen: started");
                Ok(())
            }
            Err(error) => {
                log::warn!("listen: could not start: {error}");
                listening.problem = Some(error.to_string());
                Err(AppError::Listen(error.to_string()))
            }
        }
    };
    announce(app);
    result
}

/// What the listener reports, passed on to the windows.
fn on_event(app: &AppHandle, event: sw_listen::Event) {
    match event {
        sw_listen::Event::Level(level) => {
            let _ = app.emit(LEVEL_EVENT, level);
        }
        sw_listen::Event::Words(words) => {
            let found = lock(app).spotter.hear(&words);
            let references = found
                .into_iter()
                .map(|heard| HeardReference {
                    reference: heard.passage.to_string(),
                    passage: heard.passage,
                    said: heard.said,
                })
                .collect();
            let _ = app.emit(HEARD_EVENT, HeardWords { words, references });
        }
        sw_listen::Event::Failed(problem) => {
            log::warn!("listen: stopped: {problem}");
            {
                let mut listening = lock(app);
                listening.listener = None;
                listening.problem = Some(problem);
            }
            announce(app);
        }
    }
}

/// Downloads a speech model, reporting progress in the status, and checks
/// it before keeping it.
#[tauri::command]
pub async fn listen_download(app: AppHandle, model: SpeechModel) -> AppResult<ListenStatus> {
    {
        let mut listening = lock(&app);
        if listening.download.is_some() {
            return Ok(drop_and_status(listening, &app));
        }
        listening.download = Some(ModelDownload {
            model,
            progress: 0.0,
        });
        listening.problem = None;
    }
    announce(&app);

    let result = download(&app, model).await;
    {
        let mut listening = lock(&app);
        listening.download = None;
        if let Err(error) = &result {
            listening.problem = Some(error.to_string());
        }
    }
    announce(&app);
    result.map(|()| status(&app))
}

/// The status, after letting go of the lock it needs.
fn drop_and_status(
    listening: std::sync::MutexGuard<'_, Listening>,
    app: &AppHandle,
) -> ListenStatus {
    drop(listening);
    status(app)
}

async fn download(app: &AppHandle, model: SpeechModel) -> AppResult<()> {
    use std::io::Write;

    let folder = models_folder(app)?;
    std::fs::create_dir_all(&folder)?;
    let partial = folder.join(format!("{}.part", model.file()));
    let network = |error: reqwest::Error| AppError::Network(error.to_string());

    // The HTTPS library leaves the choice of encryption code to the app
    // (as the updater does too).
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    let client = reqwest::Client::builder()
        .user_agent(concat!("SimpleWorship/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(network)?;
    let mut response = client
        .get(model.url())
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(network)?;

    let mut file = std::fs::File::create(&partial)?;
    let mut hash = Sha256::new();
    let mut received: u64 = 0;
    let mut reported = Instant::now();
    while let Some(chunk) = response.chunk().await.map_err(network)? {
        file.write_all(&chunk)?;
        hash.update(&chunk);
        received += chunk.len() as u64;
        if reported.elapsed() >= PROGRESS_EVERY {
            reported = Instant::now();
            if let Some(download) = &mut lock(app).download {
                download.progress = (received as f32 / model.size() as f32).min(1.0);
            }
            announce(app);
        }
    }
    file.flush()?;
    drop(file);

    let digest: String = hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if digest != model.sha256() {
        let _ = std::fs::remove_file(&partial);
        return Err(AppError::Listen(
            "the downloaded speech model was damaged; try again".into(),
        ));
    }
    std::fs::rename(&partial, model_path(app, model)?)?;
    log::info!("listen: downloaded the {model:?} speech model");
    Ok(())
}

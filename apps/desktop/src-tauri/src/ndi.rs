//! NDI output: the stream look (and, if wanted, the projector's picture) as
//! NDI sources on the church network. See `sw_ndi` for how pictures get out.
//!
//! For each source there is a window nobody sees, showing the output with
//! the same renderer as the projector (`apps/desktop/src/ndi`). Whenever it
//! has drawn something new, its page calls [`ndi_frame_ready`]; a picture
//! of it is then taken and handed to the source's compositor, which fades
//! to it if the operator uses fades. The source's sender sends the
//! compositor's picture 30 times a second.
//!
//! NDI is off until turned on in Settings, and needs the free NDI Runtime
//! (or NDI Tools) from ndi.video. Without it, the status says so.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sw_ndi::{Compositor, Runtime, Sender};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use ts_rs::TS;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Sent to the operator window when the NDI status changes. Keep in sync
/// with `apps/desktop/src/lib/ndi.svelte.ts`.
pub const STATUS_EVENT: &str = "ndi://status";

const SETTINGS_KEY: &str = "ndi";

/// Label of the operator window.
const MAIN_WINDOW: &str = "main";

/// The NDI settings, saved between services.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "desktop.ts")]
pub struct NdiSettings {
    /// Send the stream look over NDI.
    pub enabled: bool,
    /// Also send the projector's picture, as a second source.
    pub main: bool,
}

/// The NDI sources SimpleWorship can send.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub enum NdiSource {
    /// The stream look (lower thirds), with transparency.
    Stream,
    /// What the projector shows.
    Main,
}

impl NdiSource {
    const ALL: [NdiSource; 2] = [NdiSource::Stream, NdiSource::Main];

    /// The hidden window's label.
    fn label(self) -> &'static str {
        match self {
            NdiSource::Stream => "ndi-stream",
            NdiSource::Main => "ndi-main",
        }
    }

    fn from_label(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|source| source.label() == label)
    }

    /// The name receivers list, after the computer's name.
    fn ndi_name(self) -> &'static str {
        match self {
            NdiSource::Stream => "SimpleWorship Stream",
            NdiSource::Main => "SimpleWorship Main",
        }
    }

    /// The `source` in the page's address.
    fn key(self) -> &'static str {
        match self {
            NdiSource::Stream => "stream",
            NdiSource::Main => "main",
        }
    }

    fn wanted(self, settings: NdiSettings) -> bool {
        settings.enabled
            && match self {
                NdiSource::Stream => true,
                NdiSource::Main => settings.main,
            }
    }
}

/// Everything the operator window shows about NDI.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct NdiStatus {
    /// The saved settings.
    pub settings: NdiSettings,
    /// NDI can work on this system at all (macOS and Windows for now).
    pub supported: bool,
    /// The NDI library in use, e.g. "NDI SDK WIN64 6.1.1", once loaded.
    pub runtime: Option<String>,
    /// Why NDI is not sending although it is turned on.
    pub error: Option<String>,
    /// The sources being sent, with how many receivers are watching each.
    pub sources: Vec<NdiSourceStatus>,
}

/// One source being sent.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct NdiSourceStatus {
    /// Which source.
    pub source: NdiSource,
    /// Its name on the network (after the computer's name).
    pub name: String,
    /// How many receivers are watching it.
    pub receivers: u32,
}

/// NDI while it runs. The lock is never held while a sender stops (its
/// thread may be reporting receivers, which takes the lock).
#[derive(Default)]
pub struct NdiState(Mutex<Running>);

#[derive(Default)]
struct Running {
    runtime: Option<Arc<Runtime>>,
    error: Option<String>,
    active: Vec<Active>,
}

/// A source being sent.
struct Active {
    source: NdiSource,
    compositor: Arc<Compositor>,
    /// The newest picture asked for; older ones that arrive late are dropped.
    latest: Arc<AtomicU64>,
    receivers: Arc<AtomicU64>,
    /// Dropped (which stops it) outside the lock.
    sender: Option<Sender>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Starts NDI if it is turned on. Called at startup.
pub fn start_if_enabled(app: &AppHandle) {
    if load_settings(app).enabled {
        let app = app.clone();
        tauri::async_runtime::spawn(async move { apply(&app) });
    }
}

/// Makes the sources match the saved settings.
fn apply(app: &AppHandle) {
    let settings = load_settings(app);
    let state = app.state::<NdiState>();

    // Stop the sources no longer wanted (outside the lock).
    let stopping: Vec<Active> = {
        let mut running = lock(&state.0);
        let (keep, stop) = std::mem::take(&mut running.active)
            .into_iter()
            .partition(|active| active.source.wanted(settings));
        running.active = keep;
        if !settings.enabled {
            running.error = None;
        }
        stop
    };
    for active in stopping {
        drop(active.sender);
        if let Some(window) = app.get_webview_window(active.source.label()) {
            let _ = window.destroy();
        }
    }

    if settings.enabled
        && let Err(error) = start_wanted(app, settings)
    {
        log::warn!("NDI: {error}");
        lock(&state.0).error = Some(error);
    }
    announce_status(app);
}

/// Starts each wanted source that is not running yet.
fn start_wanted(app: &AppHandle, settings: NdiSettings) -> Result<(), String> {
    if !sw_ndi::snapshot::SUPPORTED {
        return Err("NDI is not available on this system yet.".into());
    }
    let state = app.state::<NdiState>();
    let runtime = {
        let cached = lock(&state.0).runtime.clone();
        match cached {
            Some(runtime) => runtime,
            None => {
                let runtime = Runtime::load().map_err(|error| error.to_string())?;
                lock(&state.0).runtime = Some(runtime.clone());
                runtime
            }
        }
    };

    for source in NdiSource::ALL {
        let running = lock(&state.0)
            .active
            .iter()
            .any(|active| active.source == source);
        if running || !source.wanted(settings) {
            continue;
        }
        let compositor = Arc::new(Compositor::new(sw_ndi::WIDTH, sw_ndi::HEIGHT));
        let receivers = Arc::new(AtomicU64::new(0));
        let sender = Sender::start(runtime.clone(), source.ndi_name(), compositor.clone(), {
            let app = app.clone();
            let receivers = receivers.clone();
            move |count| {
                receivers.store(u64::from(count), Ordering::SeqCst);
                announce_status(&app);
            }
        })
        .map_err(|error| error.to_string())?;
        open_window(app, source).map_err(|error| error.to_string())?;
        lock(&state.0).active.push(Active {
            source,
            compositor,
            latest: Arc::new(AtomicU64::new(0)),
            receivers,
            sender: Some(sender),
        });
        log::info!("NDI: sending {}", source.ndi_name());
    }
    lock(&state.0).error = None;
    Ok(())
}

/// The window that draws `source` for NDI. Nobody sees it: on macOS it is
/// hidden; on Windows it sits far off the screen (WebView2 sizes a hidden
/// window wrongly). It is exactly 1920 × 1080 pixels and see-through.
fn open_window(app: &AppHandle, source: NdiSource) -> AppResult<WebviewWindow> {
    if let Some(window) = app.get_webview_window(source.label()) {
        return Ok(window);
    }
    let url = WebviewUrl::App(format!("index.html?view=ndi&source={}", source.key()).into());
    let builder = WebviewWindowBuilder::new(app, source.label(), url)
        .title(source.ndi_name())
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .focused(false)
        .skip_taskbar(true);
    #[cfg(windows)]
    let builder = builder.visible(true).position(-20000.0, -20000.0);
    #[cfg(not(windows))]
    let builder = builder.visible(false);
    let window = builder.build()?;
    window.set_size(PhysicalSize::new(sw_ndi::WIDTH, sw_ndi::HEIGHT))?;
    Ok(window)
}

// ── Status and settings ──────────────────────────────────────────────────

fn load_settings(app: &AppHandle) -> NdiSettings {
    let state = app.state::<AppState>();
    let settings = state.database().settings().get(SETTINGS_KEY);
    settings.ok().flatten().unwrap_or_default()
}

fn status(app: &AppHandle) -> NdiStatus {
    let settings = load_settings(app);
    let state = app.state::<NdiState>();
    let running = lock(&state.0);
    NdiStatus {
        settings,
        supported: sw_ndi::snapshot::SUPPORTED,
        runtime: running
            .runtime
            .as_ref()
            .map(|runtime| runtime.version().to_owned()),
        error: running.error.clone(),
        sources: running
            .active
            .iter()
            .map(|active| NdiSourceStatus {
                source: active.source,
                name: active.source.ndi_name().to_owned(),
                receivers: u32::try_from(active.receivers.load(Ordering::SeqCst)).unwrap_or(0),
            })
            .collect(),
    }
}

fn announce_status(app: &AppHandle) {
    if let Err(error) = app.emit_to(MAIN_WINDOW, STATUS_EVENT, status(app)) {
        log::warn!("NDI: could not update the status: {error}");
    }
}

// ── Commands ─────────────────────────────────────────────────────────────

/// The NDI status.
#[tauri::command]
pub fn ndi_status(app: AppHandle) -> NdiStatus {
    status(&app)
}

/// Saves the NDI settings and starts or stops sources to match. Async: it
/// may create windows (see `commands::outputs`).
#[tauri::command]
pub async fn ndi_set(app: AppHandle, settings: NdiSettings) -> AppResult<NdiStatus> {
    app.state::<AppState>()
        .database()
        .settings()
        .set(SETTINGS_KEY, &settings)?;
    apply(&app);
    Ok(status(&app))
}

/// Called by an NDI window's page when it has drawn something new: takes a
/// picture of it for its source, fading to it over `fade_ms`. With
/// `refresh`, it is a newer picture of the same thing (a font or picture
/// that finished loading), which never starts or cuts short a fade. Async,
/// so it never holds up the main thread.
#[tauri::command]
pub async fn ndi_frame_ready(
    app: AppHandle,
    window: WebviewWindow,
    fade_ms: u32,
    refresh: bool,
) -> AppResult<()> {
    let source = NdiSource::from_label(window.label()).ok_or(AppError::NotAnNdiWindow)?;
    log::info!(
        "NDI DEBUG: ready from {} (fade {fade_ms}ms)",
        window.label()
    );
    let found = lock(&app.state::<NdiState>().0)
        .active
        .iter()
        .find(|active| active.source == source)
        .map(|active| (active.compositor.clone(), active.latest.clone()));
    let Some((compositor, latest)) = found else {
        return Ok(());
    };
    let ticket = latest.fetch_add(1, Ordering::SeqCst) + 1;
    let fade = Duration::from_millis(u64::from(fade_ms));
    // WebKit draws what is pending before taking the picture. WebView2
    // takes what is on its screen, so give it a moment to draw first.
    #[cfg(windows)]
    std::thread::sleep(Duration::from_millis(50));
    sw_ndi::snapshot::snapshot(
        &window,
        Box::new(move |picture| match picture {
            // A newer picture was asked for meanwhile: that one counts.
            Ok(_) if latest.load(Ordering::SeqCst) != ticket => {}
            Ok(frame) if refresh => compositor.refresh(frame),
            Ok(frame) => compositor.show(frame, fade),
            Err(error) => log::warn!("NDI: no picture of {}: {error}", source.ndi_name()),
        }),
    );
    Ok(())
}

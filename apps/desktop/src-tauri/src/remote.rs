//! The phone remote and the stream browser source: runs the `sw-server` on
//! the church's network while either is turned on, and connects it to this
//! app.
//!
//! - [`DesktopHost`] gives the server the engine, the library and media.
//!   Commands from phones go through the same [`dispatch`] as the
//!   operator's own, so autosave, the CCLI log and everything else apply.
//! - Paired devices (with only a hash of their token) are saved in the
//!   settings table, so phones stay paired across restarts.
//! - The operator window is told when a phone asks to pair
//!   ([`PAIR_REQUEST_EVENT`]) and when devices connect ([`STATUS_EVENT`]).

use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sw_core::{Command, EngineEvent, EngineSnapshot, Id};
use sw_server::{
    DEFAULT_PORT, Device, Host, MediaFolder, PairRequest, RemoteServer, Role, ServerConfig,
};
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use crate::commands::engine::dispatch;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Sent to the operator window when a phone asks to pair.
/// Keep in sync with `apps/desktop/src/lib/remote.svelte.ts`.
pub const PAIR_REQUEST_EVENT: &str = "remote://pair-request";

/// Sent to the operator window when the remote's status changes.
pub const STATUS_EVENT: &str = "remote://status";

const SETTINGS_KEY: &str = "remote";
const DEVICES_KEY: &str = "remoteDevices";

/// Label of the operator window.
const MAIN_WINDOW: &str = "main";

/// The remote's settings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "desktop.ts")]
pub struct RemoteSettings {
    /// Phones may connect.
    pub enabled: bool,
    /// The network port.
    pub port: u16,
    /// OBS or vMix may show the stream look, from the browser source link.
    pub browser_source: bool,
    /// The secret in the browser source link. Made when the browser source
    /// is first turned on; a new one stops old links working.
    pub stream_key: String,
}

impl Default for RemoteSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            port: DEFAULT_PORT,
            browser_source: false,
            stream_key: String::new(),
        }
    }
}

/// Everything the operator window shows about the remote.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct RemoteStatus {
    /// The saved settings.
    pub settings: RemoteSettings,
    /// The server is running.
    pub running: bool,
    /// Why it is not running although it is turned on, if so.
    pub error: Option<String>,
    /// Addresses phones can open, e.g. `http://192.168.1.20:7270`.
    pub addresses: Vec<String>,
    /// Paired devices.
    pub devices: Vec<Device>,
    /// Ids of the devices connected now.
    pub connected: Vec<Id>,
    /// Browser source links, when it is on: this computer's first (for OBS
    /// on the same computer), then its network addresses.
    pub stream_links: Vec<String>,
}

/// A pairing code, ready to show.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct PairingCode {
    /// The 6-digit code.
    pub code: String,
    /// The address to open, with the code in it.
    pub url: String,
    /// A QR code of `url`, as SVG.
    pub qr_svg: String,
}

/// The running remote (if it is running) and what goes with it.
///
/// The lock is never held while the server is used: the server calls back
/// into this module (when the devices change, say), and those callbacks
/// take the lock too. Use [`server`] to get the server.
#[derive(Default)]
pub struct RemoteState(Mutex<Running>);

#[derive(Default)]
struct Running {
    server: Option<Arc<RemoteServer>>,
    announcer: Option<mdns_sd::ServiceDaemon>,
    error: Option<String>,
    connected: Vec<Id>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The running server, if the remote is on. The lock is released before
/// it returns (see [`RemoteState`]).
fn server(app: &AppHandle) -> Option<Arc<RemoteServer>> {
    let state = app.try_state::<RemoteState>()?;
    lock(&state.0).server.clone()
}

/// Passes an engine event on to connected phones (called by [`dispatch`]).
pub fn publish(app: &AppHandle, event: &EngineEvent) {
    if let Some(server) = server(app) {
        server.publish(event);
    }
}

// ── Starting and stopping ────────────────────────────────────────────────

/// Starts the server if the phone remote or the browser source is on.
/// Called at startup.
pub fn start_if_enabled(app: &AppHandle) {
    let settings = load_settings(app);
    if settings.enabled || settings.browser_source {
        let app = app.clone();
        tauri::async_runtime::spawn(async move { apply(&app).await });
    }
}

/// Makes the server match the saved settings: started, stopped, or (when
/// only who may connect changed) adjusted without disconnecting anyone
/// else.
async fn apply(app: &AppHandle) {
    let settings = load_settings(app);
    let wanted = settings.enabled || settings.browser_source;
    let key = settings
        .browser_source
        .then_some(settings.stream_key.as_str());
    match server(app) {
        Some(running) if wanted && running.port() == settings.port => {
            running.set_phones(settings.enabled);
            running.set_stream_key(key);
        }
        _ if wanted => start(app, &settings).await,
        _ => {
            stop(app);
            lock(&app.state::<RemoteState>().0).error = None;
        }
    }
    announce_status(app);
}

async fn start(app: &AppHandle, settings: &RemoteSettings) {
    stop(app);
    let devices = load_devices(app);
    let host = Arc::new(DesktopHost { app: app.clone() });
    let config = ServerConfig {
        port: settings.port,
        static_dir: static_dir(app),
        phones: settings.enabled,
        stream_key: settings.browser_source.then(|| settings.stream_key.clone()),
    };
    let result = RemoteServer::start(config, host, devices).await;
    {
        let state = app.state::<RemoteState>();
        let mut running = lock(&state.0);
        match result {
            Ok(server) => {
                log::info!("phone remote listening on port {}", server.port());
                running.announcer = announce(server.port());
                running.server = Some(Arc::new(server));
                running.error = None;
            }
            Err(error) => {
                log::warn!("the phone remote could not start: {error}");
                running.error = Some(error.to_string());
            }
        }
    }
}

fn stop(app: &AppHandle) {
    let (server, announcer) = {
        let state = app.state::<RemoteState>();
        let mut running = lock(&state.0);
        running.connected.clear();
        (running.server.take(), running.announcer.take())
    };
    if let Some(server) = server {
        server.stop();
    }
    if let Some(announcer) = announcer {
        let _ = announcer.shutdown();
    }
}

/// Announces SimpleWorship on the network as `simpleworship.local`, for
/// phones that can open addresses by name. Best effort: many networks and
/// phones do not support it, and the QR code works without it.
fn announce(port: u16) -> Option<mdns_sd::ServiceDaemon> {
    let daemon = mdns_sd::ServiceDaemon::new().ok()?;
    let addresses: Vec<IpAddr> = local_addresses();
    let service = mdns_sd::ServiceInfo::new(
        "_http._tcp.local.",
        "SimpleWorship",
        "simpleworship.local.",
        &addresses[..],
        port,
        &[("path", "/")][..],
    )
    .ok()?;
    daemon.register(service).ok()?;
    Some(daemon)
}

/// Where the built remote web app is: beside the program when installed,
/// or in the source tree while developing.
fn static_dir(app: &AppHandle) -> PathBuf {
    let installed = app
        .path()
        .resource_dir()
        .map(|folder| folder.join("remote"))
        .ok()
        .filter(|folder| folder.join("index.html").is_file());
    installed.unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../remote/dist"))
}

/// The computer's addresses on the local network (IPv4, private ranges).
fn local_addresses() -> Vec<IpAddr> {
    let mut found: Vec<IpAddr> = local_ip_address::list_afinet_netifas()
        .unwrap_or_default()
        .into_iter()
        .map(|(_, address)| address)
        .filter(|address| match address {
            IpAddr::V4(v4) => v4.is_private() && !v4.is_loopback(),
            IpAddr::V6(_) => false,
        })
        .collect();
    found.sort();
    found.dedup();
    found
}

// ── The host: what the server can ask of the app ─────────────────────────

struct DesktopHost {
    app: AppHandle,
}

impl Host for DesktopHost {
    fn snapshot(&self) -> EngineSnapshot {
        self.app.state::<AppState>().engine().snapshot()
    }

    fn dispatch(&self, command: Command) -> Result<(), String> {
        log::info!("from a phone: {command:?}");
        dispatch(&self.app, &self.app.state::<AppState>(), command)
            .map_err(|error| error.to_string())
    }

    fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        answer_call(&self.app, method, params).map_err(|error| error.to_string())
    }

    fn media_path(&self, folder: MediaFolder, file: &str) -> Option<PathBuf> {
        let folders = &self.app.state::<AppState>().media_folders;
        let path = match folder {
            MediaFolder::Media => folders.media.join(file),
            MediaFolder::Thumbnails => folders.thumbnails.join(file),
        };
        path.is_file().then_some(path)
    }

    fn pairing_requested(&self, request: PairRequest) {
        if let Err(error) = self.app.emit_to(MAIN_WINDOW, PAIR_REQUEST_EVENT, request) {
            log::warn!("could not ask about a phone: {error}");
        }
    }

    fn devices_changed(&self, devices: &[Device]) {
        let state = self.app.state::<AppState>();
        if let Err(error) = state.database().settings().set(DEVICES_KEY, &devices) {
            log::warn!("could not save the paired devices: {error}");
        }
        announce_status(&self.app);
    }

    fn connections_changed(&self, connected: Vec<Id>) {
        lock(&self.app.state::<RemoteState>().0).connected = connected;
        announce_status(&self.app);
    }
}

/// Answers a question from a phone (its role was already checked).
fn answer_call(app: &AppHandle, method: &str, params: Value) -> AppResult<Value> {
    use crate::commands::{scripture, settings as settings_commands, songs};
    let state = app.state::<AppState>();
    let text = |key: &str| {
        params
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    let id = |key: &str| -> AppResult<Id> {
        text(key)
            .parse()
            .map_err(|_| AppError::Conversion(format!("missing {key}")))
    };
    let parsed = |key: &str| -> AppResult<Value> {
        params
            .get(key)
            .cloned()
            .ok_or_else(|| AppError::Conversion(format!("missing {key}")))
    };

    Ok(match method {
        "songs.search" => json!(songs::songs_search(state, text("query"))?),
        "songs.preview" => json!(songs::song_show_in_preview(app.clone(), state, id("id")?)?),
        "bibles.list" => json!(scripture::bibles_list(state)?),
        "scripture.parse" => json!(scripture::scripture_parse(text("text"))),
        "scripture.preview" => {
            let read = |key: &str| -> AppResult<Value> { parsed(key) };
            let passage = from_json(read("passage")?)?;
            let bible_ids = from_json(read("bibleIds")?)?;
            let options = from_json(read("options")?)?;
            json!(scripture::scripture_show_in_preview(
                app.clone(),
                state,
                passage,
                bible_ids,
                options
            )?)
        }
        "alerts.templates" => json!(settings_commands::settings_get(state)?.alert_templates),
        "stage.options" => json!(settings_commands::settings_get(state)?.stage),
        _ => return Err(AppError::Conversion(format!("unknown question {method}"))),
    })
}

// ── Saved settings and devices ───────────────────────────────────────────

fn load_settings(app: &AppHandle) -> RemoteSettings {
    let state = app.state::<AppState>();
    let settings = state.database().settings().get(SETTINGS_KEY);
    settings.ok().flatten().unwrap_or_default()
}

fn save_settings(app: &AppHandle, settings: &RemoteSettings) -> AppResult<()> {
    let state = app.state::<AppState>();
    Ok(state.database().settings().set(SETTINGS_KEY, settings)?)
}

fn load_devices(app: &AppHandle) -> Vec<Device> {
    let state = app.state::<AppState>();
    let devices = state.database().settings().get(DEVICES_KEY);
    devices.ok().flatten().unwrap_or_default()
}

/// Reads a value a phone sent.
fn from_json<T: serde::de::DeserializeOwned>(value: Value) -> AppResult<T> {
    serde_json::from_value(value).map_err(|error| AppError::Conversion(error.to_string()))
}

fn status(app: &AppHandle) -> RemoteStatus {
    let settings = load_settings(app);
    let (server, error, connected) = {
        let state = app.state::<RemoteState>();
        let running = lock(&state.0);
        (
            running.server.clone(),
            running.error.clone(),
            running.connected.clone(),
        )
    };
    let port = server.as_deref().map_or(settings.port, RemoteServer::port);
    let addresses: Vec<String> = local_addresses()
        .into_iter()
        .map(|address| format!("http://{address}:{port}"))
        .collect();
    let stream_links = if settings.browser_source && server.is_some() {
        std::iter::once(format!("http://localhost:{port}"))
            .chain(addresses.iter().cloned())
            .map(|base| format!("{base}/stream?key={}", settings.stream_key))
            .collect()
    } else {
        Vec::new()
    };
    RemoteStatus {
        running: server.is_some(),
        error,
        addresses,
        stream_links,
        devices: server
            .as_deref()
            .map(RemoteServer::devices)
            .unwrap_or_else(|| load_devices(app)),
        connected,
        settings,
    }
}

fn announce_status(app: &AppHandle) {
    if let Err(error) = app.emit_to(MAIN_WINDOW, STATUS_EVENT, status(app)) {
        log::warn!("could not update the remote status: {error}");
    }
}

// ── Commands for the operator window ─────────────────────────────────────

/// The remote's status.
#[tauri::command]
pub fn remote_status(app: AppHandle) -> RemoteStatus {
    status(&app)
}

/// Turns the phone remote on (on `port`) or off, and saves the choice.
#[tauri::command]
pub async fn remote_set_enabled(
    app: AppHandle,
    enabled: bool,
    port: u16,
) -> AppResult<RemoteStatus> {
    let settings = RemoteSettings {
        enabled,
        port,
        ..load_settings(&app)
    };
    save_settings(&app, &settings)?;
    apply(&app).await;
    Ok(status(&app))
}

/// Turns the browser source on or off, and saves the choice.
#[tauri::command]
pub async fn stream_link_set_enabled(app: AppHandle, enabled: bool) -> AppResult<RemoteStatus> {
    let mut settings = load_settings(&app);
    settings.browser_source = enabled;
    if settings.stream_key.is_empty() {
        settings.stream_key = sw_server::pairing::new_token();
    }
    save_settings(&app, &settings)?;
    apply(&app).await;
    Ok(status(&app))
}

/// Makes a new browser source link. The old link stops working at once.
#[tauri::command]
pub async fn stream_link_renew(app: AppHandle) -> AppResult<RemoteStatus> {
    let mut settings = load_settings(&app);
    settings.stream_key = sw_server::pairing::new_token();
    save_settings(&app, &settings)?;
    apply(&app).await;
    Ok(status(&app))
}

/// A new pairing code and QR code for "Connect a Phone".
#[tauri::command]
pub fn remote_pairing_start(app: AppHandle) -> AppResult<PairingCode> {
    let code = server(&app).ok_or(AppError::RemoteOff)?.new_pairing_code();
    let base = status(&app)
        .addresses
        .into_iter()
        .next()
        .ok_or(AppError::NoNetwork)?;
    let url = format!("{base}/#code={code}");
    let qr_svg = qrcode::QrCode::new(url.as_bytes())
        .map(|qr| {
            qr.render::<qrcode::render::svg::Color>()
                .min_dimensions(240, 240)
                .quiet_zone(true)
                .build()
        })
        .unwrap_or_default();
    Ok(PairingCode { code, url, qr_svg })
}

/// Stops accepting the pairing code ("Connect a Phone" was closed).
#[tauri::command]
pub fn remote_pairing_cancel(app: AppHandle) {
    if let Some(server) = server(&app) {
        server.cancel_pairing();
    }
}

/// The operator's answer to a phone asking to pair: a role, or `None`.
#[tauri::command]
pub fn remote_answer(app: AppHandle, request: Id, role: Option<Role>) {
    if let Some(server) = server(&app) {
        server.answer(request, role);
    }
}

/// Changes what a paired device may do.
#[tauri::command]
pub fn remote_set_role(app: AppHandle, device: Id, role: Role) -> AppResult<()> {
    server(&app)
        .ok_or(AppError::RemoteOff)?
        .set_role(device, role);
    Ok(())
}

/// Removes a paired device: it is disconnected and cannot connect again.
#[tauri::command]
pub fn remote_remove(app: AppHandle, device: Id) -> AppResult<()> {
    match server(&app) {
        Some(server) => server.remove(device),
        None => {
            // Not running: just forget it.
            let devices: Vec<Device> = load_devices(&app)
                .into_iter()
                .filter(|known| known.id != device)
                .collect();
            app.state::<AppState>()
                .database()
                .settings()
                .set(DEVICES_KEY, &devices)?;
        }
    }
    Ok(())
}

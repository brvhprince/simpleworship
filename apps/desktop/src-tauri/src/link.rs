//! Joining another SimpleWorship computer (the host, the one running the
//! service) over the church network, as a second operator or a mirror.
//!
//! A joined computer runs the host's service, not its own: this module
//! keeps a WebSocket to the host's server (`sw-server`, the same one phones
//! use) and
//!
//! - passes the host's engine events on to every window here, so the
//!   operator window, the screens and NDI all show the host's service;
//! - sends this computer's commands to the host (see
//!   `commands::engine::engine_dispatch`), and the operator window's
//!   library questions too ([`link_call`], answered by `host_calls.rs` on
//!   the host);
//! - works out the difference between the two clocks, so videos and
//!   countdowns stay in step.
//!
//! Each computer has its own preview pane: the host tags preview changes
//! with the operator they belong to, and only this computer's are passed
//! on. Pairing works as for phones: the host's code, then the host's
//! operator allows this computer and chooses its role.
//!
//! The link is saved, so a joined computer rejoins by itself after a
//! restart, and reconnects if the network drops.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sw_core::{Command, EngineEvent, EngineSnapshot, Id, LOCAL_OPERATOR};
use sw_server::Role;
use sw_server::protocol::{ClientMessage, ServerMessage};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{mpsc, oneshot};
use ts_rs::TS;

use crate::commands::engine::ENGINE_EVENT;
use crate::error::{AppError, AppResult};
use crate::plain_http;
use crate::state::AppState;

/// The network service hosts announce themselves with (see `remote.rs`).
pub const SERVICE_TYPE: &str = "_simpleworship._tcp.local.";

/// Sent to every window when the link's status changes. Keep in sync with
/// `apps/desktop/src/lib/link.svelte.ts`.
pub const STATUS_EVENT: &str = "link://status";

/// Sent to every window when this computer joins or leaves a host: each
/// reloads, to take its pictures and videos from the right computer.
pub const CHANGED_EVENT: &str = "link://changed";

const SETTINGS_KEY: &str = "link";

/// What the main computer announces that this one passes on to its own
/// windows, so their lists (fetched from the main computer) reload and the
/// schedule's name is the main computer's.
const SHARED_NOTICES: [&str; 6] = [
    crate::media::MEDIA_EVENT,
    crate::commands::presentations::PRESENTATIONS_EVENT,
    crate::bibles::BIBLES_EVENT,
    crate::commands::songs::SONGS_EVENT,
    crate::commands::settings::SETTINGS_EVENT,
    crate::schedule_file::SCHEDULE_FILE_EVENT,
];

/// Connected (again): this computer's lists reload from the main computer,
/// which may have changed while it was away, and the schedule's name is
/// the main computer's.
fn refresh_shared(app: &AppHandle) {
    for event in [
        crate::media::MEDIA_EVENT,
        crate::commands::presentations::PRESENTATIONS_EVENT,
        crate::bibles::BIBLES_EVENT,
        crate::commands::songs::SONGS_EVENT,
    ] {
        let _ = app.emit(event, ());
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let asked = request(&app, |id| ClientMessage::Call {
            id,
            method: "schedule_file_status".into(),
            params: Value::Null,
        });
        if let Ok(status) = asked.await {
            let _ = app.emit(crate::schedule_file::SCHEDULE_FILE_EVENT, status);
        }
    });
}

/// Label of the operator window.
const MAIN_WINDOW: &str = "main";

/// How long a host may take to answer a request.
const CALL_TIMEOUT: Duration = Duration::from_secs(30);
/// How long to wait for the host's operator to allow this computer.
const PAIRING_TIMEOUT: Duration = Duration::from_secs(150);
/// How often to measure the difference between the clocks again.
const CLOCK_EVERY: Duration = Duration::from_secs(300);
/// How long the main computer may be unreachable before this computer
/// offers to take over the service.
const LOST_AFTER: Duration = Duration::from_secs(10);

/// The saved link to a host.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinkSettings {
    /// The host's address, e.g. `http://192.168.1.20:7270`.
    address: String,
    /// This computer's token on the host.
    token: String,
    /// The host's name, as it calls itself.
    host_name: String,
    /// What this computer may do there.
    role: Role,
}

/// Everything the windows show about the link.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct LinkStatus {
    /// This computer has joined a host.
    pub joined: bool,
    /// The host's name.
    pub host_name: String,
    /// The host's address, for pictures and videos (`{address}/media/…`).
    pub address: String,
    /// This computer's token there, for those addresses.
    pub token: String,
    /// What this computer may do: an operator (full control) or a mirror.
    pub role: Option<Role>,
    /// Connected right now (otherwise reconnecting).
    pub connected: bool,
    /// The host's clock minus this computer's, in milliseconds.
    #[ts(type = "number")]
    pub clock_offset_ms: i64,
    /// The host removed this computer.
    pub removed: bool,
    /// The main computer has been unreachable for a while: this computer
    /// can take over the service ([`link_take_over`]).
    pub host_lost: bool,
}

/// What taking over found.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct TakeOverResult {
    /// Pictures, videos and sound in the service that are not on this
    /// computer (they show as black, or are silent).
    pub missing_media: u32,
}

/// A host found on the network.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct FoundHost {
    /// What it calls itself, e.g. "Winners Chapel (MAIN-PC)".
    pub name: String,
    /// Its address, e.g. `http://192.168.1.20:7270`.
    pub address: String,
    /// Its SimpleWorship version.
    pub version: String,
    /// It runs the same version as this computer (needed to join).
    pub compatible: bool,
}

/// The link while it runs.
#[derive(Default)]
pub struct LinkState {
    inner: Mutex<Linked>,
    clock_offset_ms: AtomicI64,
}

#[derive(Default)]
struct Linked {
    settings: Option<LinkSettings>,
    /// This computer's id on the host (its preview pane there).
    device: Option<Id>,
    /// Messages to send to the host.
    outgoing: Option<mpsc::UnboundedSender<String>>,
    /// Requests waiting for the host's answer.
    pending: HashMap<u32, oneshot::Sender<Result<Value, String>>>,
    next_id: u32,
    connected: bool,
    removed: bool,
    /// Ends the connection (leaving).
    stop: Option<oneshot::Sender<()>>,
    /// The service as last heard from the main computer, kept up to date
    /// for taking over (see `sw_core::takeover`).
    copy: Option<EngineSnapshot>,
    /// Since when the main computer has been unreachable.
    lost_since: Option<Instant>,
    host_lost: bool,
}

fn lock(state: &LinkState) -> MutexGuard<'_, Linked> {
    state
        .inner
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Whether this computer runs a host's service instead of its own.
pub fn is_joined(app: &AppHandle) -> bool {
    app.try_state::<LinkState>()
        .is_some_and(|state| lock(&state).settings.is_some())
}

/// Loads the saved link, if any, and connects. Called at startup, before
/// anything else uses the engine.
pub fn start_if_joined(app: &AppHandle) {
    let saved: Option<LinkSettings> = app
        .state::<AppState>()
        .database()
        .settings()
        .get(SETTINGS_KEY)
        .ok()
        .flatten();
    if let Some(settings) = saved {
        lock(&app.state::<LinkState>()).settings = Some(settings);
        connect(app);
    }
}

fn status(app: &AppHandle) -> LinkStatus {
    let state = app.state::<LinkState>();
    let linked = lock(&state);
    match &linked.settings {
        Some(settings) => LinkStatus {
            joined: true,
            host_name: settings.host_name.clone(),
            address: settings.address.clone(),
            token: settings.token.clone(),
            role: Some(settings.role),
            connected: linked.connected,
            clock_offset_ms: state.clock_offset_ms.load(Ordering::Relaxed),
            removed: false,
            host_lost: linked.host_lost && linked.copy.is_some(),
        },
        None => LinkStatus {
            removed: linked.removed,
            ..LinkStatus::default()
        },
    }
}

fn announce_status(app: &AppHandle) {
    if let Err(error) = app.emit(STATUS_EVENT, status(app)) {
        log::warn!("link: could not update the status: {error}");
    }
}

fn save(app: &AppHandle, settings: Option<&LinkSettings>) -> AppResult<()> {
    let state = app.state::<AppState>();
    let database = state.database();
    match settings {
        Some(settings) => database.settings().set(SETTINGS_KEY, settings)?,
        None => database.settings().delete(SETTINGS_KEY)?,
    }
    Ok(())
}

// ── The connection ────────────────────────────────────────────────────────

/// Starts the connection to the saved host, reconnecting until left.
fn connect(app: &AppHandle) {
    let (stop, stopped) = oneshot::channel::<()>();
    lock(&app.state::<LinkState>()).stop = Some(stop);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut stopped = stopped;
        let mut retry = Duration::from_millis(500);
        loop {
            let Some(settings) = lock(&app.state::<LinkState>()).settings.clone() else {
                return;
            };
            tokio::select! {
                _ = &mut stopped => return,
                ended = run_connection(&app, &settings) => {
                    if let Err(error) = ended {
                        log::warn!("link: connection to {} ended: {error}", settings.address);
                    }
                }
            }
            let state = app.state::<LinkState>();
            {
                let mut linked = lock(&state);
                linked.connected = false;
                linked.outgoing = None;
                for (_, waiting) in linked.pending.drain() {
                    let _ = waiting.send(Err("lost the connection to the host".into()));
                }
                // Unreachable for a while: offer to take over.
                let since = *linked.lost_since.get_or_insert_with(Instant::now);
                if since.elapsed() >= LOST_AFTER {
                    linked.host_lost = true;
                }
                if linked.settings.is_none() {
                    drop(linked);
                    announce_status(&app);
                    return;
                }
            }
            announce_status(&app);
            tokio::select! {
                _ = &mut stopped => return,
                () = tokio::time::sleep(retry) => {}
            }
            retry = (retry * 2).min(Duration::from_secs(5));
        }
    });
}

/// One connection to the host, until it drops.
async fn run_connection(app: &AppHandle, settings: &LinkSettings) -> Result<(), String> {
    let socket_address = format!(
        "{}/ws?token={}",
        settings.address.replacen("http", "ws", 1),
        settings.token
    );
    let (socket, _) = tokio_tungstenite::connect_async(&socket_address)
        .await
        .map_err(|error| error.to_string())?;
    let (mut sender, mut receiver) = socket.split();
    let (outgoing, mut to_send) = mpsc::unbounded_channel::<String>();
    lock(&app.state::<LinkState>()).outgoing = Some(outgoing);

    let mut clock_checked: Option<Instant> = None;
    loop {
        if clock_checked.is_none_or(|at| at.elapsed() > CLOCK_EVERY) {
            clock_checked = Some(Instant::now());
            let app = app.clone();
            let address = settings.address.clone();
            tauri::async_runtime::spawn(async move { measure_clock(&app, &address).await });
        }
        tokio::select! {
            message = to_send.recv() => {
                let Some(text) = message else { return Ok(()) };
                sender
                    .send(tokio_tungstenite::tungstenite::Message::Text(text.into()))
                    .await
                    .map_err(|error| error.to_string())?;
            }
            incoming = receiver.next() => {
                let Some(incoming) = incoming else { return Ok(()) };
                let incoming = incoming.map_err(|error| error.to_string())?;
                if let tokio_tungstenite::tungstenite::Message::Text(text) = incoming {
                    match serde_json::from_str::<ServerMessage>(&text) {
                        Ok(message) => receive(app, message),
                        Err(error) => log::warn!("link: not understood: {error}"),
                    }
                }
            }
            () = tokio::time::sleep(CLOCK_EVERY) => {}
        }
    }
}

/// Handles one message from the host.
fn receive(app: &AppHandle, message: ServerMessage) {
    let state = app.state::<LinkState>();
    match message {
        ServerMessage::Welcome {
            device,
            role,
            snapshot,
            ..
        } => {
            {
                let mut linked = lock(&state);
                linked.device = Some(device);
                linked.connected = true;
                linked.lost_since = None;
                linked.host_lost = false;
                // The host's snapshot for this device has its own preview,
                // which is this computer's local one.
                linked.copy = Some((*snapshot).clone());
                if let Some(settings) = &mut linked.settings {
                    settings.role = role;
                }
            }
            // The windows may have missed changes while disconnected: give
            // them the whole state again, as events they already handle.
            for event in events_for(*snapshot) {
                let _ = app.emit(ENGINE_EVENT, &event);
            }
            announce_status(app);
            refresh_shared(app);
        }
        ServerMessage::Event { event } => {
            let event = {
                let mut linked = lock(&state);
                let event = for_this_computer(*event, linked.device);
                if let (Some(event), Some(copy)) = (&event, &mut linked.copy) {
                    copy.apply(event);
                }
                event
            };
            if let Some(event) = event {
                let _ = app.emit(ENGINE_EVENT, &event);
            }
        }
        ServerMessage::Result { id, value, error } => {
            if let Some(waiting) = lock(&state).pending.remove(&id) {
                let _ = waiting.send(match error {
                    Some(error) => Err(error),
                    None => Ok(value.unwrap_or(Value::Null)),
                });
            }
        }
        ServerMessage::RoleChanged { role } => {
            let settings = {
                let mut linked = lock(&state);
                if let Some(settings) = &mut linked.settings {
                    settings.role = role;
                }
                linked.settings.clone()
            };
            let _ = save(app, settings.as_ref());
            announce_status(app);
            // An operator and a mirror have different windows.
            let _ = app.emit(CHANGED_EVENT, ());
        }
        ServerMessage::Removed => {
            {
                let mut linked = lock(&state);
                linked.settings = None;
                linked.removed = true;
            }
            let _ = save(app, None);
            announce_status(app);
            let _ = app.emit(CHANGED_EVENT, ());
        }
        // The main computer's library or schedule file changed.
        ServerMessage::Notice { name, payload } => {
            if SHARED_NOTICES.contains(&name.as_str()) {
                let _ = app.emit(&name, payload);
            }
        }
    }
}

/// The host's event as this computer's windows should see it: a preview
/// change only if it is this computer's own preview (shown as the local
/// one), everything else as it is.
fn for_this_computer(event: EngineEvent, device: Option<Id>) -> Option<EngineEvent> {
    match event {
        EngineEvent::PreviewChanged { operator, preview } => {
            (Some(operator) == device).then_some(EngineEvent::PreviewChanged {
                operator: LOCAL_OPERATOR,
                preview,
            })
        }
        other => Some(other),
    }
}

/// The whole state as the events that set each part of it.
fn events_for(snapshot: EngineSnapshot) -> Vec<EngineEvent> {
    vec![
        EngineEvent::ThemesChanged {
            themes: snapshot.themes,
        },
        EngineEvent::ScheduleChanged {
            schedule: snapshot.schedule,
        },
        EngineEvent::PreviewChanged {
            operator: LOCAL_OPERATOR,
            preview: snapshot.preview,
        },
        EngineEvent::LiveChanged {
            live: snapshot.live,
        },
        EngineEvent::OutputChanged {
            output: Box::new(snapshot.output),
        },
        EngineEvent::StageChanged {
            stage: snapshot.stage,
        },
        EngineEvent::AudioChanged {
            audio: snapshot.audio,
        },
        EngineEvent::TransitionsChanged {
            transitions: snapshot.transitions,
        },
        EngineEvent::StreamSettingsChanged {
            settings: snapshot.stream,
        },
    ]
}

/// Measures the host's clock against this one: the host's time, minus the
/// time half-way through the request, from the quickest of a few tries.
async fn measure_clock(app: &AppHandle, address: &str) {
    #[derive(Deserialize)]
    struct Now {
        now: i64,
    }
    let mut best: Option<(Duration, i64)> = None;
    for _ in 0..5 {
        let sent = Instant::now();
        let before = sw_core::time::now();
        let Ok(response) = plain_http::get(address, "/api/time", Duration::from_secs(3)).await
        else {
            continue;
        };
        let Ok(Now { now }) = response.json::<Now>() else {
            continue;
        };
        let took = sent.elapsed();
        let middle = before + i64::try_from(took.as_millis() / 2).unwrap_or(0);
        if best.is_none_or(|(quickest, _)| took < quickest) {
            best = Some((took, now - middle));
        }
    }
    if let Some((_, offset)) = best {
        app.state::<LinkState>()
            .clock_offset_ms
            .store(offset, Ordering::Relaxed);
        announce_status(app);
    }
}

/// Sends `message` to the host and waits for its answer.
async fn request(app: &AppHandle, message: impl FnOnce(u32) -> ClientMessage) -> AppResult<Value> {
    let state = app.state::<LinkState>();
    let (answer, waiting) = oneshot::channel();
    {
        let mut linked = lock(&state);
        let Some(outgoing) = linked.outgoing.clone() else {
            return Err(AppError::HostUnreachable);
        };
        linked.next_id = linked.next_id.wrapping_add(1).max(1);
        let id = linked.next_id;
        linked.pending.insert(id, answer);
        let text = serde_json::to_string(&message(id))
            .map_err(|error| AppError::Conversion(error.to_string()))?;
        if outgoing.send(text).is_err() {
            linked.pending.remove(&id);
            return Err(AppError::HostUnreachable);
        }
    }
    match tokio::time::timeout(CALL_TIMEOUT, waiting).await {
        Ok(Ok(Ok(value))) => Ok(value),
        Ok(Ok(Err(error))) => Err(AppError::FromHost(error)),
        _ => Err(AppError::HostUnreachable),
    }
}

/// Sends an engine command to the host (see `engine_dispatch`).
pub async fn dispatch(app: &AppHandle, command: Command) -> AppResult<()> {
    request(app, |id| ClientMessage::Dispatch { id, command }).await?;
    Ok(())
}

/// The host's whole state, with this computer's preview as the local one.
pub async fn snapshot(app: &AppHandle) -> AppResult<EngineSnapshot> {
    let value = request(app, |id| ClientMessage::Call {
        id,
        method: "engine.snapshot".into(),
        params: Value::Null,
    })
    .await?;
    serde_json::from_value(value).map_err(|error| AppError::Conversion(error.to_string()))
}

// ── Commands for the windows ─────────────────────────────────────────────

/// The link's status.
#[tauri::command]
pub fn link_status(app: AppHandle) -> LinkStatus {
    status(&app)
}

/// Runs one of the operator window's commands on the host (see
/// `host_calls.rs` there), with the arguments the window would give it.
#[tauri::command]
pub async fn link_call(app: AppHandle, method: String, params: Value) -> AppResult<Value> {
    request(&app, |id| ClientMessage::Call { id, method, params }).await
}

/// Looks for hosts on the network for a few seconds.
#[tauri::command]
pub async fn link_discover(app: AppHandle) -> AppResult<Vec<FoundHost>> {
    let ours = app.package_info().version.to_string();
    let daemon = mdns_sd::ServiceDaemon::new().map_err(|e| AppError::Network(e.to_string()))?;
    let browsing = daemon
        .browse(SERVICE_TYPE)
        .map_err(|e| AppError::Network(e.to_string()))?;
    let own = crate::remote::own_addresses();
    let mut found: Vec<FoundHost> = Vec::new();
    let until = Instant::now() + Duration::from_secs(3);
    while let Some(left) = until.checked_duration_since(Instant::now()) {
        let Ok(Ok(event)) = tokio::time::timeout(left, browsing.recv_async()).await else {
            break;
        };
        if let mdns_sd::ServiceEvent::ServiceResolved(info) = event {
            let Some(ip) = info.get_addresses_v4().into_iter().next() else {
                continue;
            };
            let itself =
                info.get_property_val_str("instance") == Some(crate::remote::instance_id());
            if itself || own.contains(&std::net::IpAddr::V4(ip)) {
                continue;
            }
            let address = format!("http://{ip}:{}", info.get_port());
            let version = info
                .get_property_val_str("version")
                .unwrap_or("")
                .to_owned();
            let name = info
                .get_property_val_str("name")
                .unwrap_or(info.get_fullname())
                .to_owned();
            if !found.iter().any(|host| host.address == address) {
                found.push(FoundHost {
                    compatible: version == ours,
                    name,
                    address,
                    version,
                });
            }
        }
    }
    let _ = daemon.shutdown();
    Ok(found)
}

#[derive(Deserialize)]
struct Hello {
    protocol: u32,
    version: String,
    name: Option<String>,
    instance: Option<String>,
}

#[derive(Deserialize)]
struct PairAnswer {
    token: String,
    role: Role,
}

/// Joins the host at `address` (e.g. `192.168.1.20` or
/// `http://192.168.1.20:7270`) with its pairing `code`, calling this
/// computer `name`. Waits until the host's operator allows it.
#[tauri::command]
pub async fn link_join(
    app: AppHandle,
    address: String,
    code: String,
    name: String,
) -> AppResult<LinkStatus> {
    let address = normalize(&address);
    let hello: Hello = plain_http::get(&address, "/api/hello", Duration::from_secs(5))
        .await
        .map_err(|error| {
            log::warn!("link: no answer from {address}: {error}");
            AppError::HostUnreachable
        })?
        .json()
        .map_err(|_| AppError::HostUnreachable)?;
    if hello.instance.as_deref() == Some(crate::remote::instance_id()) {
        return Err(AppError::JoinSelf);
    }
    let ours = app.package_info().version.to_string();
    if hello.protocol != sw_server::PROTOCOL_VERSION || hello.version != ours {
        return Err(AppError::HostVersion {
            host: hello.version,
            ours,
        });
    }
    log::info!("link: asking {address} to let this computer join");
    let response = plain_http::post_json(
        &address,
        "/api/pair",
        &serde_json::json!({ "code": code.trim(), "name": name.trim() }),
        PAIRING_TIMEOUT,
    )
    .await
    .map_err(|error| {
        log::warn!("link: pairing with {address} failed: {error}");
        AppError::HostUnreachable
    })?;
    if !response.ok() {
        return Err(AppError::FromHost(response.text()));
    }
    let answer: PairAnswer = response
        .json()
        .map_err(|error| AppError::Conversion(error.to_string()))?;
    let settings = LinkSettings {
        address,
        token: answer.token,
        host_name: hello.name.unwrap_or_else(|| "SimpleWorship".into()),
        role: answer.role,
    };
    save(&app, Some(&settings))?;
    {
        let state = app.state::<LinkState>();
        let mut linked = lock(&state);
        linked.settings = Some(settings);
        linked.removed = false;
    }
    // This computer's phone remote and browser source are its own
    // service's; a joined computer runs the host's.
    crate::remote::stop_for_link(&app);
    connect(&app);
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(status(&app))
}

/// Leaves the host: this computer runs its own service again.
#[tauri::command]
pub fn link_leave(app: AppHandle) -> AppResult<LinkStatus> {
    let state = app.state::<LinkState>();
    let stop = {
        let mut linked = lock(&state);
        linked.settings = None;
        linked.device = None;
        linked.connected = false;
        linked.outgoing = None;
        linked.stop.take()
    };
    if let Some(stop) = stop {
        let _ = stop.send(());
    }
    save(&app, None)?;
    crate::remote::start_if_enabled(&app);
    let _ = app.emit(CHANGED_EVENT, ());
    announce_status(&app);
    Ok(status(&app))
}

/// The main computer stopped answering: carries on its service here, from
/// the last state heard, and leaves the link. This computer's other windows
/// reload (to show its own pictures and videos); the operator window shows
/// the result first, then reloads itself.
#[tauri::command]
pub fn link_take_over(app: AppHandle) -> AppResult<TakeOverResult> {
    let state = app.state::<LinkState>();
    let (service, stop) = {
        let mut linked = lock(&state);
        if linked.connected {
            return Err(AppError::FromHost(
                "the main computer is answering again: carry on from there".into(),
            ));
        }
        let service = linked.copy.take().ok_or(AppError::HostUnreachable)?;
        linked.settings = None;
        linked.device = None;
        linked.outgoing = None;
        linked.lost_since = None;
        linked.host_lost = false;
        (service, linked.stop.take())
    };
    if let Some(stop) = stop {
        let _ = stop.send(());
    }
    save(&app, None)?;

    let offset = state.clock_offset_ms.load(Ordering::Relaxed);
    let service = service.shifted(offset);
    let missing_media = missing_media(&app, &service);
    log::info!(
        "link: took over the service ({missing_media} media files are not on this computer)"
    );
    crate::commands::engine::take_over(&app, &app.state::<AppState>(), service)?;
    crate::remote::start_if_enabled(&app);
    announce_status(&app);
    let _ = app.emit_filter(CHANGED_EVENT, (), |target| match target {
        tauri::EventTarget::WebviewWindow { label } => label != MAIN_WINDOW,
        _ => true,
    });
    Ok(TakeOverResult { missing_media })
}

/// How many media files the service uses that are not on this computer.
fn missing_media(app: &AppHandle, service: &EngineSnapshot) -> u32 {
    let folder = &app.state::<AppState>().media_folders.media;
    let missing = media_files(service)
        .iter()
        .filter(|file| !folder.join(file).is_file())
        .count();
    u32::try_from(missing).unwrap_or(u32::MAX)
}

/// Every picture, video and sound file the service refers to: anything
/// shaped like a media reference (`{ "id", "file" }`) anywhere in it, so
/// backgrounds, slides, schedule items and the music are all counted.
fn media_files(service: &EngineSnapshot) -> std::collections::BTreeSet<String> {
    fn walk(value: &Value, found: &mut std::collections::BTreeSet<String>) {
        match value {
            Value::Object(map) => {
                if let (Some(Value::String(_)), Some(Value::String(file))) =
                    (map.get("id"), map.get("file"))
                {
                    found.insert(file.clone());
                }
                map.values().for_each(|value| walk(value, found));
            }
            Value::Array(items) => items.iter().for_each(|value| walk(value, found)),
            _ => {}
        }
    }
    let mut found = std::collections::BTreeSet::new();
    if let Ok(value) = serde_json::to_value(service) {
        walk(&value, &mut found);
    }
    found
}

/// `192.168.1.20` → `http://192.168.1.20:7270`.
fn normalize(address: &str) -> String {
    let address = address.trim().trim_end_matches('/');
    let address = if address.contains("://") {
        address.to_owned()
    } else {
        format!("http://{address}")
    };
    let has_port = address
        .rsplit_once(':')
        .is_some_and(|(_, port)| port.chars().all(|c| c.is_ascii_digit()) && !port.is_empty());
    if has_port {
        address
    } else {
        format!("{address}:{}", sw_server::DEFAULT_PORT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_get_a_scheme_and_the_usual_port() {
        assert_eq!(normalize("192.168.1.20"), "http://192.168.1.20:7270");
        assert_eq!(
            normalize("http://192.168.1.20:8000/"),
            "http://192.168.1.20:8000"
        );
        assert_eq!(normalize(" main-pc.local "), "http://main-pc.local:7270");
    }

    #[test]
    fn finds_the_media_a_service_uses() {
        let mut engine = sw_core::Engine::new();
        let picture = sw_core::MediaItem {
            id: Id::new(),
            name: "Sunrise".into(),
            kind: sw_core::MediaKind::Image,
            file: "sunrise.jpg".into(),
            width: None,
            height: None,
            duration_ms: None,
            size_bytes: 0,
            created_at: 0,
            collection: None,
        };
        engine
            .dispatch(Command::PreviewMedia {
                media: Box::new(picture),
            })
            .unwrap();
        engine.dispatch(Command::GoLive).unwrap();
        let files = media_files(&engine.snapshot());
        assert!(files.contains("sunrise.jpg"), "{files:?}");
    }

    #[test]
    fn only_this_computers_preview_is_passed_on() {
        let mine = Id::new();
        let other = Id::new();
        let preview = |operator| EngineEvent::PreviewChanged {
            operator,
            preview: None,
        };
        assert!(matches!(
            for_this_computer(preview(mine), Some(mine)),
            Some(EngineEvent::PreviewChanged { operator, .. }) if operator == LOCAL_OPERATOR
        ));
        assert!(for_this_computer(preview(other), Some(mine)).is_none());
        assert!(for_this_computer(EngineEvent::LiveChanged { live: None }, Some(mine)).is_some());
    }
}

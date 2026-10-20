//! The HTTP and WebSocket server on the church's network.
//!
//! - `GET /` and other paths: the remote web app (files from `static_dir`).
//! - `POST /api/pair`: a phone sends the pairing code and its name; the
//!   answer waits until the operator allows or refuses it.
//! - `GET /ws?token=…`: the live connection (see [`crate::protocol`]).
//! - `GET /api/check?token=…`: whether this device is still allowed.
//! - `GET /media/{file}` and `/thumbnails/{file}` (with `?token=`): pictures
//!   for slide thumbnails (and, for the browser source, videos).
//! - `GET /stream?key=…`: the stream look, for OBS or vMix to show as a
//!   browser source. It connects to `/ws` with the stream key, which can
//!   only watch: it never sends commands.
//!
//! Phones and the browser source can be allowed separately. While phones
//! are not allowed, their tokens are answered "unavailable" rather than
//! "unauthorized", so phones keep their pairing and reconnect later.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use axum::Json;
use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path as UrlPath, Query, Request, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use sw_core::{EngineEvent, Id, time};
use tokio::sync::{broadcast, oneshot};
use tower::ServiceExt;
use tower_http::services::ServeFile;
use ts_rs::TS;

use crate::Host;
use crate::pairing::{self, Device, Pairing};
use crate::protocol::{ClientMessage, ServerMessage};
use crate::roles::{self, Role};

/// How long a phone waits for the operator to allow it.
const APPROVAL_TIMEOUT: Duration = Duration::from_secs(120);

/// How many events may queue for a slow phone before it is sent the whole
/// state again instead.
const EVENT_QUEUE: usize = 256;

/// Something every connection needs to hear about.
#[derive(Clone, Debug)]
enum Signal {
    /// An engine event, already as the JSON to send.
    Event(Arc<str>),
    RoleChanged(Id, Role),
    Removed(Id),
    /// Phones are no longer allowed (the browser source stays connected).
    PhonesOff,
    Shutdown,
}

/// Who is on the other end of a connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Viewer {
    /// A paired phone or tablet.
    Phone(Id),
    /// A browser source (OBS, vMix) showing the stream look. It can only
    /// watch.
    Stream,
}

/// A phone asking to be allowed, shown to the operator.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "remote.ts")]
pub struct PairRequest {
    /// Identifies the request (to answer it).
    pub id: Id,
    /// The name the phone gave.
    pub name: String,
}

/// Why the server could not start.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// The port is in use (another program, or SimpleWorship twice).
    #[error("could not use port {port}: {source}")]
    Port {
        /// The port.
        port: u16,
        /// What the system said.
        source: std::io::Error,
    },
}

/// Settings for [`RemoteServer::start`].
#[derive(Clone, Debug)]
pub struct ServerConfig {
    /// The port to listen on (all network interfaces).
    pub port: u16,
    /// The built remote web app.
    pub static_dir: PathBuf,
    /// Allow paired phones (the phone remote is on).
    pub phones: bool,
    /// The key of the browser source link, if the browser source is on.
    pub stream_key: Option<String>,
}

/// State shared by every request.
struct Shared {
    host: Arc<dyn Host>,
    devices: Mutex<Vec<Device>>,
    pairing: Mutex<Pairing>,
    pending: Mutex<HashMap<Id, oneshot::Sender<Option<Role>>>>,
    /// Open connections per device.
    connections: Mutex<HashMap<Id, usize>>,
    signals: broadcast::Sender<Signal>,
    static_dir: PathBuf,
    /// Whether paired phones may connect.
    phones: AtomicBool,
    /// The hash of the browser source's key, if it is on.
    stream_key: Mutex<Option<String>>,
    /// Stands for the browser source in signals (as a device id would).
    stream_id: Id,
}

/// Locks a mutex, carrying on if a thread panicked while holding it (the
/// data is still usable: lists and maps, never half-written).
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The running server. Dropping it does not stop it; call [`RemoteServer::stop`].
pub struct RemoteServer {
    shared: Arc<Shared>,
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
    port: u16,
}

impl RemoteServer {
    /// Starts listening on `config.port`, allowing the given devices.
    pub async fn start(
        config: ServerConfig,
        host: Arc<dyn Host>,
        devices: Vec<Device>,
    ) -> Result<Self, ServerError> {
        let listener = tokio::net::TcpListener::bind(("0.0.0.0", config.port))
            .await
            .map_err(|source| ServerError::Port {
                port: config.port,
                source,
            })?;
        let port = listener
            .local_addr()
            .map(|address| address.port())
            .unwrap_or(config.port);

        let (signals, _) = broadcast::channel(EVENT_QUEUE);
        let shared = Arc::new(Shared {
            host,
            devices: Mutex::new(devices),
            pairing: Mutex::new(Pairing::default()),
            pending: Mutex::new(HashMap::new()),
            connections: Mutex::new(HashMap::new()),
            signals,
            static_dir: config.static_dir,
            phones: AtomicBool::new(config.phones),
            stream_key: Mutex::new(config.stream_key.as_deref().map(pairing::hash_token)),
            stream_id: Id::new(),
        });

        let router = Router::new()
            .route("/api/hello", get(hello))
            .route("/api/pair", post(pair))
            .route("/api/check", get(check))
            .route("/ws", get(connect))
            .route("/stream", get(stream_page))
            .route("/media/{file}", get(media_file))
            .route("/thumbnails/{file}", get(thumbnail_file))
            .fallback(get(static_file))
            .with_state(shared.clone());

        let (shutdown, stopped) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let serve = axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            });
            if let Err(error) = serve.await {
                eprintln!("the remote server stopped: {error}");
            }
        });

        Ok(Self {
            shared,
            shutdown: Mutex::new(Some(shutdown)),
            port,
        })
    }

    /// The port it is listening on.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Passes an engine event on to every connected device.
    pub fn publish(&self, event: &EngineEvent) {
        let message = ServerMessage::Event {
            event: Box::new(event.clone()),
        };
        if let Ok(json) = serde_json::to_string(&message) {
            // No receivers (nobody connected) is fine.
            let _ = self.shared.signals.send(Signal::Event(json.into()));
        }
    }

    /// A new pairing code for "Connect a Phone".
    pub fn new_pairing_code(&self) -> String {
        lock(&self.shared.pairing).new_code()
    }

    /// Stops accepting the current pairing code.
    pub fn cancel_pairing(&self) {
        lock(&self.shared.pairing).cancel();
    }

    /// The operator's answer to a [`PairRequest`]: a role, or `None` to refuse.
    pub fn answer(&self, request: Id, role: Option<Role>) {
        if let Some(reply) = lock(&self.shared.pending).remove(&request) {
            let _ = reply.send(role);
        }
    }

    /// The devices allowed to connect.
    pub fn devices(&self) -> Vec<Device> {
        lock(&self.shared.devices).clone()
    }

    /// The devices connected right now.
    pub fn connected(&self) -> Vec<Id> {
        lock(&self.shared.connections).keys().copied().collect()
    }

    /// Changes what a device may do, straight away.
    pub fn set_role(&self, device: Id, role: Role) {
        let devices = {
            let mut devices = lock(&self.shared.devices);
            if let Some(found) = devices.iter_mut().find(|known| known.id == device) {
                found.role = role;
            }
            devices.clone()
        };
        self.shared.host.devices_changed(&devices);
        let _ = self.shared.signals.send(Signal::RoleChanged(device, role));
    }

    /// Removes a device: it is disconnected and can no longer connect.
    pub fn remove(&self, device: Id) {
        let devices = {
            let mut devices = lock(&self.shared.devices);
            devices.retain(|known| known.id != device);
            devices.clone()
        };
        self.shared.host.devices_changed(&devices);
        let _ = self.shared.signals.send(Signal::Removed(device));
    }

    /// Allows paired phones, or stops allowing them (they are disconnected,
    /// but keep their pairing for when phones are allowed again).
    pub fn set_phones(&self, allowed: bool) {
        let was = self.shared.phones.swap(allowed, Ordering::SeqCst);
        if was && !allowed {
            let _ = self.shared.signals.send(Signal::PhonesOff);
        }
    }

    /// Sets the browser source's key, or turns the browser source off
    /// (`None`). Browser sources using an old key are disconnected.
    pub fn set_stream_key(&self, key: Option<&str>) {
        let hash = key.map(pairing::hash_token);
        let mut current = lock(&self.shared.stream_key);
        if *current != hash {
            *current = hash;
            let _ = self
                .shared
                .signals
                .send(Signal::Removed(self.shared.stream_id));
        }
    }

    /// Stops the server and disconnects every device.
    pub fn stop(&self) {
        let _ = self.shared.signals.send(Signal::Shutdown);
        if let Some(shutdown) = lock(&self.shutdown).take() {
            let _ = shutdown.send(());
        }
        // Refuse any phone still waiting for approval.
        lock(&self.shared.pending).clear();
    }
}

// ── Routes ───────────────────────────────────────────────────────────────

/// Lets a phone check it found SimpleWorship.
async fn hello() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "app": "SimpleWorship", "protocol": crate::PROTOCOL_VERSION }))
}

#[derive(Deserialize)]
struct PairBody {
    code: String,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PairAnswer {
    token: String,
    name: String,
    role: Role,
}

/// A phone asks to connect with the pairing code; waits for the operator.
async fn pair(State(shared): State<Arc<Shared>>, Json(body): Json<PairBody>) -> Response {
    if !shared.phones.load(Ordering::SeqCst) {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "The phone remote is turned off.",
        )
            .into_response();
    }
    if !lock(&shared.pairing).check(&body.code) {
        return (
            StatusCode::UNAUTHORIZED,
            "That code is wrong or has expired.",
        )
            .into_response();
    }
    let request = PairRequest {
        id: Id::new(),
        name: body.name.trim().chars().take(60).collect(),
    };
    let (reply, answer) = oneshot::channel();
    lock(&shared.pending).insert(request.id, reply);
    shared.host.pairing_requested(request.clone());

    let role = tokio::time::timeout(APPROVAL_TIMEOUT, answer).await;
    lock(&shared.pending).remove(&request.id);
    let Ok(Ok(Some(role))) = role else {
        return (
            StatusCode::FORBIDDEN,
            "The operator did not allow this device.",
        )
            .into_response();
    };

    let (device, token) = pairing::new_device(&request.name, role);
    let devices = {
        let mut devices = lock(&shared.devices);
        devices.push(device.clone());
        devices.clone()
    };
    shared.host.devices_changed(&devices);
    Json(PairAnswer {
        token,
        name: device.name,
        role,
    })
    .into_response()
}

#[derive(Deserialize)]
struct TokenQuery {
    token: String,
}

/// Who a token (or the browser source's key) belongs to. Refused with
/// "unauthorized" when nobody has it, or "unavailable" for a paired phone
/// while phones are not allowed.
fn authorize(shared: &Shared, token: &str) -> Result<Viewer, StatusCode> {
    if lock(&shared.stream_key).as_deref() == Some(pairing::hash_token(token).as_str()) {
        return Ok(Viewer::Stream);
    }
    let device = pairing::find_device(&lock(&shared.devices), token).map(|device| device.id);
    match device {
        Some(_) if !shared.phones.load(Ordering::SeqCst) => Err(StatusCode::SERVICE_UNAVAILABLE),
        Some(id) => Ok(Viewer::Phone(id)),
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Whether a token is still allowed: lets a phone tell "removed" apart from
/// "the Wi-Fi dropped" (or "phones are off for now") when its connection
/// fails.
async fn check(State(shared): State<Arc<Shared>>, Query(query): Query<TokenQuery>) -> StatusCode {
    match authorize(&shared, &query.token) {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(status) => status,
    }
}

/// Opens the live connection for a paired device or the browser source.
async fn connect(
    State(shared): State<Arc<Shared>>,
    Query(query): Query<TokenQuery>,
    upgrade: WebSocketUpgrade,
) -> Response {
    match authorize(&shared, &query.token) {
        Ok(viewer) => upgrade.on_upgrade(move |socket| run_connection(shared, viewer, socket)),
        Err(StatusCode::SERVICE_UNAVAILABLE) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "The phone remote is turned off.",
        )
            .into_response(),
        Err(status) => (status, "This device is not allowed.").into_response(),
    }
}

/// One connection: sends the state, passes on events, and carries out
/// what a phone asks if its role allows.
async fn run_connection(shared: Arc<Shared>, viewer: Viewer, socket: WebSocket) {
    let device_id = match viewer {
        Viewer::Phone(id) => id,
        Viewer::Stream => shared.stream_id,
    };
    let (mut sender, mut receiver) = socket.split();
    let mut signals = shared.signals.subscribe();
    let Some(first) = welcome(&shared, viewer) else {
        return;
    };
    if sender.send(Message::Text(first.into())).await.is_err() {
        return;
    }
    if let Viewer::Phone(id) = viewer {
        connection_changed(&shared, id, true);
    }

    loop {
        tokio::select! {
            incoming = receiver.next() => {
                let Some(Ok(message)) = incoming else { break };
                match message {
                    Message::Text(text) => {
                        let answer = handle(&shared, viewer, &text).await;
                        if sender.send(Message::Text(answer.into())).await.is_err() {
                            break;
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            signal = signals.recv() => {
                let outgoing = match signal {
                    Ok(Signal::Event(json)) => Some(json.to_string()),
                    Ok(Signal::RoleChanged(id, role)) if id == device_id => {
                        serde_json::to_string(&ServerMessage::RoleChanged { role }).ok()
                    }
                    Ok(Signal::Removed(id)) if id == device_id => {
                        let removed = serde_json::to_string(&ServerMessage::Removed).ok();
                        if let Some(removed) = removed {
                            let _ = sender.send(Message::Text(removed.into())).await;
                        }
                        break;
                    }
                    Ok(Signal::PhonesOff) if viewer != Viewer::Stream => break,
                    Ok(Signal::Shutdown) => break,
                    Ok(_) => None,
                    // Fell behind: send the whole state again.
                    Err(broadcast::error::RecvError::Lagged(_)) => welcome(&shared, viewer),
                    Err(broadcast::error::RecvError::Closed) => break,
                };
                if let Some(json) = outgoing
                    && sender.send(Message::Text(json.into())).await.is_err()
                {
                    break;
                }
            }
        }
    }
    let _ = sender.send(Message::Close(None)).await;
    if let Viewer::Phone(id) = viewer {
        connection_changed(&shared, id, false);
    }
}

/// The welcome message: who the device is and the whole state.
fn welcome(shared: &Shared, viewer: Viewer) -> Option<String> {
    let (name, role) = match viewer {
        Viewer::Stream => ("Browser source".to_owned(), Role::Stage),
        Viewer::Phone(device_id) => {
            let mut devices = lock(&shared.devices);
            let device = devices.iter_mut().find(|device| device.id == device_id)?;
            device.last_seen = time::now();
            (device.name.clone(), device.role)
        }
    };
    let message = ServerMessage::Welcome {
        name,
        role,
        snapshot: Box::new(shared.host.snapshot()),
    };
    serde_json::to_string(&message).ok()
}

fn connection_changed(shared: &Shared, device_id: Id, opened: bool) {
    let connected = {
        let mut connections = lock(&shared.connections);
        let count = connections.entry(device_id).or_insert(0);
        if opened {
            *count += 1;
        } else {
            *count = count.saturating_sub(1);
        }
        connections.retain(|_, count| *count > 0);
        connections.keys().copied().collect::<Vec<_>>()
    };
    shared.host.connections_changed(connected);
    if opened {
        let devices = lock(&shared.devices).clone();
        shared.host.devices_changed(&devices);
    }
}

/// Carries out one message from a device, if its role allows, and returns
/// the answer to send.
async fn handle(shared: &Arc<Shared>, viewer: Viewer, text: &str) -> String {
    // A phone's role is looked up each time: the operator may change it.
    let role = match viewer {
        Viewer::Stream => None,
        Viewer::Phone(device_id) => lock(&shared.devices)
            .iter()
            .find(|device| device.id == device_id)
            .map(|device| device.role),
    };
    let answer = |id: u32, result: Result<serde_json::Value, String>| {
        let (value, error) = match result {
            Ok(value) => (Some(value), None),
            Err(error) => (None, Some(error)),
        };
        serde_json::to_string(&ServerMessage::Result { id, value, error }).unwrap_or_default()
    };
    let Some(role) = role else {
        return answer(0, Err("This device is not allowed.".into()));
    };

    match serde_json::from_str::<ClientMessage>(text) {
        Ok(ClientMessage::Dispatch { id, command }) => {
            let host = shared.host.clone();
            let result = tokio::task::spawn_blocking(move || {
                if !roles::may_dispatch(role, &command, &host.snapshot()) {
                    return Err("This device is not allowed to do that.".to_owned());
                }
                host.dispatch(command)
            })
            .await
            .unwrap_or_else(|error| Err(error.to_string()));
            answer(id, result.map(|()| serde_json::Value::Null))
        }
        Ok(ClientMessage::Call { id, method, params }) => {
            if !roles::may_call(role, &method) {
                return answer(id, Err("This device is not allowed to do that.".into()));
            }
            let host = shared.host.clone();
            let result = tokio::task::spawn_blocking(move || host.call(&method, params))
                .await
                .unwrap_or_else(|error| Err(error.to_string()));
            answer(id, result)
        }
        Err(error) => answer(0, Err(format!("not understood: {error}"))),
    }
}

/// A file from the media library: pictures for phones (which show video
/// backgrounds as stills), pictures and videos for the browser source.
async fn media_file(
    State(shared): State<Arc<Shared>>,
    UrlPath(file): UrlPath<String>,
    Query(query): Query<TokenQuery>,
    request: Request,
) -> Response {
    serve_media(
        &shared,
        crate::MediaFolder::Media,
        &file,
        &query.token,
        request,
    )
    .await
}

/// A thumbnail (the still of a video background).
async fn thumbnail_file(
    State(shared): State<Arc<Shared>>,
    UrlPath(file): UrlPath<String>,
    Query(query): Query<TokenQuery>,
    request: Request,
) -> Response {
    serve_media(
        &shared,
        crate::MediaFolder::Thumbnails,
        &file,
        &query.token,
        request,
    )
    .await
}

async fn serve_media(
    shared: &Shared,
    folder: crate::MediaFolder,
    file: &str,
    token: &str,
    request: Request,
) -> Response {
    let viewer = match authorize(shared, token) {
        Ok(viewer) => viewer,
        Err(status) => return status.into_response(),
    };
    let kind = content_type(file);
    let allowed =
        kind.starts_with("image/") || (viewer == Viewer::Stream && kind.starts_with("video/"));
    if !allowed || !is_plain_name(file) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(path) = shared.host.media_path(folder, file) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    // Answers range requests, so videos can start anywhere and loop.
    match ServeFile::new(&path).oneshot(request).await {
        Ok(response) => response.into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// The browser source page (see `apps/remote/stream.html`).
async fn stream_page(State(shared): State<Arc<Shared>>) -> Response {
    let page = shared.static_dir.join("stream.html");
    if page.is_file() {
        send_file(&page, "text/html; charset=utf-8").await
    } else {
        (
            StatusCode::NOT_FOUND,
            "The stream page is missing from this installation.",
        )
            .into_response()
    }
}

/// The remote web app's files; any other path gets `index.html`, so links
/// inside the app work.
async fn static_file(State(shared): State<Arc<Shared>>, uri: axum::http::Uri) -> Response {
    let wanted = uri.path().trim_start_matches('/');
    let relative = Path::new(wanted);
    let safe = relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)));
    let path = shared.static_dir.join(relative);
    if safe && !wanted.is_empty() && path.is_file() {
        return send_file(&path, content_type(wanted)).await;
    }
    let index = shared.static_dir.join("index.html");
    if index.is_file() {
        send_file(&index, "text/html; charset=utf-8").await
    } else {
        (
            StatusCode::NOT_FOUND,
            "The remote app is missing from this installation.",
        )
            .into_response()
    }
}

async fn send_file(path: &Path, kind: &'static str) -> Response {
    match tokio::fs::read(path).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, kind)], bytes).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// A file name with no folders in it.
fn is_plain_name(file: &str) -> bool {
    !file.is_empty() && !file.contains(['/', '\\']) && file != "." && file != ".."
}

fn content_type(file: &str) -> &'static str {
    let extension = file.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match extension.as_str() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "webmanifest" => "application/manifest+json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_file_names_are_served_as_media() {
        assert!(is_plain_name("0192.jpg"));
        assert!(!is_plain_name("../secret.jpg"));
        assert!(!is_plain_name("a/b.jpg"));
    }

    #[test]
    fn knows_the_types_the_app_needs() {
        assert_eq!(content_type("index.html"), "text/html; charset=utf-8");
        assert_eq!(content_type("app.JS"), "text/javascript; charset=utf-8");
        assert_eq!(content_type("thumb.jpg"), "image/jpeg");
    }
}

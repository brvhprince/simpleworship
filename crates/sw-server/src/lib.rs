//! # sw-server
//!
//! SimpleWorship on the church's network: the phone remote, and stage
//! displays on tablets. Phones connect to the computer running
//! SimpleWorship over the local Wi-Fi; nothing goes through the internet.
//!
//! A phone is just another client of the same engine: it sends the same
//! [`Command`]s as the operator window and receives the same
//! [`EngineEvent`](sw_core::EngineEvent)s (see [`protocol`]). Each paired
//! device has a [`Role`] that limits what it may do ([`roles`]), checked
//! here for every command.
//!
//! This crate knows nothing about the desktop app: the app implements
//! [`Host`] to give the server the engine, the library and the media.

pub mod pairing;
pub mod protocol;
pub mod roles;
pub mod server;

use std::path::PathBuf;

use serde_json::Value;
use sw_core::{Command, EngineSnapshot, Id};

pub use pairing::Device;
pub use roles::Role;
pub use server::{PairRequest, RemoteServer, ServerConfig, ServerError};

/// Bumped when the messages change in a way old remotes cannot handle.
/// Version 2: each device has its own preview pane, the welcome says which
/// device it is, and computers can join (see `Role::Mirror`).
pub const PROTOCOL_VERSION: u32 = 2;

/// The usual port. Changeable in the app's settings.
pub const DEFAULT_PORT: u16 = 7270;

/// Which media folder a file is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaFolder {
    /// The media library's files.
    Media,
    /// Thumbnails (stills of videos).
    Thumbnails,
}

/// What the server needs from the app around it. Called from background
/// threads, so it must be shareable.
pub trait Host: Send + Sync + 'static {
    /// The whole engine state, with `device`'s own preview pane (each
    /// operator has one; see `sw_core::LOCAL_OPERATOR`).
    fn snapshot(&self, device: Id) -> EngineSnapshot;
    /// Carries out a command from `device` (already checked against its
    /// role), exactly as if an operator had done it, using `device`'s own
    /// preview pane.
    fn dispatch(&self, device: Id, command: Command) -> Result<(), String>;
    /// Answers a question from `device`, such as `"songs.search"`, or
    /// carries out a request (anything it shows in a preview goes to
    /// `device`'s own).
    fn call(&self, device: Id, method: &str, params: Value) -> Result<Value, String>;
    /// Where a media file is, if it exists.
    fn media_path(&self, folder: MediaFolder, file: &str) -> Option<PathBuf>;
    /// A phone has the right code and asks to be allowed: ask the operator,
    /// then call [`RemoteServer::answer`].
    fn pairing_requested(&self, request: PairRequest);
    /// The list of devices changed (one was added, removed, renamed or
    /// connected): save it.
    fn devices_changed(&self, devices: &[Device]);
    /// Which devices are connected now.
    fn connections_changed(&self, connected: Vec<Id>);
}

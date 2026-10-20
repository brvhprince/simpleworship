//! The messages a remote and the server exchange over the WebSocket, as
//! JSON. The remote app is served by the same SimpleWorship, so both sides
//! always speak the same version.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sw_core::{Command, EngineEvent, EngineSnapshot, Id};
use ts_rs::TS;

use crate::roles::Role;

/// Sent by a remote.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "remote.ts")]
pub enum ClientMessage {
    /// Send a command to the engine. Answered with a `result` with `id`.
    Dispatch {
        /// Chosen by the remote, to match the answer.
        id: u32,
        /// The command.
        command: Command,
    },
    /// Ask the host something (search songs, look up scripture…).
    Call {
        /// Chosen by the remote, to match the answer.
        id: u32,
        /// What to ask, e.g. `"songs.search"`.
        method: String,
        /// The question's details.
        #[ts(type = "unknown")]
        params: Value,
    },
}

/// Sent by the server.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "remote.ts")]
pub enum ServerMessage {
    /// Right after connecting: who the device is, and the whole state.
    Welcome {
        /// The device's id: the operator its preview pane belongs to.
        device: Id,
        /// The device's name.
        name: String,
        /// What it may do.
        role: Role,
        /// Everything the engine knows.
        snapshot: Box<EngineSnapshot>,
    },
    /// Something changed.
    Event {
        /// What changed.
        event: Box<EngineEvent>,
    },
    /// The answer to a `dispatch` or `call`.
    Result {
        /// The id the remote sent.
        id: u32,
        /// The answer, if it worked.
        #[ts(type = "unknown")]
        value: Option<Value>,
        /// Why it did not, if it did not.
        error: Option<String>,
    },
    /// The operator changed this device's role.
    RoleChanged {
        /// The new role.
        role: Role,
    },
    /// The operator removed this device: forget the token.
    Removed,
    /// Something outside the engine changed (the media library, songs,
    /// Bibles, settings, the schedule file), for computers that joined this
    /// one to refresh what they show. `name` is the desktop app's event for
    /// it; phones ignore these.
    Notice {
        /// What changed: the desktop app's event name, e.g. "media://changed".
        name: String,
        /// The event's payload, if it has one.
        #[ts(type = "unknown")]
        payload: Value,
    },
}

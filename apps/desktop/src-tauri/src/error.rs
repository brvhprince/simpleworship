//! The error type returned by every IPC command.

use serde::{Serialize, Serializer};
use thiserror::Error;

/// Anything that can go wrong while handling a command from the UI.
///
/// Errors reach JavaScript as a plain message string, so a rejected
/// `invoke()` promise can be shown to the operator as-is.
#[derive(Debug, Error)]
pub enum AppError {
    /// The library database failed.
    #[error(transparent)]
    Database(#[from] sw_db::DbError),

    /// The engine rejected a command.
    #[error(transparent)]
    Engine(#[from] sw_core::EngineError),

    /// Tauri failed, e.g. while creating a window.
    #[error(transparent)]
    Tauri(#[from] tauri::Error),

    /// A schedule file could not be read or written.
    #[error(transparent)]
    ScheduleFile(#[from] sw_files::ScheduleFileError),

    /// The operating system refused a file operation.
    #[error("could not read or write the file: {0}")]
    Io(#[from] std::io::Error),

    /// "Save" was asked for a schedule that has never been saved; the UI
    /// should ask where to save it ("Save As") instead.
    #[error("choose where to save the schedule first")]
    NoSchedulePath,

    /// A song id that is not in the library.
    #[error("that song is no longer in the library")]
    SongNotFound,

    /// A Bible file could not be imported.
    #[error(transparent)]
    BibleImport(#[from] sw_bible::import::ImportError),

    /// A Bible id that is not installed (any more).
    #[error("that Bible is no longer installed")]
    BibleNotFound,

    /// Scripture was asked for without choosing a translation.
    #[error("choose a Bible translation first")]
    NoTranslation,

    /// A file that is not an image or video we can show.
    #[error(
        "only images (PNG, JPEG, GIF, WebP), videos (MP4, MOV, WebM) and audio (MP3, M4A, WAV) can be imported"
    )]
    UnsupportedMedia,

    /// A media id that is not in the library (any more).
    #[error("that media file is no longer in the library")]
    MediaNotFound,

    /// A presentation could not be converted to PDF.
    #[error("{0}")]
    Conversion(String),

    /// The host computer this one joined cannot be reached right now.
    #[error(
        "the main SimpleWorship computer cannot be reached; check it is on and on the same network"
    )]
    HostUnreachable,

    /// The host computer refused or could not carry out a request.
    #[error("{0}")]
    FromHost(String),

    /// The host computer runs another version of SimpleWorship.
    #[error(
        "that computer runs SimpleWorship {host}, this one {ours}: update both to the same version first"
    )]
    HostVersion {
        /// The host's version.
        host: String,
        /// This computer's version.
        ours: String,
    },

    /// The address given is this computer's own.
    #[error("that address is this computer; type the address of the main computer instead")]
    JoinSelf,

    /// Looking on the network failed.
    #[error("could not look on the network: {0}")]
    Network(String),

    /// A command only NDI's hidden windows may use came from elsewhere.
    #[error("only an NDI output window can do that")]
    NotAnNdiWindow,

    /// The phone remote is turned off.
    #[error("the phone remote is turned off; turn it on in Settings")]
    RemoteOff,

    /// The computer is not on a network phones can reach.
    #[error("this computer is not connected to a network; connect it to the church Wi-Fi first")]
    NoNetwork,

    /// A presentation id that is not in the library (any more).
    #[error("that presentation is no longer in the library")]
    PresentationNotFound,

    /// A theme id that is not in the library (any more).
    #[error("that theme is no longer in the library")]
    ThemeNotFound,

    /// A monitor index that does not exist (any more).
    #[error("monitor {0} is not connected")]
    MonitorNotFound(usize),
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

/// The result type of IPC commands.
pub type AppResult<T> = Result<T, AppError>;

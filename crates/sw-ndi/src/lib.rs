//! # sw-ndi
//!
//! SimpleWorship on NDI: the stream look (and, if wanted, the projector's
//! picture) as NDI sources on the church network, with transparency, for
//! vMix, OBS (with the NDI plugin), TriCaster and other NDI receivers.
//!
//! How a picture gets out:
//!
//! 1. A hidden window draws the output with the same renderer as the
//!    projector (see `apps/desktop/src/ndi`).
//! 2. When it changes, [`snapshot`] asks the system's webview (WebKit on
//!    macOS, WebView2 on Windows) for a picture of the page: a [`Frame`].
//! 3. The [`Compositor`] keeps the current frame and blends the old one into
//!    the new one when the operator uses a fade.
//! 4. A [`Sender`] sends the compositor's picture 30 times a second through
//!    the NDI library ([`Runtime`]), which the user installs from ndi.video
//!    (SimpleWorship does not ship it). Without it, NDI is simply off.
//!
//! The pages are only redrawn when something changes, so videos show as a
//! still frame and alerts as a still banner.
//!
//! NDI® is a registered trademark of Vizrt NDI AB.

#![deny(unsafe_code)]

mod compositor;
mod frame;
mod runtime;
pub mod snapshot;

pub use compositor::Compositor;
pub use frame::Frame;
pub use runtime::{NdiError, Runtime, Sender};

/// The picture size of every NDI source.
pub const WIDTH: u32 = 1920;
/// The picture size of every NDI source.
pub const HEIGHT: u32 = 1080;
/// Frames sent per second.
pub const FRAME_RATE: u32 = 30;

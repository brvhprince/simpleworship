//! Pictures of a webview's page, taken by the system's own webview: WebKit
//! on macOS, WebView2 on Windows. They work while the window is hidden or
//! off the screen, and keep the page's transparency (the window must be
//! made see-through, and the page must have no background).

use crate::frame::Frame;

#[cfg(target_os = "macos")]
mod mac;
#[cfg(windows)]
mod windows;

/// What [`snapshot`] calls with the picture, from any thread.
pub type Done = Box<dyn FnOnce(Result<Frame, String>) + Send + 'static>;

/// Takes a picture of `window`'s page at [`WIDTH`](crate::WIDTH) ×
/// [`HEIGHT`](crate::HEIGHT) pixels (the window should be that size) and
/// calls `done` with it. Returns at once; `done` is called a few dozen
/// milliseconds later.
pub fn snapshot(window: &tauri::WebviewWindow, done: Done) {
    #[cfg(target_os = "macos")]
    mac::snapshot(window, done);
    #[cfg(windows)]
    windows::snapshot(window, done);
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = window;
        done(Err("NDI is not available on this system yet".to_owned()));
    }
}

/// Whether [`snapshot`] works on this system.
pub const SUPPORTED: bool = cfg!(any(target_os = "macos", windows));

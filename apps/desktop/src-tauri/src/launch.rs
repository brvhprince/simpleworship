//! Opening schedule files from outside the app: double-clicking a `.sws`
//! file, or dropping it on SimpleWorship's icon.
//!
//! - **macOS** tells the running app with an "opened" event.
//! - **Windows and Linux** start the program with the file's path as an
//!   argument. If SimpleWorship is already running, the single-instance
//!   plugin hands that argument to the running copy instead of starting a
//!   second one (two copies would share one library).
//!
//! The file is not opened here: the operator window decides, so it can ask
//! about unsaved changes first. Until the window is ready, the path waits
//! in [`PendingOpen`].

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, State};

/// Sent to the operator window with a schedule file to open.
/// Keep in sync with `OPEN_FILE_EVENT` in `apps/desktop/src/lib/schedule-file.svelte.ts`.
pub const OPEN_FILE_EVENT: &str = "app://open-schedule";

/// Label of the operator window.
const MAIN_WINDOW: &str = "main";

/// A schedule file to open once the operator window is ready, and whether
/// it is (after that, files are sent straight to it).
#[derive(Default)]
pub struct PendingOpen(Mutex<(Option<PathBuf>, bool)>);

/// The first schedule file among program arguments, if any.
pub fn schedule_in_args<S: AsRef<str>>(args: &[S]) -> Option<PathBuf> {
    args.iter()
        .skip(1) // the program itself
        .map(|arg| PathBuf::from(arg.as_ref()))
        .find(|path| is_schedule(path))
}

fn is_schedule(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("sws"))
}

/// Opens a schedule file in the operator window (now, or once it is ready),
/// and brings the window to the front.
pub fn open_schedule(app: &AppHandle, path: PathBuf) {
    if !is_schedule(&path) {
        return;
    }
    let pending = app.state::<PendingOpen>();
    let mut guard = pending
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.1 {
        drop(guard);
        if let Err(error) = app.emit_to(MAIN_WINDOW, OPEN_FILE_EVENT, path.display().to_string()) {
            log::warn!("could not pass on the schedule to open: {error}");
        }
    } else {
        guard.0 = Some(path);
    }
    show_main_window(app);
}

/// Brings the operator window to the front (another copy was started, or a
/// file was opened from outside).
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Called by the operator window once it can open files: returns the file
/// SimpleWorship was started with, if any.
#[tauri::command]
pub fn launch_ready(pending: State<'_, PendingOpen>) -> Option<String> {
    let mut guard = pending
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.1 = true;
    guard.0.take().map(|path| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_schedule_among_the_arguments() {
        let args = ["SimpleWorship.exe", "--flag", r"C:\Services\Sunday.SWS"];
        assert_eq!(
            schedule_in_args(&args),
            Some(PathBuf::from(r"C:\Services\Sunday.SWS"))
        );
        assert_eq!(schedule_in_args(&["SimpleWorship", "notes.txt"]), None);
        assert_eq!(
            schedule_in_args(&["Sunday.sws"]),
            None,
            "the first is the program"
        );
    }
}

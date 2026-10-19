//! Output windows: the projector screen and the stage display.
//!
//! An output window either covers a whole monitor or, for rehearsing on a
//! laptop with one screen, sits in a normal window ("windowed" mode).
//!
//! Full-screen outputs use Tauri's *simple fullscreen*. On macOS it covers
//! the menu bar without moving the window to its own Space; native
//! fullscreen would make the projector Space swipe away together with the
//! operator window. On Windows and Linux it is regular borderless fullscreen
//! on the monitor the window was placed on.
//!
//! ## Sharing the operator's screen
//!
//! Outputs normally float above everything, so nothing can cover what the
//! audience sees. An output can also be put on the screen the operator window
//! is on (handy for testing on a laptop). Then, as in EasyWorship, the
//! operator window floats above the output instead, so the app can always be
//! reached. See [`update_stacking`].

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, LogicalSize, Manager, Monitor, PhysicalPosition, PhysicalSize, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder, Window, WindowEvent,
};
use ts_rs::TS;

use crate::error::{AppError, AppResult};

/// Label of the operator window, as configured in `tauri.conf.json`.
const MAIN_WINDOW: &str = "main";

/// Size of a windowed output: 960×540 is a 16:9 window that fits on a laptop.
const WINDOWED_SIZE: LogicalSize<f64> = LogicalSize::new(960.0, 540.0);

/// The kinds of output window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub enum OutputKind {
    /// What the congregation sees.
    Projector,
    /// The confidence monitor for musicians and speakers: current and next
    /// slide, plus a clock.
    Stage,
}

impl OutputKind {
    /// The Tauri window label. Each kind has at most one window.
    fn label(self) -> &'static str {
        match self {
            OutputKind::Projector => "output",
            OutputKind::Stage => "stage",
        }
    }

    /// The kind of output a window label belongs to, if any.
    fn from_label(label: &str) -> Option<Self> {
        [OutputKind::Projector, OutputKind::Stage]
            .into_iter()
            .find(|kind| kind.label() == label)
    }

    fn title(self) -> &'static str {
        match self {
            OutputKind::Projector => "SimpleWorship Output",
            OutputKind::Stage => "SimpleWorship Stage Display",
        }
    }

    /// The UI page to load (see `apps/desktop/src/App.svelte`).
    fn url(self) -> WebviewUrl {
        let view = match self {
            OutputKind::Projector => "output",
            OutputKind::Stage => "stage",
        };
        WebviewUrl::App(format!("index.html?view={view}").into())
    }
}

/// Where to show an output window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub enum OutputTarget {
    /// Cover a whole monitor, by its index in [`list_monitors`].
    Monitor {
        /// Index into the monitor list.
        index: usize,
    },
    /// A normal, resizable window. Useful when there is only one screen.
    Windowed,
}

/// A connected display, as shown in the output picker.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct MonitorInfo {
    /// Position in the list; pass it back in [`OutputTarget::Monitor`].
    pub index: usize,
    /// The name the operating system reports, e.g. "DELL U2720Q".
    pub name: String,
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
    /// Whether this is the primary display (usually the operator's screen).
    pub is_primary: bool,
}

/// Lists the connected monitors.
pub fn list_monitors(app: &AppHandle) -> AppResult<Vec<MonitorInfo>> {
    let primary = app.primary_monitor()?;
    let is_primary = |m: &Monitor| {
        primary
            .as_ref()
            .is_some_and(|p| p.position() == m.position())
    };

    let monitors = app.available_monitors()?;
    Ok(monitors
        .iter()
        .enumerate()
        .map(|(index, monitor)| MonitorInfo {
            index,
            name: monitor
                .name()
                .cloned()
                .unwrap_or_else(|| format!("Display {}", index + 1)),
            width: monitor.size().width,
            height: monitor.size().height,
            is_primary: is_primary(monitor),
        })
        .collect())
}

/// Which monitor each full-screen output window covers, keyed by window
/// label. A monitor is identified by its top-left corner, which is unique
/// among connected monitors. Windowed outputs have no entry.
///
/// Managed by Tauri (registered in `lib.rs`).
#[derive(Default)]
pub struct OutputPlacements(Mutex<HashMap<&'static str, PhysicalPosition<i32>>>);

impl OutputPlacements {
    fn set(&self, label: &'static str, monitor: Option<PhysicalPosition<i32>>) {
        let mut placements = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match monitor {
            Some(position) => placements.insert(label, position),
            None => placements.remove(label),
        };
    }

    fn get(&self, label: &str) -> Option<PhysicalPosition<i32>> {
        let placements = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        placements.get(label).copied()
    }
}

/// Opens an output window, or moves it if it is already open.
pub fn open_output(app: &AppHandle, kind: OutputKind, target: OutputTarget) -> AppResult<()> {
    let window = match app.get_webview_window(kind.label()) {
        Some(window) => window,
        None => WebviewWindowBuilder::new(app, kind.label(), kind.url())
            .title(kind.title())
            .background_color(tauri::window::Color(0, 0, 0, 255))
            // Shown only once it is in place, to avoid a flash on the wrong screen.
            .visible(false)
            .focused(false)
            .build()?,
    };

    let monitor = place(app, &window, target)?;
    app.state::<OutputPlacements>().set(kind.label(), monitor);
    window.show()?;
    update_stacking(app)?;

    // Keep keyboard focus on the operator window, so arrow keys keep working.
    if let Some(main) = app.get_webview_window(MAIN_WINDOW) {
        main.set_focus()?;
    }
    Ok(())
}

/// Closes an output window if it is open.
pub fn close_output(app: &AppHandle, kind: OutputKind) -> AppResult<()> {
    if let Some(window) = app.get_webview_window(kind.label()) {
        window.close()?;
    }
    Ok(())
}

/// Moves and sizes `window` for `target`. Returns the position of the
/// monitor it now covers, or `None` for a windowed output.
fn place(
    app: &AppHandle,
    window: &WebviewWindow,
    target: OutputTarget,
) -> AppResult<Option<PhysicalPosition<i32>>> {
    // Leave fullscreen first: a window cannot be moved while it is fullscreen.
    window.set_simple_fullscreen(false)?;

    match target {
        OutputTarget::Monitor { index } => {
            let monitors = app.available_monitors()?;
            let monitor = monitors
                .get(index)
                .ok_or(AppError::MonitorNotFound(index))?;
            let position = *monitor.position();
            let size = *monitor.size();

            window.set_decorations(false)?;
            window.set_position(PhysicalPosition::new(position.x, position.y))?;
            window.set_size(PhysicalSize::new(size.width, size.height))?;
            window.set_simple_fullscreen(true)?;
            Ok(Some(position))
        }
        OutputTarget::Windowed => {
            window.set_decorations(true)?;
            window.set_size(WINDOWED_SIZE)?;
            window.center()?;
            Ok(None)
        }
    }
}

/// Decides which windows float above the others:
///
/// - A full-screen output on a screen of its own floats above everything.
/// - A full-screen output on the operator's screen stays *below* the
///   operator window, which then floats instead, so it is never hidden.
/// - Windowed outputs behave like normal windows.
///
/// Called whenever an output opens, moves or closes, and when the operator
/// window moves to another screen.
fn update_stacking(app: &AppHandle) -> AppResult<()> {
    let Some(main) = app.get_webview_window(MAIN_WINDOW) else {
        return Ok(());
    };
    let operator_screen = main.current_monitor()?.map(|monitor| *monitor.position());
    let placements = app.state::<OutputPlacements>();

    let mut covers_operator_screen = false;
    for kind in [OutputKind::Projector, OutputKind::Stage] {
        let Some(window) = app.get_webview_window(kind.label()) else {
            continue;
        };
        let screen = placements.get(kind.label());
        let shares_screen = screen.is_some() && screen == operator_screen;

        covers_operator_screen |= shares_screen;
        window.set_always_on_top(screen.is_some() && !shares_screen)?;
    }

    main.set_always_on_top(covers_operator_screen)?;
    Ok(())
}

/// Window event hook (registered in `lib.rs`).
///
/// - Closing the operator window quits the app, instead of leaving output
///   windows open with no way to control them.
/// - Moving the operator window, or closing an output, can change which
///   window should float on top (see [`update_stacking`]).
pub fn on_window_event(window: &Window, event: &WindowEvent) {
    let app = window.app_handle();
    let is_main = window.label() == MAIN_WINDOW;

    match event {
        WindowEvent::Destroyed if is_main => app.exit(0),
        WindowEvent::Destroyed => {
            if let Some(kind) = OutputKind::from_label(window.label()) {
                app.state::<OutputPlacements>().set(kind.label(), None);
                restack(app);
            }
        }
        WindowEvent::Moved(_) if is_main => restack(app),
        _ => {}
    }
}

/// [`update_stacking`] for event handlers, which cannot return errors.
fn restack(app: &AppHandle) {
    if let Err(error) = update_stacking(app) {
        eprintln!("could not update window stacking: {error}");
    }
}

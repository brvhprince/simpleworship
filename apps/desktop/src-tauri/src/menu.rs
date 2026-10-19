//! The native menu bar.
//!
//! Menu items do not act by themselves. Each click is forwarded to the
//! operator window as a [`MENU_EVENT`] carrying the item's id, and the UI
//! decides what to do (see `apps/desktop/src/lib/schedule-file.svelte.ts`).
//! That keeps flows such as "you have unsaved changes: save first?" in one
//! place, where they are easy to follow.
//!
//! On macOS the menu belongs to the app and sits in the screen's menu bar.
//! On Windows and Linux it is attached to the operator window only; output
//! windows must never show a menu.

use tauri::menu::{
    Menu, MenuBuilder, MenuEvent, MenuItem, MenuItemBuilder, Submenu, SubmenuBuilder,
};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::recent::RecentSchedule;

/// The event channel menu clicks are forwarded on, to the operator window.
/// Keep in sync with `MENU_EVENT` in `apps/desktop/src/lib/menu.ts`.
pub const MENU_EVENT: &str = "menu://action";

/// Prefix of the ids of "Open Recent" entries; the rest of the id is the path.
pub const OPEN_RECENT_PREFIX: &str = "open-recent:";

/// Label of the operator window.
const MAIN_WINDOW: &str = "main";

/// The "Open Recent" submenu, kept so it can be refilled when the list changes.
pub struct RecentMenu(Submenu<Wry>);

/// Builds the menu bar and attaches it.
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let recent = SubmenuBuilder::new(app, "Open Recent").build()?;
    let menu = build(app, &recent)?;

    #[cfg(target_os = "macos")]
    app.set_menu(menu)?;
    #[cfg(not(target_os = "macos"))]
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        window.set_menu(menu)?;
    }

    app.manage(RecentMenu(recent));
    app.on_menu_event(forward_to_ui);
    Ok(())
}

/// Refills "Open Recent" with `recent`.
pub fn update_recent(app: &AppHandle, recent: &[RecentSchedule]) -> tauri::Result<()> {
    let submenu = &app.state::<RecentMenu>().0;
    for item in submenu.items()? {
        submenu.remove(&item)?;
    }

    if recent.is_empty() {
        let placeholder = MenuItemBuilder::new("No Recent Schedules")
            .enabled(false)
            .build(app)?;
        submenu.append(&placeholder)?;
    }
    for entry in recent {
        let id = format!("{OPEN_RECENT_PREFIX}{}", entry.path);
        submenu.append(&MenuItem::with_id(
            app,
            id,
            &entry.name,
            true,
            None::<&str>,
        )?)?;
    }
    Ok(())
}

fn build(app: &AppHandle, recent: &Submenu<Wry>) -> tauri::Result<Menu<Wry>> {
    let item = |id: &str, text: &str, accelerator: &str| {
        MenuItemBuilder::with_id(id, text)
            .accelerator(accelerator)
            .build(app)
    };

    // A custom Quit item instead of the built-in one, so the UI can ask
    // about unsaved changes first.
    let quit = item("quit", "Quit SimpleWorship", "CmdOrCtrl+Q")?;

    let file = SubmenuBuilder::new(app, "File")
        .item(&item("schedule-new", "New Schedule", "CmdOrCtrl+N")?)
        .item(&item("schedule-open", "Open Schedule…", "CmdOrCtrl+O")?)
        .item(recent)
        .separator()
        .item(&item("schedule-save", "Save Schedule", "CmdOrCtrl+S")?)
        .item(&item(
            "schedule-save-as",
            "Save Schedule As…",
            "CmdOrCtrl+Shift+S",
        )?);
    #[cfg(not(target_os = "macos"))]
    let file = file.separator().item(&quit);
    let file = file.build()?;

    // Without an Edit menu, macOS does not deliver ⌘C / ⌘V / ⌘Z to text fields.
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let window = SubmenuBuilder::new(app, "Window").minimize().build()?;

    let menu = MenuBuilder::new(app);
    #[cfg(target_os = "macos")]
    let menu = menu.item(
        &SubmenuBuilder::new(app, "SimpleWorship")
            .about(None)
            .separator()
            .services()
            .separator()
            .hide()
            .hide_others()
            .show_all()
            .separator()
            .item(&quit)
            .build()?,
    );

    menu.items(&[&file, &edit, &window]).build()
}

/// Sends a menu click to the operator window.
fn forward_to_ui(app: &AppHandle, event: MenuEvent) {
    if let Err(error) = app.emit_to(MAIN_WINDOW, MENU_EVENT, event.id().as_ref()) {
        eprintln!("could not forward menu click: {error}");
    }
}

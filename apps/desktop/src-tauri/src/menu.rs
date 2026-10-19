//! The native menu bar.
//!
//! Menu items do not act by themselves. Each click is forwarded to the
//! operator window as a [`MENU_EVENT`] carrying the item's id, and the UI
//! decides what to do (see `apps/desktop/src/lib/schedule-file.svelte.ts`).
//! The Help menu's links and "Open Logs Folder" are the exception: they
//! need nothing from the UI, so they are handled here.
//! That keeps flows such as "you have unsaved changes: save first?" in one
//! place, where they are easy to follow.
//!
//! On macOS the menu belongs to the app and sits in the screen's menu bar.
//! On Windows and Linux it is attached to the operator window only; output
//! windows must never show a menu.

use std::collections::HashMap;
use std::sync::Mutex;
use tauri::menu::{
    Menu, MenuBuilder, MenuEvent, MenuItem, MenuItemBuilder, Submenu, SubmenuBuilder,
};

use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_opener::OpenerExt;

use crate::recent::RecentSchedule;

/// The event channel menu clicks are forwarded on, to the operator window.
/// Keep in sync with `MENU_EVENT` in `apps/desktop/src/lib/menu.ts`.
pub const MENU_EVENT: &str = "menu://action";

/// Prefix of the ids of "Open Recent" entries; the rest of the id is the path.
pub const OPEN_RECENT_PREFIX: &str = "open-recent:";

/// Label of the operator window.
const MAIN_WINDOW: &str = "main";

/// Where the project lives: the guide, releases and the issue tracker.
const PROJECT_URL: &str = "https://github.com/brvhprince/simpleworship";

/// The "Open Recent" submenu, kept so it can be refilled when the list
/// changes (and replaced when the menu is rebuilt in another language).
pub struct RecentMenu(Mutex<Submenu<Wry>>);

/// The menu's words in the interface language, sent by the operator window
/// (see `apps/desktop/src/lib/i18n`). Missing keys use the English below.
#[derive(Default)]
pub struct MenuLabels(Mutex<HashMap<String, String>>);

/// Builds the menu bar and attaches it. Called once at startup.
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    app.manage(MenuLabels::default());
    let recent = attach(app)?;
    app.manage(RecentMenu(Mutex::new(recent)));
    app.on_menu_event(forward_to_ui);
    Ok(())
}

/// Builds the menu in the current language and attaches it, returning the
/// new "Open Recent" submenu.
fn attach(app: &AppHandle) -> tauri::Result<Submenu<Wry>> {
    let labels = app
        .state::<MenuLabels>()
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    let label =
        |key: &str, english: &str| labels.get(key).cloned().unwrap_or_else(|| english.into());

    let recent = SubmenuBuilder::new(app, label("menu.openRecent", "Open Recent")).build()?;
    let menu = build(app, &recent, &label)?;

    #[cfg(target_os = "macos")]
    app.set_menu(menu)?;
    #[cfg(not(target_os = "macos"))]
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        window.set_menu(menu)?;
    }
    Ok(recent)
}

/// Rebuilds the menu with the operator window's translations (sent when
/// the interface language is chosen or changes).
#[tauri::command]
pub fn menu_set_labels(app: AppHandle, labels: HashMap<String, String>) -> Result<(), String> {
    *app.state::<MenuLabels>()
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = labels;
    let recent = attach(&app).map_err(|error| error.to_string())?;
    *app.state::<RecentMenu>()
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = recent;
    let list = app.state::<crate::state::AppState>().recent().list();
    update_recent(&app, &list).map_err(|error| error.to_string())
}

/// Refills "Open Recent" with `recent`.
pub fn update_recent(app: &AppHandle, recent: &[RecentSchedule]) -> tauri::Result<()> {
    let submenu = app
        .state::<RecentMenu>()
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    for item in submenu.items()? {
        submenu.remove(&item)?;
    }

    if recent.is_empty() {
        let text = app
            .state::<MenuLabels>()
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get("menu.noRecent")
            .cloned()
            .unwrap_or_else(|| "No Recent Schedules".into());
        let placeholder = MenuItemBuilder::new(text).enabled(false).build(app)?;
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

/// Builds the menu bar. `label(key, english)` gives each item's words.
fn build(
    app: &AppHandle,
    recent: &Submenu<Wry>,
    label: &dyn Fn(&str, &str) -> String,
) -> tauri::Result<Menu<Wry>> {
    let item = |id: &str, text: String, accelerator: &str| {
        MenuItemBuilder::with_id(id, text)
            .accelerator(accelerator)
            .build(app)
    };
    let text = |id: &str, key: &str, english: &str| {
        MenuItemBuilder::with_id(id, label(key, english)).build(app)
    };

    // A custom Quit item instead of the built-in one, so the UI can ask
    // about unsaved changes first.
    let quit = item(
        "quit",
        label("menu.quit", "Quit SimpleWorship"),
        "CmdOrCtrl+Q",
    )?;

    let file = SubmenuBuilder::new(app, label("menu.file", "File"))
        .item(&item(
            "schedule-new",
            label("menu.new", "New Schedule"),
            "CmdOrCtrl+N",
        )?)
        .item(&item(
            "schedule-open",
            label("menu.open", "Open Schedule…"),
            "CmdOrCtrl+O",
        )?)
        .item(recent)
        .separator()
        .item(&item(
            "schedule-save",
            label("menu.save", "Save Schedule"),
            "CmdOrCtrl+S",
        )?)
        .item(&item(
            "schedule-save-as",
            label("menu.saveAs", "Save Schedule As…"),
            "CmdOrCtrl+Shift+S",
        )?)
        .separator()
        .item(&text(
            "import",
            "menu.import",
            "Import from Other Software…",
        )?)
        .item(&text("usage", "menu.usage", "Song Usage Report…")?);
    // Settings lives in the app menu on macOS, and in File elsewhere.
    let settings = item(
        "settings",
        label("menu.settings", "Settings…"),
        "CmdOrCtrl+,",
    )?;
    #[cfg(not(target_os = "macos"))]
    let file = file.separator().item(&settings).separator().item(&quit);
    let file = file.build()?;

    // Without an Edit menu, macOS does not deliver ⌘C / ⌘V / ⌘Z to text fields.
    // Undo and Redo go to the UI (not straight to the webview), so editors
    // with their own undo, like the slide editor, can take them over.
    let edit = SubmenuBuilder::new(app, label("menu.edit", "Edit"))
        .item(&item("undo", label("menu.undo", "Undo"), "CmdOrCtrl+Z")?)
        .item(&item(
            "redo",
            label("menu.redo", "Redo"),
            "CmdOrCtrl+Shift+Z",
        )?)
        .separator()
        .cut_with_text(label("menu.cut", "Cut"))
        .copy_with_text(label("menu.copy", "Copy"))
        .paste_with_text(label("menu.paste", "Paste"))
        .select_all_with_text(label("menu.selectAll", "Select All"))
        .build()?;

    let window = SubmenuBuilder::new(app, label("menu.window", "Window"))
        .minimize_with_text(label("menu.minimize", "Minimize"))
        .build()?;

    let help = SubmenuBuilder::new(app, label("menu.help", "Help"))
        .item(&text("help-guide", "menu.guide", "SimpleWorship Guide")?)
        .item(&text(
            "help-moving",
            "menu.moving",
            "Moving from EasyWorship",
        )?)
        .separator()
        .item(&text(
            "check-updates",
            "menu.checkUpdates",
            "Check for Updates…",
        )?)
        .separator()
        .item(&text("help-report", "menu.report", "Report a Problem…")?)
        .item(&text("help-logs", "menu.logs", "Open Logs Folder")?);
    // On macOS, About is in the app menu.
    #[cfg(not(target_os = "macos"))]
    let help = help
        .separator()
        .item(&text("about", "menu.about", "About SimpleWorship")?);
    let help = help.build()?;

    let menu = MenuBuilder::new(app);
    #[cfg(target_os = "macos")]
    let menu = menu.item(
        // Labelled here (and translated), and About opens SimpleWorship's
        // own About dialog rather than the system's.
        &SubmenuBuilder::new(app, "SimpleWorship")
            .item(&text("about", "menu.about", "About SimpleWorship")?)
            .separator()
            .item(&settings)
            .separator()
            .services_with_text(label("menu.services", "Services"))
            .separator()
            .hide_with_text(label("menu.hide", "Hide SimpleWorship"))
            .hide_others_with_text(label("menu.hideOthers", "Hide Others"))
            .show_all_with_text(label("menu.showAll", "Show All"))
            .separator()
            .item(&quit)
            .build()?,
    );

    menu.items(&[&file, &edit, &window, &help]).build()
}

/// The project's web pages that the app links to.
fn project_page(page: &str) -> Option<String> {
    Some(match page {
        "website" => PROJECT_URL.to_owned(),
        "guide" => format!("{PROJECT_URL}#readme"),
        "moving" => format!("{PROJECT_URL}/blob/main/docs/moving-from-easyworship.md"),
        "report" => format!("{PROJECT_URL}/issues/new"),
        "license" => format!("{PROJECT_URL}/blob/main/LICENSE"),
        "changes" => format!("{PROJECT_URL}/blob/main/CHANGELOG.md"),
        _ => return None,
    })
}

/// Opens one of the project's web pages in the browser: "website",
/// "guide", "moving", "report", "license" or "changes".
#[tauri::command]
pub fn open_project_page(app: AppHandle, page: String) -> Result<(), String> {
    let url = project_page(&page).ok_or_else(|| format!("no page called {page}"))?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| error.to_string())
}

/// Handles the Help menu's links here, and sends every other click to the
/// operator window.
fn forward_to_ui(app: &AppHandle, event: MenuEvent) {
    let page = match event.id().as_ref() {
        "help-guide" => Some("guide"),
        "help-moving" => Some("moving"),
        "help-report" => Some("report"),
        "help-logs" => {
            if let Err(error) = open_logs_folder(app.clone()) {
                log::warn!("could not open the logs folder: {error}");
            }
            return;
        }
        _ => None,
    };
    if let Some(page) = page {
        if let Err(error) = open_project_page(app.clone(), page.into()) {
            log::warn!("could not open the link: {error}");
        }
        return;
    }
    if let Err(error) = app.emit_to(MAIN_WINDOW, MENU_EVENT, event.id().as_ref()) {
        log::warn!("could not forward menu click: {error}");
    }
}

/// Opens the folder with SimpleWorship's log files, e.g. to send one with a
/// problem report.
#[tauri::command]
pub fn open_logs_folder(app: AppHandle) -> Result<(), String> {
    let folder = app
        .path()
        .app_log_dir()
        .map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    app.opener()
        .open_path(folder.display().to_string(), None::<&str>)
        .map_err(|error| error.to_string())
}

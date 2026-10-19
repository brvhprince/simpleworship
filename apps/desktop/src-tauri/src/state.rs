//! Application state shared by all commands.

use std::sync::{Mutex, MutexGuard};

use sw_core::Engine;
use sw_db::Database;
use tauri::{AppHandle, Manager};

use crate::autosave::Autosave;
use crate::recent::RecentSchedules;
use crate::schedule_file::ScheduleFile;

/// File name of the library database inside the app data directory.
const DATABASE_FILE: &str = "library.sqlite";

/// Everything the commands need, managed by Tauri (`app.manage`).
///
/// **Lock order:** when a command needs several locks, it takes them in
/// the order the fields are declared (engine, then schedule file, …).
/// Taking them in a consistent order means two commands can never wait
/// for each other forever.
pub struct AppState {
    engine: Mutex<Engine>,
    schedule_file: Mutex<ScheduleFile>,
    database: Mutex<Database>,
    recent: Mutex<RecentSchedules>,
    /// The autosave writer. It has its own thread, so it needs no lock.
    pub autosave: Autosave,
}

impl AppState {
    /// Opens the library in the platform's app data directory, e.g.
    /// `~/Library/Application Support/org.simpleworship.app` on macOS.
    pub fn load(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&data_dir)?;

        Ok(Self {
            engine: Mutex::new(Engine::new()),
            schedule_file: Mutex::new(ScheduleFile::default()),
            database: Mutex::new(Database::open(data_dir.join(DATABASE_FILE))?),
            recent: Mutex::new(RecentSchedules::load(&data_dir)),
            autosave: Autosave::start(data_dir),
        })
    }

    /// Locks the engine.
    pub fn engine(&self) -> MutexGuard<'_, Engine> {
        lock(&self.engine)
    }

    /// Locks the schedule file status.
    pub fn schedule_file(&self) -> MutexGuard<'_, ScheduleFile> {
        lock(&self.schedule_file)
    }

    /// Locks the database.
    pub fn database(&self) -> MutexGuard<'_, Database> {
        lock(&self.database)
    }

    /// Locks the recent schedules list.
    pub fn recent(&self) -> MutexGuard<'_, RecentSchedules> {
        lock(&self.recent)
    }
}

/// Locks a mutex, carrying on if a previous holder panicked.
///
/// A poisoned lock means a command panicked half-way. None of this state has
/// invariants a panic could break, so carrying on beats crashing the app in
/// the middle of a service.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

//! Application state shared by all commands.

use std::sync::{Mutex, MutexGuard};

use sw_core::Engine;
use sw_db::Database;
use tauri::{AppHandle, Manager};

/// File name of the library database inside the app data directory.
const DATABASE_FILE: &str = "library.sqlite";

/// Everything the commands need, managed by Tauri (`app.manage`).
pub struct AppState {
    engine: Mutex<Engine>,
    database: Mutex<Database>,
}

impl AppState {
    /// Opens the library in the platform's app data directory, e.g.
    /// `~/Library/Application Support/org.simpleworship.app` on macOS.
    pub fn load(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&data_dir)?;

        Ok(Self {
            engine: Mutex::new(Engine::new()),
            database: Mutex::new(Database::open(data_dir.join(DATABASE_FILE))?),
        })
    }

    /// Locks the engine.
    pub fn engine(&self) -> MutexGuard<'_, Engine> {
        // A poisoned lock means a command panicked half-way. The engine has no
        // invariants that a panic could break, so carry on rather than crash
        // the app in the middle of a service.
        self.engine
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Locks the database.
    pub fn database(&self) -> MutexGuard<'_, Database> {
        self.database
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

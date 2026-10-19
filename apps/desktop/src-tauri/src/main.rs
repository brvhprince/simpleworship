//! SimpleWorship desktop entry point. All the setup lives in the library
//! crate (`lib.rs`) so it can be shared with Tauri's mobile targets.

// Hide the extra console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    simpleworship_desktop_lib::run();
}

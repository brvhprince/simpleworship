//! IPC commands, grouped by area. Each public function is registered in
//! `lib.rs` and called from the UI with `invoke("<name>", { ...args })`.
//!
//! Argument names are camelCase on the JavaScript side (Tauri converts them).

pub mod engine;
pub mod outputs;
pub mod schedule;
pub mod songs;
pub mod text;

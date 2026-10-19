//! # sw-files
//!
//! Reading and writing SimpleWorship's own file formats. For now that is the
//! `.sws` schedule file (see [`schedule_file`]).
//!
//! The functions here turn bytes into values and back. Opening and saving
//! files on disk is up to the caller, which keeps the formats easy to test
//! and usable for things that are not files (an autosave, a network
//! transfer).

pub mod schedule_file;

pub use schedule_file::{ScheduleFileError, read_schedule, write_schedule};

//! # sw-files
//!
//! Reading and writing SimpleWorship's own file formats: the `.sws`
//! schedule file (see [`schedule_file`]) and the `.swpack` theme pack (see
//! [`pack_file`]).
//!
//! The functions here turn bytes into values and back. Opening and saving
//! files on disk is up to the caller, which keeps the formats easy to test
//! and usable for things that are not files (an autosave, a network
//! transfer).

pub mod pack_file;
pub mod schedule_file;

pub use pack_file::{
    PACK_FORMAT_VERSION, PackContents, PackFileError, PackMedia, PackReader, write_pack_to,
};

pub use schedule_file::{
    MediaSource, ScheduleContents, ScheduleFileError, ScheduleReader, read_schedule,
    write_schedule, write_schedule_to,
};

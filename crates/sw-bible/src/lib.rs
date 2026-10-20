//! # sw-bible
//!
//! Everything SimpleWorship knows about the Bible, without touching the disk:
//!
//! - [`books`]: the books, their names, abbreviations and file-format codes.
//! - [`reference`]: parsing references typed by an operator ("jn 3:16-18").
//! - [`spoken`]: finding references in what a preacher says.
//! - [`import`]: reading Bibles from OSIS, Zefania, OpenSong, USX and USFM.
//!
//! Types that cross into the UI derive [`ts_rs::TS`] and are generated into
//! `packages/protocol/src/generated/bible.ts`.

pub mod books;
pub mod import;
pub mod reference;
pub mod spoken;

pub use books::{Book, BookId};
pub use reference::{Passage, VersePoint, VerseRange};

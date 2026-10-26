//! # sw-core
//!
//! The heart of SimpleWorship: the domain model (songs, schedules, themes,
//! slides) and the [`Engine`](engine::Engine), the state machine that decides
//! what is in preview, what is live, and what every output screen should draw.
//!
//! This crate performs **no I/O**. Storage lives in `sw-db`, windows and IPC live
//! in the desktop app. Keeping the core pure makes it easy to test and lets other
//! front ends (a LAN remote, an NDI renderer, multi-operator sync) drive the same
//! engine with the same [`Command`](command::Command)s.
//!
//! Every type that crosses the IPC boundary derives [`ts_rs::TS`] so the matching
//! TypeScript definitions are generated into `packages/protocol` (see
//! `pnpm gen:bindings`).

pub mod alerts;
pub mod audio;
pub mod command;
pub mod countdown;
pub mod deck;
pub mod engine;
pub mod id;
pub mod lyrics;
pub mod markup;
pub mod media;
pub mod output;
pub mod presentation;
pub mod schedule;
pub mod scripture;
pub mod slides;
pub mod song;
pub mod stage;
pub mod stream;
pub mod takeover;
pub mod theme;
pub mod themes;
pub mod time;
pub mod usage;

pub use command::{Command, EngineEvent};
pub use deck::{AutoAdvance, DeckSlide, SlideDeck, SlideStyle};
pub use engine::{Engine, EngineError, EngineSnapshot, LOCAL_OPERATOR, PaneState};
pub use id::Id;
pub use media::{CameraRef, FileFits, Logo, MediaItem, MediaKind, MediaRef, MediaScaling};
pub use output::{
    DisplayFlags, OutputState, RenderFrame, Transition, TransitionKind, VideoPlayback,
};
pub use presentation::{Presentation, PresentationSource};
pub use schedule::{ItemContent, Schedule, ScheduleItem};
pub use scripture::{BibleInfo, BookOutline, ScriptureOptions, TranslationText, Verse};
pub use slides::{Slide, SlideOptions};
pub use song::{Song, SongSummary};
pub use stream::{StreamBackdrop, StreamLook, StreamOutput, StreamSettings};
pub use theme::{Background, MediaFit, Theme};
pub use themes::{ItemLook, Look, ThemeDefaults, ThemeLibrary, ThemeRecord};

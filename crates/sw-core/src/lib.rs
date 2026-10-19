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

pub mod command;
pub mod engine;
pub mod id;
pub mod lyrics;
pub mod output;
pub mod presentation;
pub mod schedule;
pub mod slides;
pub mod song;
pub mod theme;
pub mod time;

pub use command::{Command, EngineEvent};
pub use engine::{Engine, EngineError, EngineSnapshot, PaneState};
pub use id::Id;
pub use output::{DisplayFlags, OutputState, RenderFrame, Transition, TransitionKind};
pub use presentation::{Presentation, PresentationSource};
pub use schedule::{ItemContent, Schedule, ScheduleItem};
pub use slides::{Slide, SlideOptions};
pub use song::{Song, SongSummary};
pub use theme::Theme;

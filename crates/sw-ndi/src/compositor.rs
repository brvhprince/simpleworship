//! The picture to send right now, with fades between pictures.

use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::frame::Frame;

/// Holds the current picture of one NDI source and blends from the old
/// picture to the new one during a fade, exactly as the projector's
/// renderer cross-fades. Shared between the code that takes pictures and
/// the [`Sender`](crate::Sender)'s thread.
pub struct Compositor {
    state: Mutex<State>,
}

struct State {
    width: u32,
    height: u32,
    /// The picture being faded out, during a fade.
    from: Option<Frame>,
    /// The latest picture.
    to: Frame,
    fade_started: Instant,
    fade: Duration,
    /// Bumped whenever the picture changes, so the sender can skip
    /// converting a picture it has already converted.
    version: u64,
}

impl Compositor {
    /// A compositor for pictures of `width × height`, starting see-through.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            state: Mutex::new(State {
                width,
                height,
                from: None,
                to: Frame::transparent(width, height),
                fade_started: Instant::now(),
                fade: Duration::ZERO,
                version: 0,
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Shows `frame`, fading over `fade` (zero for a cut). A new picture
    /// during a fade fades on from what is on screen at that moment.
    pub fn show(&self, frame: Frame, fade: Duration) {
        let now = Instant::now();
        let mut state = self.lock();
        let frame = frame.resized(state.width, state.height);
        let on_screen = if fade.is_zero() {
            None
        } else {
            Some(state.picture_at(now))
        };
        state.from = on_screen;
        state.to = frame;
        state.fade_started = now;
        state.fade = fade;
        state.version += 1;
    }

    /// Replaces the picture being shown (or faded to) with a newer picture
    /// of the same thing, without starting or cutting short a fade: for
    /// what finishes drawing a moment later (a font, a picture, the next
    /// second of a countdown).
    pub fn refresh(&self, frame: Frame) {
        let mut state = self.lock();
        let frame = frame.resized(state.width, state.height);
        state.to = frame;
        state.version += 1;
    }

    /// The picture at `now`, premultiplied, written into `out` unless it is
    /// still the picture numbered `shown` (what `out` already holds).
    /// Returns the number of the picture now in `out`: it changes whenever
    /// the picture does, and on every frame of a fade.
    pub fn render(&self, now: Instant, shown: u64, out: &mut Vec<u8>) -> u64 {
        let mut state = self.lock();
        if let Some(amount) = state.fade_progress(now) {
            if let Some(from) = &state.from {
                Frame::mix_into(from, &state.to, amount, out);
            }
            state.version += 1;
            return state.version;
        }
        if state.from.take().is_some() {
            // The fade just ended: settle on the new picture.
            state.version += 1;
        }
        if state.version != shown || out.is_empty() {
            out.clear();
            out.extend_from_slice(&state.to.pixels);
        }
        state.version
    }
}

impl State {
    /// How far through the fade `now` is (0 to 1), or `None` when no fade
    /// is running.
    fn fade_progress(&self, now: Instant) -> Option<f32> {
        self.from.as_ref()?;
        let elapsed = now.saturating_duration_since(self.fade_started);
        (elapsed < self.fade).then(|| elapsed.as_secs_f32() / self.fade.as_secs_f32())
    }

    /// What is on screen at `now`.
    fn picture_at(&self, now: Instant) -> Frame {
        match (self.fade_progress(now), &self.from) {
            (Some(amount), Some(from)) => {
                let mut pixels = Vec::new();
                Frame::mix_into(from, &self.to, amount, &mut pixels);
                Frame {
                    width: self.width,
                    height: self.height,
                    pixels,
                }
            }
            _ => self.to.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(value: u8) -> Frame {
        Frame {
            width: 1,
            height: 1,
            pixels: vec![value; 4],
        }
    }

    #[test]
    fn a_cut_shows_the_new_picture_at_once() {
        let compositor = Compositor::new(1, 1);
        compositor.show(solid(200), Duration::ZERO);
        let mut out = Vec::new();
        compositor.render(Instant::now(), u64::MAX, &mut out);
        assert_eq!(out, vec![200; 4]);
    }

    #[test]
    fn a_fade_blends_then_settles() {
        let compositor = Compositor::new(1, 1);
        compositor.show(solid(200), Duration::ZERO);
        let started = Instant::now();
        compositor.show(solid(0), Duration::from_millis(400));

        let mut out = Vec::new();
        let shown = compositor.render(started + Duration::from_millis(200), u64::MAX, &mut out);
        assert!((90..=110).contains(&out[0]), "half way: {}", out[0]);

        compositor.render(started + Duration::from_millis(500), shown, &mut out);
        assert_eq!(out, vec![0; 4]);
    }

    #[test]
    fn a_refresh_during_a_fade_keeps_fading() {
        let compositor = Compositor::new(1, 1);
        compositor.show(solid(200), Duration::ZERO);
        let started = Instant::now();
        compositor.show(solid(100), Duration::from_millis(400));
        compositor.refresh(solid(0));

        let mut out = Vec::new();
        let shown = compositor.render(started + Duration::from_millis(200), u64::MAX, &mut out);
        assert!(
            (90..=110).contains(&out[0]),
            "still half way, now to 0: {}",
            out[0]
        );
        compositor.render(started + Duration::from_millis(500), shown, &mut out);
        assert_eq!(out, vec![0; 4]);
    }

    #[test]
    fn an_unchanged_picture_is_not_copied_again() {
        let compositor = Compositor::new(1, 1);
        let mut out = Vec::new();
        let first = compositor.render(Instant::now(), u64::MAX, &mut out);
        out[0] = 77;
        assert_eq!(compositor.render(Instant::now(), first, &mut out), first);
        assert_eq!(out[0], 77, "left alone");

        compositor.show(solid(9), Duration::ZERO);
        let second = compositor.render(Instant::now(), first, &mut out);
        assert_ne!(second, first);
        assert_eq!(out, vec![9; 4]);
    }
}

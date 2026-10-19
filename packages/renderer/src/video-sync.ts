/**
 * Keeping every screen's copy of a video in step.
 *
 * The engine does not stream the position. It says "at time `anchor` the
 * video was at `positionMs`, and it is (or is not) playing", and each
 * screen works out where it should be from its own clock. See
 * `VideoPlayback` in `crates/sw-core/src/output.rs`.
 *
 * Seeking is the enemy of smooth playback: it throws away what the player
 * has buffered, so a player that seeks to catch up stalls, falls further
 * behind, and seeks again. So small drift is corrected by playing very
 * slightly faster or slower until the screens agree, and only a large
 * drift (or the operator seeking) moves the playhead.
 */

import type { VideoPlayback } from "@simpleworship/protocol";

/** Drift (seconds) small enough to leave alone. */
export const IN_STEP_S = 0.1;

/** Drift (seconds) beyond which the player jumps instead of catching up. */
export const SEEK_THRESHOLD_S = 2;

/** How much faster or slower a drifting player runs: ±5% is not noticeable. */
export const CATCH_UP_RATE = 0.05;

/**
 * Where the video should be, in seconds, at wall-clock time `now`
 * (milliseconds). A looping video wraps around its duration; one that does
 * not loop stops at the end. Mirrors `VideoPlayback::position_at` in Rust,
 * plus the wrapping, which needs the real duration only the webview knows.
 */
export function targetSeconds(
  playback: VideoPlayback,
  now: number,
  durationSeconds: number,
): number {
  const elapsed = playback.playing ? Math.max(0, now - playback.anchor) : 0;
  const seconds = (playback.positionMs + elapsed) / 1000;

  if (!Number.isFinite(durationSeconds) || durationSeconds <= 0) {
    return seconds;
  }
  return playback.loop ? seconds % durationSeconds : Math.min(seconds, durationSeconds);
}

/**
 * How far ahead (positive) or behind (negative) `current` is of `target`,
 * in seconds. For a looping video the shorter way round counts: 9.9 s and
 * 0.1 s of a 10-second loop are 0.2 s apart, not 9.8 s.
 */
export function driftSeconds(
  current: number,
  target: number,
  durationSeconds: number,
  loop: boolean,
): number {
  const drift = current - target;
  if (!loop || !Number.isFinite(durationSeconds) || durationSeconds <= 0) return drift;
  const half = durationSeconds / 2;
  return ((((drift + half) % durationSeconds) + durationSeconds) % durationSeconds) - half;
}

/** What to do about a drift: jump to the target, or play at this speed. */
export type Correction = { seek: true } | { seek: false; rate: number };

/** The gentlest correction for `drift` seconds (see the module comment). */
export function correctionFor(drift: number): Correction {
  const size = Math.abs(drift);
  if (size > SEEK_THRESHOLD_S) return { seek: true };
  if (size <= IN_STEP_S) return { seek: false, rate: 1 };
  // Ahead: slow down. Behind: speed up.
  return { seek: false, rate: drift > 0 ? 1 - CATCH_UP_RATE : 1 + CATCH_UP_RATE };
}

/**
 * Whether a player is still loading what it needs to play on. Drift
 * measured then is not real drift: it disappears once the data arrives.
 */
export function isBuffering(element: HTMLMediaElement): boolean {
  return element.seeking || element.readyState < HTMLMediaElement.HAVE_FUTURE_DATA;
}

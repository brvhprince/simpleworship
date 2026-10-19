/**
 * Keeping every screen's copy of a video in step.
 *
 * The engine does not stream the position. It says "at time `anchor` the
 * video was at `positionMs`, and it is (or is not) playing", and each
 * screen works out where it should be from its own clock. See
 * `VideoPlayback` in `crates/sw-core/src/output.rs`.
 */

import type { VideoPlayback } from "@simpleworship/protocol";

/** How far (in seconds) a video may drift before it is corrected. Small
 *  corrections are invisible; seeking constantly would stutter. */
export const DRIFT_TOLERANCE_S = 0.3;

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

/** Whether a video at `current` seconds is too far from `target` to leave alone. */
export function needsCorrection(current: number, target: number): boolean {
  return Math.abs(current - target) > DRIFT_TOLERANCE_S;
}

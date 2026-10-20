/**
 * Small time helpers shared by countdowns, alerts and timers.
 *
 * Times from the engine (video positions, countdown ends, alerts) are on
 * the clock of the computer running the service. A computer that joined
 * it sets the difference with {@link setClockOffset}; everything that
 * reads the time uses {@link now}, so its screens stay in step.
 */

import type { StageTimer } from "@simpleworship/protocol";

let clockOffsetMs = 0;

/** Sets how far the service's clock is ahead of this computer's. */
export function setClockOffset(ms: number): void {
  clockOffsetMs = ms;
}

/** The time on the service's clock, in milliseconds since 1970. */
export function now(): number {
  return Date.now() + clockOffsetMs;
}

/** `"4:59"`, `"1:02:05"`; negative values get a minus sign ("-0:12"). */
export function formatClock(ms: number): string {
  const negative = ms < 0;
  const total = Math.floor(Math.abs(ms) / 1000);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = String(total % 60).padStart(2, "0");
  const body =
    hours > 0 ? `${hours}:${String(minutes).padStart(2, "0")}:${seconds}` : `${minutes}:${seconds}`;
  return negative ? `-${body}` : body;
}

/**
 * Milliseconds left until `endsAt`, rounded up to the next second so a
 * countdown shows 0:00 exactly when it ends (not a second early).
 */
export function remainingMs(endsAt: number, now: number): number {
  return Math.max(0, Math.ceil((endsAt - now) / 1000) * 1000);
}

/**
 * What a stage timer shows at `now`: time counted up, or time left
 * (negative once a count-down runs over).
 */
export function stageTimerMs(timer: StageTimer, now: number): number {
  const elapsed = timer.elapsedMs + (timer.running ? Math.max(0, now - timer.anchor) : 0);
  return timer.mode === "countUp" ? elapsed : timer.durationMs - elapsed;
}

/** Ticker speed, in canvas pixels per second. Fast enough to read, slow enough to follow. */
export const TICKER_SPEED = 160;

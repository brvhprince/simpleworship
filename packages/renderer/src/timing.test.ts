import { describe, expect, it } from "vitest";
import { formatClock, remainingMs, stageTimerMs } from "./timing.ts";

describe("formatClock", () => {
  it("formats minutes, hours and overtime", () => {
    expect(formatClock(299_000)).toBe("4:59");
    expect(formatClock(3_725_000)).toBe("1:02:05");
    expect(formatClock(-12_000)).toBe("-0:12");
  });
});

describe("remainingMs", () => {
  it("rounds up to whole seconds and stops at zero", () => {
    expect(remainingMs(10_000, 9_001)).toBe(1_000);
    expect(remainingMs(10_000, 10_000)).toBe(0);
    expect(remainingMs(10_000, 12_000)).toBe(0);
  });
});

describe("stageTimerMs", () => {
  const base = { durationMs: 60_000, elapsedMs: 10_000, anchor: 1_000, running: true };

  it("counts on from the anchor while running", () => {
    expect(stageTimerMs({ ...base, mode: "countUp" }, 6_000)).toBe(15_000);
    expect(stageTimerMs({ ...base, mode: "countDown" }, 6_000)).toBe(45_000);
  });

  it("stands still while paused and goes negative in overtime", () => {
    expect(stageTimerMs({ ...base, mode: "countUp", running: false }, 99_000)).toBe(10_000);
    expect(stageTimerMs({ ...base, mode: "countDown" }, 61_000)).toBe(-10_000);
  });
});

import type { VideoPlayback } from "@simpleworship/protocol";
import { describe, expect, it } from "vitest";
import { needsCorrection, targetSeconds } from "./video-sync.ts";

const playback = (overrides: Partial<VideoPlayback> = {}): VideoPlayback => ({
  mediaId: "v",
  playing: true,
  positionMs: 0,
  anchor: 1_000,
  loop: false,
  muted: false,
  ...overrides,
});

describe("targetSeconds", () => {
  it("counts on from the anchor while playing", () => {
    expect(targetSeconds(playback({ positionMs: 2_000 }), 4_000, 60)).toBe(5);
  });

  it("stays put while paused", () => {
    expect(targetSeconds(playback({ playing: false, positionMs: 2_000 }), 99_000, 60)).toBe(2);
  });

  it("wraps looping videos and stops the others at the end", () => {
    expect(targetSeconds(playback({ loop: true }), 1_000 + 25_000, 10)).toBe(5);
    expect(targetSeconds(playback({ loop: false }), 1_000 + 25_000, 10)).toBe(10);
  });

  it("copes with an unknown duration", () => {
    expect(targetSeconds(playback(), 4_000, Number.NaN)).toBe(3);
  });
});

describe("needsCorrection", () => {
  it("ignores tiny drift", () => {
    expect(needsCorrection(10.1, 10)).toBe(false);
    expect(needsCorrection(11, 10)).toBe(true);
  });
});

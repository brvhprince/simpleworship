import type { VideoPlayback } from "@simpleworship/protocol";
import { describe, expect, it } from "vitest";
import { correctionFor, driftSeconds, targetSeconds } from "./video-sync.ts";

const playing: VideoPlayback = {
  mediaId: "v",
  playing: true,
  positionMs: 2_000,
  anchor: 10_000,
  loop: true,
  muted: false,
};

describe("targetSeconds", () => {
  it("counts on from the anchor while playing", () => {
    expect(targetSeconds(playing, 13_000, 60)).toBe(5);
  });

  it("stands still while paused", () => {
    expect(targetSeconds({ ...playing, playing: false }, 99_000, 60)).toBe(2);
  });

  it("wraps a looping video and stops one that does not loop", () => {
    expect(targetSeconds(playing, 20_000, 10)).toBe(2);
    expect(targetSeconds({ ...playing, loop: false }, 20_000, 10)).toBe(10);
  });

  it("does not wrap before the duration is known", () => {
    expect(targetSeconds(playing, 20_000, Number.NaN)).toBe(12);
  });
});

describe("driftSeconds", () => {
  it("is positive when ahead and negative when behind", () => {
    expect(driftSeconds(10.5, 10, 60, false)).toBeCloseTo(0.5);
    expect(driftSeconds(9.5, 10, 60, false)).toBeCloseTo(-0.5);
  });

  it("goes the short way round a loop", () => {
    expect(driftSeconds(9.9, 0.1, 10, true)).toBeCloseTo(-0.2);
    expect(driftSeconds(0.1, 9.9, 10, true)).toBeCloseTo(0.2);
  });
});

describe("correctionFor", () => {
  it("leaves tiny drift alone", () => {
    expect(correctionFor(0.05)).toEqual({ seek: false, rate: 1 });
  });

  it("catches up gently instead of seeking", () => {
    expect(correctionFor(0.8)).toEqual({ seek: false, rate: 0.95 });
    expect(correctionFor(-0.8)).toEqual({ seek: false, rate: 1.05 });
  });

  it("only jumps for a large drift", () => {
    expect(correctionFor(5)).toEqual({ seek: true });
  });
});

import { describe, expect, it } from "vitest";
import { fitRange, largestFittingSize, MIN_FONT_SIZE } from "./autofit.ts";

describe("largestFittingSize", () => {
  it("keeps the maximum size when the text already fits", () => {
    expect(largestFittingSize(() => true, 84)).toBe(84);
  });

  it("finds the exact largest size that fits", () => {
    expect(largestFittingSize((size) => size <= 57, 84)).toBe(57);
  });

  it("never goes below the minimum", () => {
    expect(largestFittingSize(() => false, 84)).toBe(MIN_FONT_SIZE);
  });

  it("only measures a handful of sizes", () => {
    let measurements = 0;
    largestFittingSize((size) => {
      measurements += 1;
      return size <= 40;
    }, 200);
    expect(measurements).toBeLessThanOrEqual(9);
  });
});

describe("fitRange", () => {
  it("keeps the given size when neither shrinking nor growing", () => {
    expect(fitRange(40, false, false, 800)).toBeNull();
  });

  it("only shrinks when growing is off", () => {
    expect(fitRange(40, true, false, 800)).toEqual({ min: MIN_FONT_SIZE, max: 40 });
  });

  it("only grows when shrinking is off, up to the box's height", () => {
    expect(fitRange(40, false, true, 300.6)).toEqual({ min: 40, max: 300 });
  });

  it("does both when both are on", () => {
    expect(fitRange(40, true, true, 300)).toEqual({ min: MIN_FONT_SIZE, max: 300 });
  });

  it("never asks for less than the given size from a short box", () => {
    expect(fitRange(72, false, true, 50)).toEqual({ min: 72, max: 72 });
  });
});

describe("growing without shrinking", () => {
  it("leaves text that is too long at the given size", () => {
    const range = fitRange(40, false, true, 300);
    expect(range && largestFittingSize(() => false, range.max, range.min)).toBe(40);
  });
});

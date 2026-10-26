import { describe, expect, it } from "vitest";
import { fitLimit, largestFittingSize, MIN_FONT_SIZE } from "./autofit.ts";

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

describe("fitLimit", () => {
  it("never grows text that was not asked to scale up", () => {
    expect(fitLimit(40, false, 800)).toBe(40);
  });

  it("lets short text grow up to the box's height when scaling up", () => {
    expect(fitLimit(40, true, 300.6)).toBe(300);
  });

  it("keeps the given size when the box is shorter than it", () => {
    expect(fitLimit(72, true, 50)).toBe(72);
  });
});

import { describe, expect, it } from "vitest";
import { formatBytes, formatDuration } from "./format.ts";

describe("formatDuration", () => {
  it("writes minutes and seconds, and hours when needed", () => {
    expect(formatDuration(125_000)).toBe("2:05");
    expect(formatDuration(3_725_000)).toBe("1:02:05");
    expect(formatDuration(-5)).toBe("0:00");
  });
});

describe("formatBytes", () => {
  it("picks a readable unit", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1_234_567)).toBe("1.2 MB");
    expect(formatBytes(3_000_000_000)).toBe("3.0 GB");
  });
});

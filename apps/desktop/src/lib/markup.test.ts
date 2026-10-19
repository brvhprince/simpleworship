import { describe, expect, it } from "vitest";
import { toggleFormat } from "./markup.ts";

describe("toggleFormat", () => {
  it("wraps the selection in markers, keeping it selected", () => {
    expect(toggleFormat({ text: "Join us Sunday", start: 8, end: 14 }, "bold")).toEqual({
      text: "Join us **Sunday**",
      start: 10,
      end: 16,
    });
  });

  it("unwraps a selection that already has the style", () => {
    expect(toggleFormat({ text: "Join us **Sunday**", start: 10, end: 16 }, "bold")).toEqual({
      text: "Join us Sunday",
      start: 8,
      end: 14,
    });
  });

  it("inserts an empty pair to type into when nothing is selected", () => {
    expect(toggleFormat({ text: "Hi ", start: 3, end: 3 }, "underline")).toEqual({
      text: "Hi ____",
      start: 5,
      end: 5,
    });
  });

  it("does not take half of a bold marker for italic", () => {
    const result = toggleFormat({ text: "**Sunday**", start: 2, end: 8 }, "italic");
    expect(result.text).toBe("***Sunday***");
  });
});

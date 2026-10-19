import { describe, expect, it } from "vitest";
import { dropTarget, finalIndex, insertionIndex, mediaIds } from "./drag.ts";

const row = { top: 100, height: 40 } as DOMRectReadOnly;

describe("insertionIndex", () => {
  it("inserts before a row in its top half and after it in its bottom half", () => {
    expect(insertionIndex(3, 110, row)).toBe(3);
    expect(insertionIndex(3, 130, row)).toBe(4);
  });
});

describe("finalIndex", () => {
  it("keeps upward moves as they are", () => {
    // Moving item 3 to the gap before item 1.
    expect(finalIndex(3, 1)).toBe(1);
  });

  it("accounts for the removed item on downward moves", () => {
    // Moving item 0 to the gap before item 3 leaves it at position 2.
    expect(finalIndex(0, 3)).toBe(2);
  });

  it("dropping an item next to itself does not move it", () => {
    expect(finalIndex(2, 2)).toBe(2);
    expect(finalIndex(2, 3)).toBe(2);
  });
});

describe("dropTarget", () => {
  const row = { top: 100, height: 40 } as DOMRectReadOnly;

  it("adds to the row when dropped on its middle, if it can take slides", () => {
    expect(dropTarget(3, 120, row, true)).toEqual({ kind: "into", index: 3 });
    expect(dropTarget(3, 104, row, true)).toEqual({ kind: "between", index: 3 });
    expect(dropTarget(3, 138, row, true)).toEqual({ kind: "between", index: 4 });
  });

  it("only inserts between rows that cannot take slides", () => {
    expect(dropTarget(3, 120, row, false)).toEqual({ kind: "between", index: 3 });
  });
});

describe("mediaIds", () => {
  it("reads one id per line", () => {
    expect(mediaIds("a\nb\n")).toEqual(["a", "b"]);
    expect(mediaIds("a")).toEqual(["a"]);
  });
});

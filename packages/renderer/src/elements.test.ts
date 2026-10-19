import type { TextStyle } from "@simpleworship/protocol";
import { describe, expect, it } from "vitest";
import { frameStyle, lineStyle, objectFit, runStyle, shapeStyle } from "./elements.ts";

const base = { fontSize: 80 } as TextStyle;

describe("runStyle", () => {
  it("sizes runs relative to the box, so they shrink with it", () => {
    expect(
      runStyle({ text: "x", bold: false, italic: false, underline: false, fontSize: 120 }, base),
    ).toBe("font-size: 1.5em");
  });

  it("adds only what the run changes", () => {
    const style = runStyle(
      { text: "x", bold: true, italic: false, underline: true, color: "gold" },
      base,
    );
    expect(style).toBe("color: gold; font-weight: bolder; text-decoration: underline");
  });
});

describe("frameStyle", () => {
  it("places the element and fades it only when asked", () => {
    expect(frameStyle({ x: 10, y: 20, width: 300, height: 100 }, 1)).toBe(
      "left: 10px; top: 20px; width: 300px; height: 100px",
    );
    expect(frameStyle({ x: 0, y: 0, width: 1, height: 1 }, 0.4)).toContain("opacity: 0.4");
  });
});

describe("shapes", () => {
  const shape = {
    type: "shape" as const,
    shape: "rectangle" as const,
    fill: "#fff",
    stroke: null,
    cornerRadius: 12,
  };

  it("rounds rectangles and makes ellipses round", () => {
    expect(shapeStyle(shape)).toContain("border-radius: 12px");
    expect(shapeStyle({ ...shape, shape: "ellipse" })).toContain("border-radius: 50%");
  });

  it("draws a line along the longer side", () => {
    const line = { ...shape, shape: "line" as const, stroke: { color: "red", width: 6 } };
    const style = lineStyle(line, { x: 0, y: 0, width: 400, height: 40 });
    expect(style).toContain("height: 6px");
    expect(style).toContain("top: 17px");
  });
});

describe("objectFit", () => {
  it("maps stretch to CSS fill", () => {
    expect(objectFit("stretch")).toBe("fill");
    expect(objectFit("cover")).toBe("cover");
  });
});

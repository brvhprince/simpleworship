import type { TextStyle } from "@simpleworship/protocol";
import { describe, expect, it } from "vitest";
import { backgroundCss, textBoxStyle, textStyle } from "./styles.ts";

const baseText: TextStyle = {
  fontFamily: "Arial",
  fontSize: 80,
  fontWeight: 700,
  italic: false,
  uppercase: false,
  color: "#fff",
  lineHeight: 1.2,
  align: "center",
  outline: null,
  shadow: null,
};

describe("textStyle", () => {
  it("leaves the font size to autofit", () => {
    expect(textStyle(baseText)).not.toContain("font-size");
  });

  it("draws the outline outside the letters", () => {
    const style = textStyle({ ...baseText, outline: { color: "black", width: 3 } });
    expect(style).toContain("-webkit-text-stroke: 6px black");
    expect(style).toContain("paint-order: stroke fill");
  });

  it("omits decorations that are switched off", () => {
    const style = textStyle(baseText);
    expect(style).not.toContain("text-shadow");
    expect(style).not.toContain("stroke");
  });
});

describe("textBoxStyle", () => {
  it("maps vertical alignment to flex justification", () => {
    const box = { top: 1, right: 2, bottom: 3, left: 4, autoFit: true };
    expect(textBoxStyle({ ...box, verticalAlign: "bottom" })).toContain(
      "justify-content: flex-end",
    );
    expect(textBoxStyle({ ...box, verticalAlign: "middle" })).toContain("top: 1px");
  });
});

describe("backgroundCss", () => {
  it("builds gradients and colours, but not media", () => {
    expect(backgroundCss({ type: "color", color: "red" })).toBe("red");
    expect(backgroundCss({ type: "linearGradient", from: "red", to: "blue", angleDeg: 90 })).toBe(
      "linear-gradient(90deg, red, blue)",
    );
    expect(backgroundCss({ type: "image", path: "/a.png" })).toBeNull();
  });
});

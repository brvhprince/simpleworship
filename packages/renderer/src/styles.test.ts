import type { TextStyle } from "@simpleworship/protocol";
import { describe, expect, it } from "vitest";
import { backgroundCss, labelStyle, textBoxStyle, textStyle, verseNumberStyle } from "./styles.ts";

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
    expect(
      backgroundCss({ type: "image", media: { id: "m", file: "a.png" }, fit: "cover" }),
    ).toBeNull();
  });
});

describe("labelStyle", () => {
  const label = {
    visible: true,
    fontSize: 30,
    color: "",
    fontFamily: "",
    bold: true,
    align: null,
    placement: "box" as const,
    textBox: {
      top: 0,
      right: 0,
      bottom: 0,
      left: 0,
      verticalAlign: "bottom" as const,
      autoFit: true,
    },
  };

  it("follows the slide text when it has no font, colour or alignment of its own", () => {
    const style = labelStyle(label, baseText, true);
    expect(style).toContain("font-family: Arial");
    expect(style).toContain("color: #fff");
    expect(style).toContain("text-align: center");
    expect(style).toContain("font-size: 30px");
  });

  it("leaves the size to autofit in its own box", () => {
    const style = labelStyle({ ...label, color: "gold", align: "right" }, baseText, false);
    expect(style).not.toContain("font-size");
    expect(style).toContain("color: gold");
    expect(style).toContain("text-align: right");
  });
});

describe("verseNumberStyle", () => {
  it("fades numbers without a colour and raises them when asked", () => {
    const numbers = { visible: true, color: "", sizePercent: 55, superscript: true, bold: false };
    expect(verseNumberStyle(numbers)).toContain("opacity: 0.8");
    expect(verseNumberStyle(numbers)).toContain("vertical-align: super");
    const inline = verseNumberStyle({ ...numbers, color: "gold", superscript: false });
    expect(inline).toContain("color: gold");
    expect(inline).not.toContain("opacity");
    expect(inline).toContain("vertical-align: baseline");
  });
});

import type { FooterStyle } from "@simpleworship/protocol";
import { describe, expect, it } from "vitest";
import { applyPreset, currentPreset, REFERENCE_SPACING } from "./label-placement.ts";

const label: FooterStyle = {
  visible: true,
  fontSize: 39,
  color: "",
  fontFamily: "",
  bold: true,
  align: null,
  placement: "box",
  textBox: {
    top: 880,
    right: 120,
    bottom: 70,
    left: 120,
    verticalAlign: "bottom",
    autoFit: true,
    scaleUp: false,
  },
};

describe("label presets", () => {
  it("puts the label along the top or bottom edge with the chosen alignment", () => {
    const top = applyPreset(label, "top-right", REFERENCE_SPACING);
    expect(top.textBox.top).toBe(70);
    expect(top.textBox.verticalAlign).toBe("top");
    expect(top.align).toBe("right");

    const bottom = applyPreset(label, "bottom-center", REFERENCE_SPACING);
    expect(bottom.textBox.bottom).toBe(70);
    expect(bottom.textBox.top).toBe(1080 - 70 - 130);
  });

  it("recognises a preset, and a box dragged somewhere else", () => {
    const placed = applyPreset(label, "top-left", REFERENCE_SPACING);
    expect(currentPreset(placed, REFERENCE_SPACING)).toBe("top-left");
    expect(
      currentPreset({ ...placed, textBox: { ...placed.textBox, top: 300 } }, REFERENCE_SPACING),
    ).toBeNull();
    expect(
      currentPreset(applyPreset(label, "after-text", REFERENCE_SPACING), REFERENCE_SPACING),
    ).toBe("after-text");
  });
});

/**
 * Quick positions for the scripture reference and the footer: one click
 * puts the box in a corner or along an edge, or straight after the text.
 * The box can then be dragged anywhere in the theme designer.
 */

import type { FooterStyle, HorizontalAlign } from "@simpleworship/protocol";
import { CANVAS_HEIGHT } from "@simpleworship/renderer";
import type { MessageKey } from "./i18n/i18n.svelte.ts";

/** A quick position: an edge and an alignment, or after the text. */
export type LabelPreset = `${"top" | "bottom"}-${HorizontalAlign}` | "after-text";

/** The quick positions, in the order the designer shows them. */
export const LABEL_PRESETS: { value: LabelPreset; label: MessageKey }[] = [
  { value: "top-left", label: "place.topLeft" },
  { value: "top-center", label: "place.topCenter" },
  { value: "top-right", label: "place.topRight" },
  { value: "bottom-left", label: "place.bottomLeft" },
  { value: "bottom-center", label: "place.bottomCenter" },
  { value: "bottom-right", label: "place.bottomRight" },
  { value: "after-text", label: "place.afterText" },
];

/** Side margins of a label box, in canvas pixels (like the text box). */
const SIDE_MARGIN = 120;

/** How a kind of label sits along an edge. */
export interface EdgeSpacing {
  /** Distance from the top or bottom edge, in canvas pixels. */
  margin: number;
  /** Height of the box, in canvas pixels. */
  height: number;
}

/**
 * Edge spacing for the reference and the footer. The reference sits a little
 * further in, so both fit along the bottom without overlapping.
 */
export const REFERENCE_SPACING: EdgeSpacing = { margin: 70, height: 130 };
export const FOOTER_SPACING: EdgeSpacing = { margin: 26, height: 64 };

/** The label moved to a quick position. */
export function applyPreset(
  label: FooterStyle,
  preset: LabelPreset,
  spacing: EdgeSpacing,
): FooterStyle {
  if (preset === "after-text") return { ...label, placement: "afterText" };

  const [edge, align] = preset.split("-") as ["top" | "bottom", HorizontalAlign];
  const near = spacing.margin;
  const far = CANVAS_HEIGHT - spacing.margin - spacing.height;
  return {
    ...label,
    placement: "box",
    align,
    textBox: {
      ...label.textBox,
      left: SIDE_MARGIN,
      right: SIDE_MARGIN,
      top: edge === "top" ? near : far,
      bottom: edge === "top" ? far : near,
      verticalAlign: edge,
    },
  };
}

/**
 * The quick position the label is in, or `null` if it has been dragged
 * somewhere else.
 */
export function currentPreset(label: FooterStyle, spacing: EdgeSpacing): LabelPreset | null {
  if (label.placement === "afterText") return "after-text";
  for (const { value } of LABEL_PRESETS) {
    if (value === "after-text") continue;
    const placed = applyPreset(label, value, spacing);
    if (
      JSON.stringify(placed.textBox) === JSON.stringify(label.textBox) &&
      placed.align === label.align
    ) {
      return value;
    }
  }
  return null;
}

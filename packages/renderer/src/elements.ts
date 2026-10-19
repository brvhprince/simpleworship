/**
 * Turns slide elements (text boxes, shapes, pictures) into CSS. Kept free
 * of Svelte and the DOM so the rules are easy to unit-test (see
 * `elements.test.ts`). Used by `ElementsLayer` and by the desktop app's
 * slide editor, so editing looks exactly like the projector.
 */

import type { ElementContent, Frame, TextRun, TextStyle } from "@simpleworship/protocol";

/** Joins `property: value` pairs into an inline style string, skipping empty values. */
function css(declarations: Record<string, string | number | null | undefined>): string {
  return Object.entries(declarations)
    .filter(([, value]) => value !== null && value !== undefined && value !== "")
    .map(([property, value]) => `${property}: ${value}`)
    .join("; ");
}

/** Position, size and opacity of an element on the canvas. */
export function frameStyle(frame: Frame, opacity: number): string {
  return css({
    left: `${frame.x}px`,
    top: `${frame.y}px`,
    width: `${frame.width}px`,
    height: `${frame.height}px`,
    opacity: opacity < 1 ? opacity : null,
  });
}

type TextContent = Extract<ElementContent, { type: "text" }>;
type ShapeContent = Extract<ElementContent, { type: "shape" }>;

/** The text box itself: its fill, rounded corners and padding. */
export function textBoxStyle(content: TextContent): string {
  return css({
    background: content.fill,
    "border-radius": content.cornerRadius ? `${content.cornerRadius}px` : null,
    padding: content.padding ? `${content.padding}px` : null,
  });
}

/** Where the text sits in its box, top to bottom (as flex alignment). */
export function verticalJustify(content: TextContent): string {
  return { top: "flex-start", middle: "center", bottom: "flex-end" }[content.verticalAlign];
}

/**
 * The style of one run of text. Sizes are relative to the box's size (in
 * `em`), so when the box shrinks its text to fit, every run shrinks with it.
 */
export function runStyle(run: TextRun, base: TextStyle): string {
  return css({
    color: run.color,
    "font-size": run.fontSize ? `${round(run.fontSize / base.fontSize)}em` : null,
    "font-family": run.fontFamily,
    "font-weight": run.bold ? "bolder" : null,
    "font-style": run.italic ? "italic" : null,
    "text-decoration": run.underline ? "underline" : null,
  });
}

/** A rectangle or ellipse: fill, outline and corners. */
export function shapeStyle(content: ShapeContent): string {
  return css({
    background: content.fill,
    border: content.stroke ? `${content.stroke.width}px solid ${content.stroke.color}` : null,
    "border-radius":
      content.shape === "ellipse"
        ? "50%"
        : content.cornerRadius
          ? `${content.cornerRadius}px`
          : null,
    "box-sizing": "border-box",
  });
}

/**
 * A line runs through the middle of its box, along the longer side, as
 * thick as its outline (or 4 pixels without one).
 */
export function lineStyle(content: ShapeContent, frame: Frame): string {
  const thickness = content.stroke?.width ?? 4;
  const color = content.stroke?.color ?? content.fill ?? "#ffffff";
  const horizontal = frame.width >= frame.height;
  return css({
    position: "absolute",
    background: color,
    left: horizontal ? 0 : `${(frame.width - thickness) / 2}px`,
    top: horizontal ? `${(frame.height - thickness) / 2}px` : 0,
    width: horizontal ? "100%" : `${thickness}px`,
    height: horizontal ? `${thickness}px` : "100%",
  });
}

/** CSS `object-fit` for how a picture fills its box. */
export function objectFit(fit: "cover" | "contain" | "stretch"): string {
  return fit === "stretch" ? "fill" : fit;
}

function round(value: number): number {
  return Math.round(value * 1000) / 1000;
}

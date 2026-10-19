/**
 * Turns theme settings into CSS. Kept free of Svelte and the DOM so the
 * rules are easy to unit-test (see `styles.test.ts`).
 */

import type {
  Background,
  FooterStyle,
  TextBox,
  TextStyle,
  VerseNumberStyle,
} from "@simpleworship/protocol";

/** Joins `property: value` pairs into an inline style string, skipping empty values. */
function css(declarations: Record<string, string | number | null | undefined>): string {
  return Object.entries(declarations)
    .filter(([, value]) => value !== null && value !== undefined && value !== "")
    .map(([property, value]) => `${property}: ${value}`)
    .join("; ");
}

/** Position and alignment of the text box on the canvas. */
export function textBoxStyle(box: TextBox): string {
  const justify = { top: "flex-start", middle: "center", bottom: "flex-end" }[box.verticalAlign];
  return css({
    top: `${box.top}px`,
    right: `${box.right}px`,
    bottom: `${box.bottom}px`,
    left: `${box.left}px`,
    "justify-content": justify,
  });
}

/**
 * Font and decoration of the slide text.
 *
 * The font *size* is deliberately left out: it is owned by the
 * {@link autofit} action, which may shrink it to make the text fit.
 */
export function textStyle(style: TextStyle): string {
  return css({
    "font-family": style.fontFamily,
    "font-weight": style.fontWeight,
    "font-style": style.italic ? "italic" : "normal",
    "text-transform": style.uppercase ? "uppercase" : "none",
    color: style.color,
    "line-height": style.lineHeight,
    "text-align": style.align,
    // A text stroke is centred on the glyph edge, so half of it covers the
    // letter. Drawing it twice as wide *behind* the fill (paint-order) leaves
    // a clean outline of the requested width outside each letter.
    "-webkit-text-stroke": style.outline
      ? `${style.outline.width * 2}px ${style.outline.color}`
      : null,
    "paint-order": style.outline ? "stroke fill" : null,
    "text-shadow": style.shadow
      ? `${style.shadow.offsetX}px ${style.shadow.offsetY}px ${style.shadow.blur}px ${style.shadow.color}`
      : null,
  });
}

/**
 * Style of a line of small print: the footer (copyright, CCLI number) or
 * the scripture reference. Empty font and colour follow the slide text.
 *
 * In its own box the font size is left to {@link autofit} (pass
 * `withSize: false`); straight after the text it is fixed.
 */
export function labelStyle(label: FooterStyle, text: TextStyle, withSize: boolean): string {
  return css({
    "font-size": withSize ? `${label.fontSize}px` : null,
    "font-family": label.fontFamily || text.fontFamily,
    "font-weight": label.bold ? 700 : 400,
    color: label.color || text.color,
    "text-align": label.align ?? text.align,
    "line-height": 1.25,
    "text-shadow": text.shadow
      ? `${text.shadow.offsetX}px ${text.shadow.offsetY}px ${text.shadow.blur}px ${text.shadow.color}`
      : null,
  });
}

/**
 * Style of the verse numbers in scripture. Without a colour of their own
 * they are the verse text's colour, slightly faded.
 */
export function verseNumberStyle(numbers: VerseNumberStyle): string {
  return css({
    "font-size": `${numbers.sizePercent / 100}em`,
    color: numbers.color || null,
    opacity: numbers.color ? null : 0.8,
    "vertical-align": numbers.superscript ? "super" : "baseline",
    "line-height": numbers.superscript ? 0 : null,
    "font-weight": numbers.bold ? 700 : null,
  });
}

/**
 * CSS `background` for colours and gradients. Returns `null` for images and
 * videos, which are drawn with `<img>` / `<video>` elements instead.
 */
export function backgroundCss(background: Background): string | null {
  switch (background.type) {
    case "color":
      return background.color;
    case "linearGradient":
      return `linear-gradient(${background.angleDeg}deg, ${background.from}, ${background.to})`;
    case "image":
    case "video":
      return null;
  }
}

/**
 * The line drawings a lower third can show beside its title (see
 * `LowerThirdIcon` in `crates/sw-core/src/lower_thirds.rs`), on a 24 × 24
 * grid, drawn with a 2-unit stroke in the text's colour. Adapted from Lucide
 * (https://lucide.dev, ISC licence).
 */

import type { LowerThirdIcon } from "@simpleworship/protocol";

/** A circle as a path. */
function circle(cx: number, cy: number, r: number): string {
  return `M${cx - r} ${cy}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0`;
}

/** A rectangle with round corners as a path. */
function rect(x: number, y: number, w: number, h: number, r: number): string {
  return (
    `M${x + r} ${y}h${w - 2 * r}a${r} ${r} 0 0 1 ${r} ${r}v${h - 2 * r}` +
    `a${r} ${r} 0 0 1 ${-r} ${r}h${-(w - 2 * r)}a${r} ${r} 0 0 1 ${-r} ${-r}` +
    `v${-(h - 2 * r)}a${r} ${r} 0 0 1 ${r} ${-r}z`
  );
}

/** Each drawing's paths (the logo is a picture, not a drawing). */
export const LOWER_THIRD_ICONS: Record<Exclude<LowerThirdIcon, "none" | "logo">, string[]> = {
  facebook: ["M18 2h-3a5 5 0 0 0-5 5v3H7v4h3v8h4v-8h3l1-4h-4V7a1 1 0 0 1 1-1h3z"],
  youtube: [
    "M2.5 17a24.12 24.12 0 0 1 0-10 2 2 0 0 1 1.4-1.4 49.56 49.56 0 0 1 16.2 0A2 2 0 0 1 21.5 7a24.12 24.12 0 0 1 0 10 2 2 0 0 1-1.4 1.4 49.55 49.55 0 0 1-16.2 0A2 2 0 0 1 2.5 17",
    "m10 15 5-3-5-3z",
  ],
  instagram: [
    rect(2, 2, 20, 20, 5),
    "M16 11.37A4 4 0 1 1 12.63 8 4 4 0 0 1 16 11.37z",
    "M17.5 6.5h.01",
  ],
  whatsapp: ["M7.9 20A9 9 0 1 0 4 16.1L2 22Z"],
  phone: [
    "M22 16.92v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.5 19.5 0 0 1-6-6 19.79 19.79 0 0 1-3.07-8.67A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91a16 16 0 0 0 6 6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7A2 2 0 0 1 22 16.92z",
  ],
  web: [circle(12, 12, 10), "M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20", "M2 12h20"],
  email: [rect(2, 4, 20, 16, 2), "m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7"],
  location: ["M20 10c0 6-8 12-8 12s-8-6-8-12a8 8 0 0 1 16 0Z", circle(12, 10, 3)],
  bible: ["M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z", "M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"],
  cross: ["M12 3v18", "M7 8h10"],
};

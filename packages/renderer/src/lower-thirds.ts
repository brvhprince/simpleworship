/**
 * Helpers for lower thirds (see `crates/sw-core/src/lower_thirds.rs`):
 * which outputs show one, and how it leaves.
 */

import type { LowerThirdShown, LowerThirdStyle } from "@simpleworship/protocol";

/** Which lower thirds an output shows: the room's screens, the stream, or (the operator's live pane) all. */
export type LowerThirdPlace = "screens" | "stream" | "all";

/** The lower third `place` shows, if any. */
export function lowerThirdFor(
  shown: LowerThirdShown | null | undefined,
  place: LowerThirdPlace,
): LowerThirdShown | null {
  if (!shown) return null;
  const target = shown.item.target;
  if (place === "all" || target === "both") return shown;
  return target === place ? shown : null;
}

/** How long a lower third takes to leave, in milliseconds. */
export const LOWER_THIRD_EXIT_MS = 380;

/**
 * The exit, as a Svelte `out:` transition (`t` runs from 1 to 0): the boxed
 * styles wipe back towards their start, the others sink and fade. Instant
 * on still outputs (NDI), which are not redrawn often enough for movement.
 */
export function lowerThirdOut(
  _node: Element,
  { style, still }: { style: LowerThirdStyle; still: boolean },
) {
  const wipes = style === "bar" || style === "wipe" || style === "strap" || style === "block";
  return {
    duration: still ? 0 : LOWER_THIRD_EXIT_MS,
    css: (t: number) =>
      wipes
        ? `clip-path: inset(0 ${((1 - t) * 100).toFixed(2)}% 0 0)`
        : `opacity: ${t}; transform: translateY(${((1 - t) * 30).toFixed(1)}px)`,
  };
}

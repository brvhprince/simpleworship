/**
 * Sizes slide text to its text box.
 *
 * Normally the given font size is the *largest* size to use: when the text
 * is too long for the box at that size, the size is reduced (never below
 * {@link MIN_FONT_SIZE}) until it fits.
 *
 * With `scaleUp` (EasyWorship's "scale to fit"), short text also grows past
 * that size until it fills the box, so "Call to Worship" is not left small
 * in the middle of the screen. It is opt-in, per theme or per text box.
 */

import { tick } from "svelte";
import type { Action } from "svelte/action";

/** Text never shrinks below this size (in reference pixels), so it stays readable. */
export const MIN_FONT_SIZE = 24;

/**
 * Finds the largest whole font size between `min` and `max` for which
 * `fits(size)` is true, using a binary search (about 7 layout passes for
 * typical sizes). Returns `min` if nothing fits.
 */
export function largestFittingSize(
  fits: (size: number) => boolean,
  max: number,
  min: number = MIN_FONT_SIZE,
): number {
  let low = Math.floor(min);
  let high = Math.floor(max);
  if (high <= low || fits(high)) {
    return Math.max(high, low);
  }

  // `best` is the largest size known to fit so far.
  let best = low;
  while (low <= high) {
    const middle = Math.floor((low + high) / 2);
    if (fits(middle)) {
      best = middle;
      low = middle + 1;
    } else {
      high = middle - 1;
    }
  }
  return best;
}

/**
 * The font sizes to choose from, or `null` to use `maxSize` as it is.
 *
 * - `shrink` lets long text go down to {@link MIN_FONT_SIZE}.
 * - `grow` lets short text go up to the box's height (one line cannot be
 *   taller than that; the width check in {@link autofit} stops long words
 *   from growing too far).
 *
 * Each only moves the size its own way: with `grow` alone, text that does
 * not fit at `maxSize` stays at `maxSize`, as the shrink option is off.
 */
export function fitRange(
  maxSize: number,
  shrink: boolean,
  grow: boolean,
  boxHeight: number,
): { min: number; max: number } | null {
  if (!shrink && !grow) return null;
  return {
    min: shrink ? MIN_FONT_SIZE : maxSize,
    max: grow ? Math.max(maxSize, Math.floor(boxHeight)) : maxSize,
  };
}

/** Options for the {@link autofit} action. */
export interface AutofitOptions {
  /** Shrink long text to fit (never below {@link MIN_FONT_SIZE}). */
  enabled: boolean;
  /** The font size it was given, in reference pixels. */
  maxSize: number;
  /** Also grow short text past `maxSize` until it fills the box. */
  scaleUp?: boolean;
  /** Anything that changes the text's size (the text itself, the font…).
   *  The fit is recomputed whenever it changes. */
  content: unknown;
}

/**
 * Svelte action for a text box element. Its *first child* must be the
 * element holding the text; that child is measured against the box.
 *
 * ```svelte
 * <div class="box" use:autofit={{ enabled, maxSize, content: text }}>
 *   <div class="text">{text}</div>
 * </div>
 * ```
 */
export const autofit: Action<HTMLElement, AutofitOptions> = (box, initial) => {
  let options = initial;

  function fitText() {
    const text = box.firstElementChild as HTMLElement | null;
    if (!text) return;

    const apply = (size: number) => {
      box.style.fontSize = `${size}px`;
    };
    const fits = (size: number) => {
      apply(size);
      return text.scrollHeight <= box.clientHeight && text.scrollWidth <= box.clientWidth;
    };

    const range = fitRange(
      options.maxSize,
      options.enabled,
      options.scaleUp ?? false,
      box.clientHeight,
    );
    apply(range ? largestFittingSize(fits, range.max, range.min) : options.maxSize);
  }

  fitText();
  // Web fonts can finish loading after the first layout and change the size
  // of the text, so measure again once they are ready.
  document.fonts?.ready.then(fitText);

  return {
    update(next) {
      options = next;
      // Svelte calls `update` before it has put the new text (or swapped
      // lyrics for scripture) on the page. Measuring now would measure the
      // old content, so wait until Svelte has updated the page. (Not for an
      // animation frame: hidden windows, such as NDI's, never get one.)
      void tick().then(fitText);
    },
  };
};

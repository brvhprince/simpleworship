/**
 * Shrinks slide text until it fits its text box.
 *
 * The theme's font size is the *largest* size to use. When the text is too
 * long for the box at that size, the size is reduced (never below
 * {@link MIN_FONT_SIZE}) until it fits.
 */

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

/** Options for the {@link autofit} action. */
export interface AutofitOptions {
  /** Shrink to fit. When `false`, the text is always `maxSize`. */
  enabled: boolean;
  /** The theme's font size, in reference pixels. */
  maxSize: number;
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

    apply(options.enabled ? largestFittingSize(fits, options.maxSize) : options.maxSize);
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
      // old content, so wait for the next frame, when the new text is laid out.
      requestAnimationFrame(fitText);
    },
  };
};

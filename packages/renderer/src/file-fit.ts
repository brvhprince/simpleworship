/**
 * Each media file's own fit: the Scaling chosen for it in the Media tab
 * (Automatic, Stretch, Maintain or Zoom), as in EasyWorship. It wins over
 * the fit a theme or slide asks for, wherever the file fills the screen.
 *
 * The engine records it in the backgrounds it sends (`fileFit`), so the
 * screens, the stream, NDI and joined computers all agree. Backgrounds drawn
 * without the engine (the theme designer, the slide editor) look it up in
 * the media library instead, through {@link setFileFitLookup}.
 */

import type { Background, MediaFit, MediaScaling } from "@simpleworship/protocol";
import { getContext, setContext } from "svelte";

/** A file's own fit by its media id, or `null` for one on automatic. */
export type FileFitLookup = (mediaId: string) => MediaFit | null;

const FILE_FIT_LOOKUP = Symbol("simpleworship.fileFitLookup");

/** Provides the media library's fits to every renderer component below the caller. */
export function setFileFitLookup(lookup: FileFitLookup): void {
  setContext(FILE_FIT_LOOKUP, lookup);
}

/** The lookup set by the host; without one, only the engine's `fileFit` counts. */
export function getFileFitLookup(): FileFitLookup {
  return getContext<FileFitLookup | undefined>(FILE_FIT_LOOKUP) ?? (() => null);
}

/** The fit a file's Scaling asks for, or `null` for automatic. Mirrors `MediaScaling::fit` in sw-core. */
export function scalingFit(scaling: MediaScaling): MediaFit | null {
  switch (scaling) {
    case "stretch":
      return "stretch";
    case "maintain":
      return "contain";
    case "zoom":
      return "cover";
    default:
      return null;
  }
}

/**
 * How a background's picture, video or camera fills the screen: the file's
 * own fit if it has one (from the engine, else from `lookup`), otherwise
 * the fit the theme or slide asks for. Other backgrounds always cover.
 */
export function backgroundFit(background: Background, lookup: FileFitLookup): MediaFit {
  switch (background.type) {
    case "image":
    case "video":
      return background.fileFit ?? lookup(background.media.id) ?? background.fit;
    case "camera":
      return background.fit;
    default:
      return "cover";
  }
}

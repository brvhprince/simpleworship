/**
 * Turning media file paths into URLs the webview can load.
 *
 * Themes store absolute file paths. How a path becomes a URL depends on the
 * host: the desktop app uses Tauri's asset protocol, a browser-based display
 * would use an HTTP URL served by the app. The host provides a resolver once,
 * near the top of its component tree, with {@link setMediaResolver}.
 */

import { getContext, setContext } from "svelte";

/** Converts an absolute media file path to a loadable URL. */
export type MediaResolver = (path: string) => string;

const MEDIA_RESOLVER = Symbol("simpleworship.mediaResolver");

/** Provides the media resolver to every renderer component below the caller. */
export function setMediaResolver(resolver: MediaResolver): void {
  setContext(MEDIA_RESOLVER, resolver);
}

/** The resolver set by the host, or one that returns the path unchanged. */
export function getMediaResolver(): MediaResolver {
  return getContext<MediaResolver | undefined>(MEDIA_RESOLVER) ?? ((path) => path);
}

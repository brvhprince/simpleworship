/**
 * Turning media library references into URLs the webview can load.
 *
 * Themes and slides refer to images and videos by {@link MediaRef} (an id
 * and a file name in the library's media folder). How that becomes a URL
 * depends on the host: the desktop app uses Tauri's asset protocol, a
 * browser-based display would use HTTP URLs served by the app. The host
 * provides the URLs once, near the top of its component tree, with
 * {@link setMediaUrls}.
 */

import type { MediaRef } from "@simpleworship/protocol";
import { getContext, setContext } from "svelte";

/** How to load media files and their thumbnails. */
export interface MediaUrls {
  /** URL of the media file itself. */
  file(media: MediaRef): string;
  /** URL of its small preview image (a JPEG), used instead of playing
   *  videos in thumbnails. */
  thumbnail(media: MediaRef): string;
}

const MEDIA_URLS = Symbol("simpleworship.mediaUrls");

/** Provides media URLs to every renderer component below the caller. */
export function setMediaUrls(urls: MediaUrls): void {
  setContext(MEDIA_URLS, urls);
}

/** The URLs set by the host. Without a host, nothing can be loaded. */
export function getMediaUrls(): MediaUrls {
  return (
    getContext<MediaUrls | undefined>(MEDIA_URLS) ?? {
      file: () => "",
      thumbnail: () => "",
    }
  );
}

/**
 * Making thumbnails (and measuring videos) in the webview.
 *
 * The webview already knows how to decode every image and video it can
 * show, so it draws one frame onto a canvas and saves it as a small JPEG.
 * This avoids shipping a video tool such as ffmpeg.
 *
 * Media is served from the asset protocol, a different origin from the
 * app. The asset protocol allows the app's origin (CORS), and loading with
 * `crossOrigin = "anonymous"` keeps the canvas exportable.
 */

import { t } from "./i18n/i18n.svelte.ts";

/** Thumbnail size: 16:9, big enough for the media grid. */
const WIDTH = 384;
const HEIGHT = 216;

/** What was learned about a media file. */
export interface MediaDetails {
  jpeg: Uint8Array;
  width: number;
  height: number;
  /** Length in milliseconds, for videos. */
  durationMs: number | null;
}

/** Draws a thumbnail of an image. */
export async function imageDetails(url: string): Promise<MediaDetails> {
  const image = new Image();
  image.crossOrigin = "anonymous";
  image.src = url;
  await image.decode();
  return {
    jpeg: await encode(drawFrame(image, image.naturalWidth, image.naturalHeight)),
    width: image.naturalWidth,
    height: image.naturalHeight,
    durationMs: null,
  };
}

/** How long a file may take to open or to show a frame before giving up. */
const TIMEOUT_MS = 15_000;
/** Average brightness (0–255) below which a frame counts as black. */
const DARK = 18;

/**
 * Draws a thumbnail of a video and measures it. Rejects if the webview
 * cannot play it, or it takes too long.
 *
 * The frame comes from a little way in (videos often start black). It is
 * drawn only once it has really been decoded: WebKit reports a seek as done
 * before the frame is ready, which drew black thumbnails. If the frame is
 * still nearly black (a slow fade in), later points are tried, and the
 * brightest is kept.
 */
export async function videoDetails(url: string): Promise<MediaDetails> {
  const video = document.createElement("video");
  video.crossOrigin = "anonymous";
  video.muted = true;
  video.preload = "auto";
  video.playsInline = true;
  video.src = url;

  try {
    await waitFor(video, "loadedmetadata");
    const duration = Number.isFinite(video.duration) ? video.duration : 0;
    const times = [Math.min(1, duration / 3), duration * 0.1, duration * 0.25, duration * 0.5];

    let best: { canvas: HTMLCanvasElement; brightness: number } | null = null;
    for (const time of new Set(times.map((time) => Math.max(0, time)))) {
      await seek(video, time);
      const canvas = drawFrame(video, video.videoWidth, video.videoHeight);
      const brightness = averageBrightness(canvas);
      if (!best || brightness > best.brightness) best = { canvas, brightness };
      if (brightness >= DARK) break;
    }
    if (!best) throw new Error(t("thumbs.failed"));

    return {
      jpeg: await encode(best.canvas),
      width: video.videoWidth,
      height: video.videoHeight,
      durationMs: duration ? Math.round(duration * 1000) : null,
    };
  } finally {
    video.removeAttribute("src");
    video.load();
  }
}

/** Seeks to `time` and waits until that frame has been decoded. */
async function seek(video: HTMLVideoElement, time: number): Promise<void> {
  const seeked = waitFor(video, "seeked");
  video.currentTime = time;
  await seeked;
  // At least the current frame's data, then the frame itself presented
  // (where the webview can say so); a short wait where it cannot.
  if (video.readyState < HTMLMediaElement.HAVE_CURRENT_DATA) await waitFor(video, "loadeddata");
  await new Promise<void>((resolve) => {
    const fallback = setTimeout(resolve, 400);
    video.requestVideoFrameCallback?.(() => {
      clearTimeout(fallback);
      resolve();
    });
  });
}

/**
 * Whether the thumbnail at `url` is nearly black, like those made before
 * thumbnails waited for the video's frame. A missing or unreadable one
 * counts as not dark: it is made again when it fails to show.
 */
export async function isDarkThumbnail(url: string): Promise<boolean> {
  try {
    const image = new Image();
    image.crossOrigin = "anonymous";
    image.src = url;
    await image.decode();
    return averageBrightness(drawFrame(image, image.naturalWidth, image.naturalHeight)) < DARK;
  } catch {
    return false;
  }
}

/** Measures a sound file's length. Audio has no thumbnail. */
export async function audioDetails(url: string): Promise<{ durationMs: number | null }> {
  const audio = new Audio();
  audio.preload = "metadata";
  audio.src = url;
  await waitFor(audio, "loadedmetadata");
  const durationMs = Number.isFinite(audio.duration) ? Math.round(audio.duration * 1000) : null;
  audio.removeAttribute("src");
  return { durationMs };
}

/**
 * Resolves on `event`; rejects if the element reports an error first, or
 * nothing happens within {@link TIMEOUT_MS} (a stalled file would otherwise
 * be waited on for ever).
 */
function waitFor(element: HTMLMediaElement, event: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new TimedOut()), TIMEOUT_MS);
    element.addEventListener(
      event,
      () => {
        clearTimeout(timer);
        resolve();
      },
      { once: true },
    );
    element.addEventListener(
      "error",
      () => {
        clearTimeout(timer);
        reject(new Error(t("thumbs.cannotPlay")));
      },
      { once: true },
    );
  });
}

/** A file that took too long: worth trying again later, unlike one that cannot play. */
export class TimedOut extends Error {
  constructor() {
    super(t("thumbs.timedOut"));
  }
}

/** Draws `source` scaled to cover a thumbnail-sized canvas. */
function drawFrame(source: CanvasImageSource, width: number, height: number): HTMLCanvasElement {
  const canvas = document.createElement("canvas");
  canvas.width = WIDTH;
  canvas.height = HEIGHT;
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) throw new Error(t("thumbs.cannotDraw"));

  const scale = Math.max(WIDTH / width, HEIGHT / height);
  const drawWidth = width * scale;
  const drawHeight = height * scale;
  context.fillStyle = "#000";
  context.fillRect(0, 0, WIDTH, HEIGHT);
  context.drawImage(
    source,
    (WIDTH - drawWidth) / 2,
    (HEIGHT - drawHeight) / 2,
    drawWidth,
    drawHeight,
  );
  return canvas;
}

/** The average brightness of `canvas`, 0 (black) to 255, from a sample of its pixels. */
function averageBrightness(canvas: HTMLCanvasElement): number {
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) return 255;
  const { data } = context.getImageData(0, 0, canvas.width, canvas.height);
  let total = 0;
  let count = 0;
  // Every 16th pixel is plenty for "is it black?".
  for (let at = 0; at < data.length; at += 64) {
    total += 0.299 * (data[at] ?? 0) + 0.587 * (data[at + 1] ?? 0) + 0.114 * (data[at + 2] ?? 0);
    count += 1;
  }
  return count ? total / count : 0;
}

/** Encodes `canvas` as a JPEG. */
async function encode(canvas: HTMLCanvasElement): Promise<Uint8Array> {
  const blob = await new Promise<Blob | null>((resolve) =>
    canvas.toBlob(resolve, "image/jpeg", 0.8),
  );
  if (!blob) throw new Error(t("thumbs.failed"));
  return new Uint8Array(await blob.arrayBuffer());
}

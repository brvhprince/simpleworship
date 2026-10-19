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
    jpeg: await drawThumbnail(image, image.naturalWidth, image.naturalHeight),
    width: image.naturalWidth,
    height: image.naturalHeight,
    durationMs: null,
  };
}

/**
 * Draws a thumbnail of a video (a frame a little way in, to skip a black
 * first frame) and measures it. Rejects if the webview cannot play it.
 */
export async function videoDetails(url: string): Promise<MediaDetails> {
  const video = document.createElement("video");
  video.crossOrigin = "anonymous";
  video.muted = true;
  video.preload = "auto";
  video.src = url;

  await waitFor(video, "loadedmetadata");
  video.currentTime = Math.min(1, video.duration / 3) || 0;
  await waitFor(video, "seeked");

  const details = {
    jpeg: await drawThumbnail(video, video.videoWidth, video.videoHeight),
    width: video.videoWidth,
    height: video.videoHeight,
    durationMs: Number.isFinite(video.duration) ? Math.round(video.duration * 1000) : null,
  };
  video.removeAttribute("src");
  video.load();
  return details;
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

/** Resolves on `event`, rejects if the element reports an error first. */
function waitFor(element: HTMLMediaElement, event: string): Promise<void> {
  return new Promise((resolve, reject) => {
    element.addEventListener(event, () => resolve(), { once: true });
    element.addEventListener(
      "error",
      () => reject(new Error("This file cannot be played here. MP4 (H.264) videos work best.")),
      { once: true },
    );
  });
}

/** Draws `source` scaled to cover the thumbnail, and encodes it as JPEG. */
async function drawThumbnail(source: CanvasImageSource, width: number, height: number) {
  const canvas = document.createElement("canvas");
  canvas.width = WIDTH;
  canvas.height = HEIGHT;
  const context = canvas.getContext("2d");
  if (!context) throw new Error("Cannot draw thumbnails in this window.");

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

  const blob = await new Promise<Blob | null>((resolve) =>
    canvas.toBlob(resolve, "image/jpeg", 0.8),
  );
  if (!blob) throw new Error("Could not create a thumbnail.");
  return new Uint8Array(await blob.arrayBuffer());
}

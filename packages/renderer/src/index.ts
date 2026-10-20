/**
 * @module @simpleworship/renderer
 *
 * Draws SimpleWorship slides. The same components power the projector
 * output, the stage display and every preview thumbnail, so what the
 * operator sees is exactly what the audience sees.
 *
 * - {@link ScaledCanvas}: the 1920×1080 canvas, scaled to fit.
 * - {@link OutputView}: a live `OutputState`, with transitions.
 * - {@link SlideView}: a single still slide.
 * - {@link StreamView}: the stream look of a live `OutputState`.
 *
 * Import the bundled fonts once per app with
 * `import "@simpleworship/renderer/fonts.css";`.
 */

export { autofit } from "./autofit.ts";
export { default as BackgroundLayer } from "./BackgroundLayer.svelte";
export {
  type CameraUse,
  getCameraUse,
  listCameras,
  onCamerasChanged,
  setCameraUse,
} from "./camera.ts";
export { CANVAS_HEIGHT, CANVAS_WIDTH } from "./canvas.ts";
export { default as ElementsLayer } from "./ElementsLayer.svelte";
export {
  frameStyle,
  lineStyle,
  objectFit,
  runStyle,
  shapeStyle,
  textBoxStyle as elementBoxStyle,
  verticalJustify,
} from "./elements.ts";
export { frameAt } from "./frames.ts";
export { getMediaUrls, type MediaUrls, setMediaUrls } from "./media.ts";
export { default as OutputView } from "./OutputView.svelte";
export { default as ScaledCanvas } from "./ScaledCanvas.svelte";
export { default as SlideView } from "./SlideView.svelte";
export { default as StreamView } from "./StreamView.svelte";
export { textStyle } from "./styles.ts";
export { formatClock, remainingMs, stageTimerMs } from "./timing.ts";

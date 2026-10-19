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
 *
 * Import the bundled fonts once per app with
 * `import "@simpleworship/renderer/fonts.css";`.
 */

export { CANVAS_HEIGHT, CANVAS_WIDTH } from "./canvas.ts";
export { getMediaUrls, type MediaUrls, setMediaUrls } from "./media.ts";
export { default as OutputView } from "./OutputView.svelte";
export { default as ScaledCanvas } from "./ScaledCanvas.svelte";
export { default as SlideView } from "./SlideView.svelte";

import type { Presentation, RenderFrame } from "@simpleworship/protocol";

/**
 * The frame for one slide of a presentation, for drawing thumbnails.
 * Mirrors `Presentation::frame` in `crates/sw-core/src/presentation.rs`.
 */
export function frameAt(presentation: Presentation, index: number): RenderFrame | null {
  const slide = presentation.slides[index];
  if (!slide) return null;
  const theme = slide.background
    ? { ...presentation.theme, background: slide.background }
    : presentation.theme;
  return {
    text: slide.text,
    scripture: slide.scripture,
    caption: slide.caption,
    // Thumbnails show a countdown's message; only live screens tick.
    countdown: null,
    footer: presentation.footer,
    theme,
  };
}

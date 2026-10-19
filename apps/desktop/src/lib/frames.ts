import type { Presentation, RenderFrame } from "@simpleworship/protocol";

/**
 * The frame for one slide of a presentation, for drawing thumbnails.
 * Mirrors `Presentation::frame` in `crates/sw-core/src/presentation.rs`.
 */
export function frameAt(presentation: Presentation, index: number): RenderFrame | null {
  const slide = presentation.slides[index];
  if (!slide) return null;
  return { text: slide.text, footer: presentation.footer, theme: presentation.theme };
}

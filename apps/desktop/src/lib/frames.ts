import type { Presentation, RenderFrame } from "@simpleworship/protocol";

/**
 * The frame for one slide of a presentation, for drawing thumbnails.
 * Mirrors `Presentation::frame` in `crates/sw-core/src/presentation.rs`:
 * the slide's own theme (if any), then the item's background override,
 * then the slide's own background, then its text adjustments.
 */
export function frameAt(presentation: Presentation, index: number): RenderFrame | null {
  const slide = presentation.slides[index];
  if (!slide) return null;

  const base = slide.look?.theme ?? presentation.theme;
  const background = slide.background ?? presentation.backgroundOverride ?? base.background;
  const style = slide.style;
  const theme = {
    ...base,
    background,
    text: {
      ...base.text,
      fontSize: style?.fontSize ?? base.text.fontSize,
      color: style?.color ?? base.text.color,
      align: style?.align ?? base.text.align,
    },
  };

  // The reference can be shown on the first slide of a passage only.
  const firstOnly = slide.scripture && index > 0 && !theme.scripture.referenceOnEverySlide;

  return {
    text: slide.text,
    rich: slide.rich,
    scripture: slide.scripture,
    caption: firstOnly ? null : slide.caption,
    // Thumbnails show a countdown's message; only live screens tick.
    countdown: null,
    footer: presentation.footer,
    theme,
  };
}

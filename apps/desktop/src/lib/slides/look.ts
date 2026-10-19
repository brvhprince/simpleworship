/**
 * What a presentation slide looks like in the editor: its theme (the
 * slide's own, the presentation's, or the default for presentations) and
 * the frame the renderer draws. Mirrors `Presentation::from_deck` and
 * `ThemeLibrary::resolve` in `crates/sw-core`.
 */

import type {
  DeckSlide,
  RenderFrame,
  SlideDeck,
  Theme,
  ThemeLibrary,
} from "@simpleworship/protocol";

/** The theme a slide is drawn with. */
export function slideTheme(library: ThemeLibrary, deck: SlideDeck, slide: DeckSlide): Theme | null {
  const { themes, defaults } = library;
  const find = (id: string | null) => (id ? themes.find((record) => record.id === id) : undefined);
  const record =
    find(slide.themeId) ??
    find(deck.themeId) ??
    find(defaults.presentation) ??
    find(defaults.text) ??
    themes[0];
  return record?.theme ?? null;
}

/** The frame for drawing a slide (its elements over its background). */
export function slideFrame(
  library: ThemeLibrary,
  deck: SlideDeck,
  slide: DeckSlide,
): RenderFrame | null {
  const theme = slideTheme(library, deck, slide);
  if (!theme) return null;
  return {
    text: "",
    rich: null,
    scripture: null,
    caption: null,
    countdown: null,
    footer: null,
    elements: slide.elements,
    theme: { ...theme, background: slide.background ?? theme.background },
  };
}

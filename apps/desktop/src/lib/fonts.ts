/**
 * Font family lists for themes.
 */

/** A font family list with sensible fallbacks for when a font is missing. */
export function fontFamily(name: string): string {
  const serif = /serif|merriweather|georgia|times|garamond/i.test(name) && !/sans/i.test(name);
  return `"${name}", ${serif ? "Georgia, serif" : '"Open Sans", Helvetica, Arial, sans-serif'}`;
}

/** The first family in a font list, e.g. "Montserrat". */
export function firstFamily(list: string): string {
  return (
    list
      .split(",")[0]
      ?.trim()
      .replace(/^["']|["']$/g, "") ?? ""
  );
}

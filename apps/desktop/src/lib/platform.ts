/** `true` on macOS, where shortcuts use ⌘ instead of Ctrl. */
export const isMac = /Mac|iPhone|iPad/.test(navigator.userAgent);

/** How to write the main shortcut modifier on this platform, e.g. "⌘S" or "Ctrl+S". */
export function withModifier(key: string): string {
  return isMac ? `⌘${key}` : `Ctrl+${key}`;
}

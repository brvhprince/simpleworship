/**
 * Release notes, as shown in the update window.
 */

/**
 * The release notes (CHANGELOG.md's section, in Markdown) as plain text
 * for the update window: no **stars**, `backticks` or [link](address)
 * syntax; the "- " of each point stays.
 */
export function plainText(markdown: string): string {
  return markdown
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .replace(/\*\*([^*]+)\*\*/g, "$1")
    .replace(/`([^`]+)`/g, "$1")
    .trim();
}

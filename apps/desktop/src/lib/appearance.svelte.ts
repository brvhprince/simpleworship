/**
 * The operator window's appearance: dark (the default, for a dim room during
 * the service), light (for bright rooms and preparation in daylight), or the
 * same as this computer. Remembered on this computer, like the language, and
 * applied as `data-theme` on the root element (see `@simpleworship/ui/tokens.css`).
 *
 * Only the operator window follows it. The screens, stage display, stream
 * and NDI windows are always dark: they show slides, and a light page around
 * a letterboxed slide would light up the room.
 */

export type Appearance = "dark" | "light" | "system";

const STORAGE_KEY = "simpleworship.appearance";
const LIGHT = "(prefers-color-scheme: light)";

function stored(): Appearance {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    return value === "light" || value === "system" ? value : "dark";
  } catch {
    return "dark";
  }
}

class AppearanceStore {
  /** The choice in Settings. */
  chosen: Appearance = $state(stored());
  /** Whether this window follows the choice (only the operator window does). */
  #follows = false;

  /** The palette in use: the choice, with "system" resolved. */
  get resolved(): "dark" | "light" {
    if (this.chosen !== "system") return this.chosen;
    return window.matchMedia(LIGHT).matches ? "light" : "dark";
  }

  /**
   * Applies the palette to this window, before anything is drawn, and keeps
   * it current: a choice made in another window, or the computer switching
   * between light and dark. Windows that show slides stay dark.
   */
  start(follows: boolean): void {
    this.#follows = follows;
    this.#apply();
    if (!follows) return;
    window.addEventListener("storage", (event) => {
      if (event.key === STORAGE_KEY) {
        this.chosen = stored();
        this.#apply();
      }
    });
    window.matchMedia(LIGHT).addEventListener("change", () => this.#apply());
  }

  /** Chooses an appearance, here and in every window. */
  choose(appearance: Appearance): void {
    this.chosen = appearance;
    try {
      if (appearance === "dark") localStorage.removeItem(STORAGE_KEY);
      else localStorage.setItem(STORAGE_KEY, appearance);
    } catch {
      // Not remembered, but used for now.
    }
    this.#apply();
  }

  #apply(): void {
    document.documentElement.dataset.theme = this.#follows ? this.resolved : "dark";
  }
}

export const appearance = new AppearanceStore();

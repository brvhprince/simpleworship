/**
 * A signal that the song library changed outside the Songs tab (for example
 * songs added from a schedule), so lists showing it know to reload.
 */
class LibrarySignal {
  /** Goes up by one on every change. Read it in an effect to depend on it. */
  version = $state(0);

  /** Announces that the library changed. */
  changed(): void {
    this.version += 1;
  }
}

export const library = new LibrarySignal();

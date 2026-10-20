/**
 * A signal that the song library changed, so lists showing it know to
 * reload: songs saved, deleted or imported here, or on the computer this
 * one joined (`songs://changed`, see `src-tauri/src/commands/songs.rs`), or
 * added from a schedule.
 */

import { listen } from "@tauri-apps/api/event";

/** Must match `SONGS_EVENT` in `src-tauri/src/commands/songs.rs`. */
const SONGS_EVENT = "songs://changed";

class LibrarySignal {
  /** Goes up by one on every change. Read it in an effect to depend on it. */
  version = $state(0);

  /** Announces that the library changed. */
  changed(): void {
    this.version += 1;
  }

  /** Follows changes made anywhere. Resolves to a function that stops. */
  connect(): Promise<() => void> {
    return listen(SONGS_EVENT, () => this.changed());
  }
}

export const library = new LibrarySignal();

/**
 * The assistant, from the operator's side: whether it is on (and has a key),
 * and its tasks. It talks to Claude from Rust (see
 * `src-tauri/src/assistant.rs`), always from this computer and with this
 * computer's key, even when it has joined another.
 *
 * The tasks return ideas; finding them in the library (songs, passages) is
 * done here with the usual commands, so the library itself is never sent.
 */

import type {
  AssistantSettings,
  AssistantStatus,
  SermonItem,
  SongIdea,
  TidiedSong,
  TranslatedSong,
  TranslateTo,
  VerseIdea,
} from "@simpleworship/protocol";
import { invoke } from "@tauri-apps/api/core";

class AssistantStore {
  status: AssistantStatus | null = $state.raw(null);
  /** A task is waiting for its answer. */
  busy = $state(false);

  /** The assistant's buttons are shown: it is on, and has a key. */
  get ready(): boolean {
    return (this.status?.settings.enabled ?? false) && (this.status?.hasKey ?? false);
  }

  async load(): Promise<void> {
    this.status = await invoke<AssistantStatus>("assistant_status");
  }

  async configure(settings: AssistantSettings): Promise<void> {
    this.status = await invoke<AssistantStatus>("assistant_configure", { settings });
  }

  /** Checks the key with Anthropic, then saves it in the keychain. */
  async setKey(key: string): Promise<void> {
    this.status = await invoke<AssistantStatus>("assistant_set_key", { key });
  }

  async forgetKey(): Promise<void> {
    this.status = await invoke<AssistantStatus>("assistant_forget_key");
  }

  findSongs(description: string): Promise<SongIdea[]> {
    return this.#task("assistant_find_songs", { description });
  }

  findVerses(theme: string): Promise<VerseIdea[]> {
    return this.#task("assistant_find_verses", { theme });
  }

  tidyLyrics(text: string): Promise<TidiedSong> {
    return this.#task("assistant_tidy_lyrics", { text });
  }

  translate(title: string, lyrics: string, language: TranslateTo): Promise<TranslatedSong> {
    return this.#task("assistant_translate", { title, lyrics, language });
  }

  sermonSlides(notes: string): Promise<SermonItem[]> {
    return this.#task("assistant_sermon_slides", { notes });
  }

  async #task<T>(command: string, args: Record<string, unknown>): Promise<T> {
    this.busy = true;
    try {
      return await invoke<T>(command, args);
    } finally {
      this.busy = false;
    }
  }
}

export const assistant = new AssistantStore();

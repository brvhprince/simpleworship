/**
 * Questions for the operator ("Save changes?"), asked in a dialog and
 * answered with a promise:
 *
 * ```ts
 * const answer = await prompts.ask({ title, message, buttons, cancel: "cancel" });
 * ```
 *
 * `PromptHost.svelte` shows the current question. Only one question is
 * shown at a time; a second one waits for the first to be answered.
 */

/** One answer button. */
export interface PromptButton {
  /** What `ask` resolves to when this button is chosen. */
  id: string;
  label: string;
  variant?: "default" | "primary" | "live";
}

/** A question to ask. */
export interface Prompt {
  title: string;
  message: string;
  /** The buttons, left to right. Put the recommended one last. */
  buttons: PromptButton[];
  /** The answer used when the operator presses Escape. */
  cancel: string;
}

interface PendingPrompt extends Prompt {
  answer: (id: string) => void;
}

class Prompts {
  /** The question on screen, if any. */
  current: PendingPrompt | null = $state(null);

  #queue: PendingPrompt[] = [];

  /** Asks a question and resolves to the id of the chosen button. */
  ask(prompt: Prompt): Promise<string> {
    return new Promise((resolve) => {
      const pending: PendingPrompt = {
        ...prompt,
        answer: (id) => {
          resolve(id);
          this.current = this.#queue.shift() ?? null;
        },
      };
      if (this.current) this.#queue.push(pending);
      else this.current = pending;
    });
  }
}

export const prompts = new Prompts();

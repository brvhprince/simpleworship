/**
 * Behaviour shared by the projector and stage windows.
 */

import type { OutputKind } from "@simpleworship/protocol";
import { closeOutput } from "./api.ts";

/**
 * Keyboard handler for an output window: Escape closes it.
 *
 * Output windows normally never have focus; the operator window keeps it.
 * But if one does (for example after clicking a full-screen output on a
 * single-screen laptop), Escape is the obvious way out.
 */
export function closeOnEscape(kind: OutputKind) {
  return (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      void closeOutput(kind);
    }
  };
}

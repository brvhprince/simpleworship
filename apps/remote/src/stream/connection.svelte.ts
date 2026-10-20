/**
 * The stream page's connection to SimpleWorship: watch only. It is sent the
 * whole state when it connects, then every change to the output.
 *
 * It never gives up: if SimpleWorship restarts, the network drops or the
 * browser source is turned off and on again, the page carries on by
 * itself, so nobody has to touch OBS during a service.
 */

import type { EngineEvent, OutputState, ServerMessage } from "@simpleworship/protocol";

/** The longest wait between attempts to reconnect. */
const MAX_RETRY_MS = 5000;

export class StreamConnection {
  /** What the output shows; `null` until connected. */
  output: OutputState | null = $state.raw(null);

  readonly #key: string;
  #socket: WebSocket | null = null;
  #retry = 500;
  #stopped = false;

  constructor(key: string) {
    this.#key = key;
  }

  /** Connects, and keeps reconnecting until {@link stop}. */
  start(): void {
    this.#stopped = false;
    const protocol = location.protocol === "https:" ? "wss" : "ws";
    const socket = new WebSocket(`${protocol}://${location.host}/ws?token=${this.#key}`);
    this.#socket = socket;
    socket.onmessage = (message) => this.#receive(JSON.parse(String(message.data)));
    socket.onclose = () => {
      if (this.#socket !== socket || this.#stopped) return;
      setTimeout(() => this.start(), this.#retry);
      this.#retry = Math.min(this.#retry * 2, MAX_RETRY_MS);
    };
  }

  stop(): void {
    this.#stopped = true;
    this.#socket?.close();
    this.#socket = null;
  }

  #receive(message: ServerMessage): void {
    if (message.type === "welcome") {
      this.output = message.snapshot.output;
      this.#retry = 500;
    } else if (message.type === "event") {
      this.#apply(message.event);
    }
  }

  #apply(event: EngineEvent): void {
    if (event.type === "outputChanged") this.output = event.output;
  }
}

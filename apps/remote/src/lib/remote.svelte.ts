/**
 * The phone's connection to SimpleWorship.
 *
 * 1. **Pairing** (once): the phone sends the 6-digit code from the
 *    operator's screen and its name, waits while the operator allows it,
 *    and keeps the token it gets (on this phone only).
 * 2. **Connecting**: a WebSocket to the same computer. The first message is
 *    the whole engine state; after that, every change arrives as it
 *    happens, so the phone always shows what the room sees.
 * 3. If the Wi-Fi drops, it reconnects by itself (the screen says so
 *    rather than showing stale slides).
 *
 * The messages are described in `crates/sw-server/src/protocol.rs`. The
 * computer checks every command against this device's role.
 */

import type {
  Command,
  EngineEvent,
  EngineSnapshot,
  MediaRef,
  OutputState,
  PaneState,
  Role,
  Schedule,
  ServerMessage,
  StageState,
} from "@simpleworship/protocol";

/** Remembered on this phone, per SimpleWorship computer. */
const TOKEN_KEY = `simpleworship.remote.${location.host}`;

/** Waits between reconnection attempts, growing to this. */
const MAX_RETRY_MS = 5000;

export type Status = "pairing" | "waiting" | "connecting" | "connected" | "offline" | "removed";

/** A command or question waiting for its answer. */
interface Pending {
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
}

class Remote {
  status: Status = $state(storedToken() ? "connecting" : "pairing");
  /** The name the operator knows this device by. */
  name = $state("");
  /** What this device may do. */
  role: Role | null = $state(null);
  /** The last error, for a few seconds. */
  error: string | null = $state(null);

  // A mirror of the engine's state, kept up to date by events.
  schedule: Schedule = $state.raw({ items: [] });
  preview: PaneState | null = $state.raw(null);
  live: PaneState | null = $state.raw(null);
  output: OutputState | null = $state.raw(null);
  stage: StageState = $state.raw({ timer: null, message: null });
  snapshot: EngineSnapshot | null = $state.raw(null);

  #socket: WebSocket | null = null;
  #nextId = 1;
  #pending = new Map<number, Pending>();
  #retry = 500;
  #errorTimer: ReturnType<typeof setTimeout> | undefined;

  /** The token this phone was given, if it is paired. */
  get token(): string | null {
    return storedToken();
  }

  /** Where a media file is on the computer (pictures and video stills only). */
  mediaUrl(media: MediaRef): string {
    return `/media/${encodeURIComponent(media.file)}?token=${this.token ?? ""}`;
  }

  thumbnailUrl(media: MediaRef): string {
    return `/thumbnails/${media.id}.jpg?token=${this.token ?? ""}`;
  }

  // ── Pairing ────────────────────────────────────────────────────────────

  /**
   * Asks to pair with `code`. Resolves once the operator has answered:
   * `true` if allowed (then the phone connects), `false` if refused.
   */
  async pair(code: string, name: string): Promise<boolean> {
    this.status = "waiting";
    try {
      const response = await fetch("/api/pair", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ code: code.trim(), name: name.trim() }),
      });
      if (!response.ok) {
        this.showError(await response.text());
        this.status = "pairing";
        return false;
      }
      const answer = (await response.json()) as { token: string };
      storeToken(answer.token);
      this.connect();
      return true;
    } catch {
      this.showError("SimpleWorship could not be reached. Is this phone on the church Wi-Fi?");
      this.status = "pairing";
      return false;
    }
  }

  /** Forgets this phone's pairing (to pair again, or with another computer). */
  forget(): void {
    storeToken(null);
    this.#socket?.close();
    this.#socket = null;
    this.status = "pairing";
    this.role = null;
  }

  // ── The connection ─────────────────────────────────────────────────────

  /** Connects (or reconnects) to SimpleWorship. */
  connect(): void {
    const token = this.token;
    if (!token) {
      this.status = "pairing";
      return;
    }
    this.#socket?.close();
    const protocol = location.protocol === "https:" ? "wss" : "ws";
    const socket = new WebSocket(`${protocol}://${location.host}/ws?token=${token}`);
    this.#socket = socket;
    if (this.status !== "connected") this.status = "connecting";

    socket.onmessage = (message) => this.#receive(JSON.parse(String(message.data)));
    socket.onclose = () => {
      if (this.#socket !== socket) return;
      this.#socket = null;
      for (const pending of this.#pending.values()) pending.reject(new Error("Disconnected"));
      this.#pending.clear();
      if (this.status === "removed" || this.status === "pairing") return;
      this.status = "offline";
      void this.#retryLater(token);
    };
  }

  /**
   * After a dropped connection: if the computer says this phone is no
   * longer allowed, pair again; otherwise (Wi-Fi trouble) try again soon.
   */
  async #retryLater(token: string): Promise<void> {
    try {
      const answer = await fetch(`/api/check?token=${token}`);
      if (answer.status === 401) {
        storeToken(null);
        this.role = null;
        this.status = "pairing";
        return;
      }
    } catch {
      // Not reachable right now: keep trying.
    }
    setTimeout(() => this.connect(), this.#retry);
    this.#retry = Math.min(this.#retry * 2, MAX_RETRY_MS);
  }

  /** Sends a command to the engine. Errors are shown, not thrown. */
  dispatch(command: Command): Promise<void> {
    return this.#send({ type: "dispatch", command }).then(
      () => undefined,
      (error: Error) => this.showError(error.message),
    );
  }

  /** Asks the computer a question, e.g. `call("songs.search", { query })`. */
  call<T>(method: string, params: Record<string, unknown> = {}): Promise<T> {
    return this.#send({ type: "call", method, params }) as Promise<T>;
  }

  showError(message: string): void {
    this.error = message;
    clearTimeout(this.#errorTimer);
    this.#errorTimer = setTimeout(() => (this.error = null), 5000);
  }

  #send(message: Record<string, unknown>): Promise<unknown> {
    const socket = this.#socket;
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      return Promise.reject(new Error("Not connected to SimpleWorship."));
    }
    const id = this.#nextId++;
    return new Promise((resolve, reject) => {
      this.#pending.set(id, { resolve, reject });
      socket.send(JSON.stringify({ ...message, id }));
    });
  }

  #receive(message: ServerMessage): void {
    switch (message.type) {
      case "welcome":
        this.name = message.name;
        this.role = message.role;
        this.#applySnapshot(message.snapshot);
        this.status = "connected";
        this.#retry = 500;
        break;
      case "event":
        this.#apply(message.event);
        break;
      case "result": {
        const pending = this.#pending.get(message.id);
        this.#pending.delete(message.id);
        if (message.error) pending?.reject(new Error(message.error));
        else pending?.resolve(message.value);
        break;
      }
      case "roleChanged":
        this.role = message.role;
        break;
      case "removed":
        storeToken(null);
        this.role = null;
        this.status = "removed";
        break;
    }
  }

  #apply(event: EngineEvent): void {
    switch (event.type) {
      case "previewChanged":
        this.preview = event.preview;
        break;
      case "liveChanged":
        this.live = event.live;
        break;
      case "outputChanged":
        this.output = event.output;
        break;
      case "scheduleChanged":
        this.schedule = event.schedule;
        break;
      case "stageChanged":
        this.stage = event.stage;
        break;
      default:
        // Themes, audio and transitions are the operator's business.
        break;
    }
  }

  #applySnapshot(snapshot: EngineSnapshot): void {
    this.snapshot = snapshot;
    this.schedule = snapshot.schedule;
    this.preview = snapshot.preview;
    this.live = snapshot.live;
    this.output = snapshot.output;
    this.stage = snapshot.stage;
  }
}

function storedToken(): string | null {
  try {
    return localStorage.getItem(TOKEN_KEY);
  } catch {
    return null;
  }
}

function storeToken(token: string | null): void {
  try {
    if (token) localStorage.setItem(TOKEN_KEY, token);
    else localStorage.removeItem(TOKEN_KEY);
  } catch {
    // Private browsing: the phone pairs again next time.
  }
}

export const remote = new Remote();

import { listen } from "@tauri-apps/api/event";

/** Registered terminal output listeners. Key = session ID. */
const outputListeners = new Map<number, (data: Uint8Array) => void>();

let listening = false;

/** Start listening for terminal events from Tauri. Call once at app init. */
export async function initTerminalEvents(): Promise<void> {
  if (listening) return;
  listening = true;

  await listen<{ session_id: number; data: string }>(
    "terminal-output",
    (event) => {
      const { session_id, data } = event.payload;
      const listener = outputListeners.get(session_id);
      if (listener) {
        // Decode base64 to Uint8Array
        const bytes = Uint8Array.from(atob(data), (c) => c.charCodeAt(0));
        listener(bytes);
      }
    },
  );

  await listen<{ session_id: number; exit_code: number | null }>(
    "terminal-exit",
    (event) => {
      outputListeners.delete(event.payload.session_id);
    },
  );
}

/** Register a callback for terminal output from a specific session. */
export function onTerminalOutput(
  sessionId: number,
  callback: (data: Uint8Array) => void,
): void {
  outputListeners.set(sessionId, callback);
}

/** Unregister a terminal output callback. */
export function offTerminalOutput(sessionId: number): void {
  outputListeners.delete(sessionId);
}

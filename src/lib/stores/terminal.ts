import { listen } from "@tauri-apps/api/event";
import {
  removeTerminalTabBySession,
  onTerminalCwdChanged,
  onTerminalProcessChanged,
} from "./tabs";

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
        const bytes = Uint8Array.from(atob(data), (c) => c.charCodeAt(0));
        listener(bytes);
      }
    },
  );

  await listen<{ session_id: number; exit_code: number | null }>(
    "terminal-exit",
    (event) => {
      const { session_id } = event.payload;
      outputListeners.delete(session_id);
      // Auto-remove the terminal tab when the shell exits
      removeTerminalTabBySession(session_id);
    },
  );

  await listen<{ session_id: number; cwd: string }>(
    "terminal-cwd-changed",
    (event) => {
      const { session_id, cwd } = event.payload;
      onTerminalCwdChanged(session_id, cwd);
    },
  );

  await listen<{ session_id: number; process_name: string | null }>(
    "terminal-process-changed",
    (event) => {
      const { session_id, process_name } = event.payload;
      onTerminalProcessChanged(session_id, process_name);
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

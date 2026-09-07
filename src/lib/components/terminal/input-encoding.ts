/**
 * Encode keyboard input from xterm.js for the `terminal_write` IPC command,
 * which expects base64 of the *UTF-8 bytes* to write to the PTY.
 *
 * `btoa(data)` is not that: it maps each UTF-16 code unit to one byte, so
 * `ñ` (U+00F1) reached the shell as the lone byte 0xF1 — invalid UTF-8 —
 * and anything above U+00FF (`€`, `—`, emoji, most pasted text) threw
 * `InvalidCharacterError` and the keystroke was silently dropped.
 */
export function encodeTerminalInput(data: string): string {
  const bytes = new TextEncoder().encode(data);
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary);
}

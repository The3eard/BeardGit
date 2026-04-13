/**
 * Strip ANSI escape codes from a string, returning plain text.
 * Used for task list sidebar preview (last line of output shown as plain text).
 */
// eslint-disable-next-line no-control-regex
const ANSI_REGEX =
  /\x1b\[[0-9;]*[a-zA-Z]|\x1b\].*?(?:\x07|\x1b\\)|\x1b[()][AB012]|\x1b[>=<]|\x0f|\x0e/g;

export function stripAnsi(text: string): string {
  return text.replace(ANSI_REGEX, "");
}

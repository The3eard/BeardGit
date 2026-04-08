/**
 * Known git configuration keys with allowed enum values.
 *
 * Keys in this map get a dropdown selector in the config editor instead of
 * a plain text input. Values are the allowed options shown in the dropdown.
 */
export const KNOWN_ENUM_KEYS: Record<string, string[]> = {
  "core.autocrlf": ["true", "false", "input"],
  "core.eol": ["lf", "crlf", "native"],
  "core.filemode": ["true", "false"],
  "core.ignorecase": ["true", "false"],
  "core.symlinks": ["true", "false"],
  "core.trustctime": ["true", "false"],
  "core.whitespace": ["trailing-space", "space-before-tab", "indent-with-non-tab", "cr-at-eol"],
  "pull.rebase": ["true", "false", "merges", "interactive"],
  "pull.ff": ["true", "false", "only"],
  "push.default": ["nothing", "current", "upstream", "tracking", "simple", "matching"],
  "push.autoSetupRemote": ["true", "false"],
  "merge.ff": ["true", "false", "only"],
  "merge.conflictstyle": ["merge", "diff3", "zdiff3"],
  "rebase.autoSquash": ["true", "false"],
  "rebase.autoStash": ["true", "false"],
  "rebase.updateRefs": ["true", "false"],
  "fetch.prune": ["true", "false"],
  "diff.algorithm": ["default", "minimal", "patience", "histogram"],
  "diff.colorMoved": ["no", "default", "plain", "blocks", "zebra", "dimmed-zebra"],
  "init.defaultBranch": ["main", "master"],
  "color.ui": ["auto", "always", "never"],
  "credential.helper": ["cache", "store", "osxkeychain", "manager", "manager-core"],
};

/**
 * Check whether a config key has known enum values.
 */
export function isEnumKey(key: string): boolean {
  return key in KNOWN_ENUM_KEYS;
}

/**
 * Get the allowed values for a known enum key, or null.
 */
export function getEnumValues(key: string): string[] | null {
  return KNOWN_ENUM_KEYS[key] ?? null;
}

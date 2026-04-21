/**
 * Repo-config dialog store — load / dirty-track / apply cycle for the
 * per-repo "Repo settings" dialog (Phase 6 of the repo-config-cli plan).
 *
 * The store carries two snapshots of the remote config:
 *
 *   - `before`  — the last-loaded-or-saved state. Readonly from the
 *                 dialog's point of view.
 *   - `current` — the edited copy the user is mutating. Writes go here
 *                 via `updateCurrent()`, never directly to `before`.
 *
 * `patch` is a derived snapshot diff (`before → current`) built with
 * {@link buildPatch}. The Save button is enabled iff the patch is
 * non-empty (see {@link isPatchEmpty}).
 *
 * Open/closed state for the dialog itself lives in
 * {@link repoConfigDialogOpen} so three independent surfaces can open
 * it (sidebar cog, repo-tab context menu, future programmatic openers)
 * without lifting state into each caller. This mirrors the
 * `tasksPopover.ts` pattern and keeps the data-bearing store importable
 * from non-UI contexts.
 */

import { derived, get, writable } from "svelte/store";
import type {
  RemoteRepoConfig,
  RemoteRepoConfigPatch,
  PatchValue,
  Visibility,
} from "../types/repoConfig";
import { patchClear, patchSet, patchUnchanged } from "../types/repoConfig";
import { loadRemoteRepoConfig } from "../api/tauri";

// ───────────────────────────────────────────────────────────────────────────
// Dialog open/closed flag
// ───────────────────────────────────────────────────────────────────────────

/**
 * Whether the `RepoConfigDialog` is currently visible.
 *
 * Defaults to `false` on app start — the dialog is always opt-in via
 * the sidebar cog icon or the repo-tab context menu.
 */
export const repoConfigDialogOpen = writable<boolean>(false);

/** Explicitly open the dialog. */
export function openRepoConfigDialog(): void {
  repoConfigDialogOpen.set(true);
}

/** Explicitly close the dialog. */
export function closeRepoConfigDialog(): void {
  repoConfigDialogOpen.set(false);
}

// ───────────────────────────────────────────────────────────────────────────
// Load / dirty-tracking state
// ───────────────────────────────────────────────────────────────────────────

/** Shape of the reactive state carried by {@link repoConfigStore}. */
export interface RepoConfigState {
  /** Repo path the config was loaded for; `null` before the first load. */
  repoPath: string | null;
  /** Last-loaded config snapshot. Compared against `current` for diffing. */
  before: RemoteRepoConfig | null;
  /** Mutable working copy — writes from the dialog go here. */
  current: RemoteRepoConfig | null;
  /** `true` while the initial load is in flight. */
  loading: boolean;
  /** Structured error string (from the backend), or `null` on success. */
  error: string | null;
}

/** Default (empty) store state. */
export function initialRepoConfigState(): RepoConfigState {
  return {
    repoPath: null,
    before: null,
    current: null,
    loading: false,
    error: null,
  };
}

/** Central reactive state for the Repo Settings dialog. */
export const repoConfigStore = writable<RepoConfigState>(
  initialRepoConfigState(),
);

/**
 * Derived snapshot: the minimal patch from `before` to `current`.
 *
 * Subscribers use this to enable/disable the Save button and to build
 * the payload for `apply_remote_repo_config`. Returns `null` when no
 * config is loaded.
 */
export const repoConfigPatch = derived(repoConfigStore, ($s) => {
  if (!$s.before || !$s.current) return null;
  return buildPatch($s.before, $s.current);
});

/** Derived convenience: `true` when the patch is empty (Save disabled). */
export const repoConfigDirty = derived(repoConfigPatch, ($p) => {
  if (!$p) return false;
  return !isPatchEmpty($p);
});

/** Begin tracking a freshly loaded config snapshot. */
export function setLoadedConfig(
  repoPath: string,
  config: RemoteRepoConfig,
): void {
  repoConfigStore.set({
    repoPath,
    before: config,
    // Deep clone so edits to `current` don't mutate `before`.
    current: cloneConfig(config),
    loading: false,
    error: null,
  });
}

/** Flip the store into the loading state for a fresh repo path. */
export function setLoading(repoPath: string): void {
  repoConfigStore.set({
    repoPath,
    before: null,
    current: null,
    loading: true,
    error: null,
  });
}

/** Set a structured error on the store (clears loading). */
export function setLoadError(message: string): void {
  repoConfigStore.update((s) => ({
    ...s,
    loading: false,
    error: message,
  }));
}

/** Reset the store to the fully empty initial shape. */
export function resetRepoConfigStore(): void {
  repoConfigStore.set(initialRepoConfigState());
}

/**
 * Mutate the `current` working copy via an updater callback. The
 * callback receives a mutable draft — changes are written back into
 * the store atomically. A no-op when no config has been loaded yet.
 */
export function updateCurrent(
  updater: (draft: RemoteRepoConfig) => void,
): void {
  repoConfigStore.update((s) => {
    if (!s.current) return s;
    const next = cloneConfig(s.current);
    updater(next);
    return { ...s, current: next };
  });
}

/** Replace `before` with the freshly applied config (post-save). */
export function commitSavedConfig(config: RemoteRepoConfig): void {
  repoConfigStore.update((s) => ({
    ...s,
    before: config,
    current: cloneConfig(config),
    error: null,
  }));
}

// ───────────────────────────────────────────────────────────────────────────
// Diff → patch
// ───────────────────────────────────────────────────────────────────────────

/**
 * Build a {@link RemoteRepoConfigPatch} describing the differences
 * between two config snapshots.
 *
 * Mirrors the Rust `diff_config` in `repo_config.rs`. Topics are
 * expressed as sorted add/remove deltas rather than a full
 * replacement; homepage uses the tri-state `PatchValue` so the
 * "explicitly clear" signal survives the IPC boundary.
 */
export function buildPatch(
  before: RemoteRepoConfig,
  after: RemoteRepoConfig,
): RemoteRepoConfigPatch {
  const description =
    before.description !== after.description ? after.description : undefined;

  let homepage: PatchValue<string>;
  if (before.homepage === after.homepage) {
    homepage = patchUnchanged<string>();
  } else if (after.homepage === null || after.homepage === "") {
    homepage = patchClear<string>();
  } else {
    homepage = patchSet<string>(after.homepage);
  }

  const beforeTopics = new Set(before.topics);
  const afterTopics = new Set(after.topics);
  const topics_added = [...afterTopics]
    .filter((t) => !beforeTopics.has(t))
    .sort();
  const topics_removed = [...beforeTopics]
    .filter((t) => !afterTopics.has(t))
    .sort();

  const visibility: Visibility | undefined =
    before.visibility !== after.visibility ? after.visibility : undefined;
  const default_branch =
    before.default_branch !== after.default_branch
      ? after.default_branch
      : undefined;
  const issues_enabled =
    before.issues_enabled !== after.issues_enabled
      ? after.issues_enabled
      : undefined;
  const wiki_enabled =
    before.wiki_enabled !== after.wiki_enabled ? after.wiki_enabled : undefined;
  const archive =
    before.archived !== after.archived ? after.archived : undefined;

  return {
    description,
    homepage,
    topics_added,
    topics_removed,
    visibility,
    default_branch,
    issues_enabled,
    wiki_enabled,
    archive,
  };
}

/** `true` when the patch would emit no field edits to the backend. */
export function isPatchEmpty(patch: RemoteRepoConfigPatch): boolean {
  return (
    patch.description === undefined &&
    patch.homepage.kind === "unchanged" &&
    patch.topics_added.length === 0 &&
    patch.topics_removed.length === 0 &&
    patch.visibility === undefined &&
    patch.default_branch === undefined &&
    patch.issues_enabled === undefined &&
    patch.wiki_enabled === undefined &&
    patch.archive === undefined
  );
}

// ───────────────────────────────────────────────────────────────────────────
// Mutation-driven refresh
// ───────────────────────────────────────────────────────────────────────────

/**
 * Re-fetch the remote config for the currently-tracked repo path, if
 * any. A no-op when the dialog has never been opened — there's nothing
 * to refresh and we don't want to trigger a backend call just because
 * a mutation event mentioned `remotes_changed`.
 *
 * Called from the mutation dispatcher when `remotes_changed` flag is
 * set; keeps the dialog's `before`/`current` snapshots in sync with
 * forge-side edits made through other surfaces.
 */
export async function refreshRepoConfig(): Promise<void> {
  const snapshot = get(repoConfigStore);
  const repoPath = snapshot.repoPath;
  if (!repoPath) return;
  try {
    const config = await loadRemoteRepoConfig(repoPath);
    setLoadedConfig(repoPath, config);
  } catch (e) {
    setLoadError(String(e));
  }
}

// ───────────────────────────────────────────────────────────────────────────
// Internal
// ───────────────────────────────────────────────────────────────────────────

/** Deep-enough clone for `RemoteRepoConfig`. Avoids structuredClone for
 * compatibility with older jsdom versions used in vitest. */
function cloneConfig(c: RemoteRepoConfig): RemoteRepoConfig {
  return {
    description: c.description,
    homepage: c.homepage,
    topics: [...c.topics],
    visibility: c.visibility,
    default_branch: c.default_branch,
    issues_enabled: c.issues_enabled,
    wiki_enabled: c.wiki_enabled,
    archived: c.archived,
    branch_protection: c.branch_protection
      ? {
          ...c.branch_protection,
          status_check_contexts: [...c.branch_protection.status_check_contexts],
        }
      : null,
    labels: c.labels.map((l) => ({ ...l })),
  };
}

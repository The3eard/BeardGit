/**
 * Shared loader for `RemoteRepoConfig`.
 *
 * - In-flight dedupe: two concurrent callers for the same `repoPath`
 *   share a single underlying CLI call.
 * - TTL cache (~30 s): a successful fetch is reused for subsequent
 *   callers until the timeout elapses or `invalidate()` runs.
 * - `force: true` bypasses the cache.
 * - Rejections are NOT cached — the next call re-fetches.
 *
 * Lives outside `repoConfig.ts` so consumers (section components +
 * `RepoConfigPage`) can opt in without importing the legacy dialog
 * state.
 */

import { loadRemoteRepoConfig } from "$lib/api/tauri";
import type { RemoteRepoConfig } from "$lib/types/repoConfig";

const TTL_MS = 30_000;

interface CacheEntry {
  config: RemoteRepoConfig;
  storedAt: number;
}

const cache = new Map<string, CacheEntry>();
const inflight = new Map<string, Promise<RemoteRepoConfig>>();

export interface LoadOptions {
  /** Skip the TTL cache and always hit the CLI. */
  force?: boolean;
}

/**
 * Fetch the remote config for `repoPath`. Concurrent calls dedupe.
 */
export async function loadConfig(
  repoPath: string,
  opts: LoadOptions = {},
): Promise<RemoteRepoConfig> {
  if (!opts.force) {
    const hit = cache.get(repoPath);
    if (hit && Date.now() - hit.storedAt < TTL_MS) return hit.config;
  }

  const existing = inflight.get(repoPath);
  if (existing && !opts.force) return existing;

  const promise = loadRemoteRepoConfig(repoPath)
    .then((config) => {
      cache.set(repoPath, { config, storedAt: Date.now() });
      return config;
    })
    .finally(() => {
      // Clear the in-flight slot regardless of success/failure so the
      // next call starts fresh.
      inflight.delete(repoPath);
    });

  inflight.set(repoPath, promise);
  return promise;
}

/** Drop the cached value (if any) for `repoPath`. */
export function invalidate(repoPath: string): void {
  cache.delete(repoPath);
  inflight.delete(repoPath);
}

/** Test-only hook: wipe all state. Not exported from `index.ts`. */
export function __resetForTests(): void {
  cache.clear();
  inflight.clear();
}

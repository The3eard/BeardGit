/**
 * AI provider store — detection, actions, and introspection.
 *
 * Actions return TaskId — output streams to the existing task viewer.
 */

import { writable, derived, get } from "svelte/store";
import * as api from "$lib/api/tauri";
import type { AvailableAiProvider, RepoAiStatus, AiProviderKind } from "$lib/types";

// ─── State ───

export const aiProviders = writable<AvailableAiProvider[]>([]);
export const repoAiStatus = writable<RepoAiStatus[]>([]);
export const preferredAiProvider = writable<AiProviderKind | null>(null);

/**
 * Whether an AI-provider detection pass is currently in progress.
 *
 * Defaults to `true` so the very first paint of `AiSettings` (before
 * `detectAiProviders` has finished its PATH probes) shows a spinner per
 * provider row instead of "Not found" — the `which claude` /
 * `claude --version` subprocesses on a cold cache can take ~1 s.
 * `detectAiProviders` flips this to `false` in its `finally` block.
 */
export const aiProvidersDetecting = writable(true);

/**
 * The AI master switch (Settings → General). Mirrors `AppConfig::ai_enabled`.
 *
 * Optimistically `true` until `loadAiEnabled()` resolves, so the very first
 * paint matches the common case; `+page.svelte` awaits the load before it
 * starts any AI work, so nothing is probed on the strength of the default.
 * Every AI surface derives its visibility from this (directly, or through
 * `hasAiProvider`), and `installAiDisabledReroute` in `navigation.ts`
 * leaves the AI views when it goes false.
 */
export const aiEnabled = writable(true);

/** Whether at least one AI provider is installed — and the switch is on. */
export const hasAiProvider = derived(
  [aiProviders, aiEnabled],
  ([p, enabled]) => enabled && p.length > 0,
);

/** The effective default provider — preferred if available, otherwise first detected. */
export const defaultAiProvider = derived(
  [aiProviders, preferredAiProvider],
  ([providers, preferred]): AiProviderKind | null => {
    if (preferred && providers.some((p) => p.kind === preferred)) {
      return preferred;
    }
    return providers.length > 0 ? providers[0].kind : null;
  },
);

// ─── Detection ───

/**
 * Scan PATH for AI tool binaries and update the store.
 *
 * Flips `aiProvidersDetecting` to `true` for the duration so the Settings
 * page can render a spinner per row while the two IPC calls + their
 * subprocess probes complete. Always clears the flag in the `finally`
 * block so a failure doesn't leave the UI stuck.
 */
export async function detectAiProviders(): Promise<void> {
  // Switch off: nothing to probe. Clear rather than keep stale results so
  // `hasAiProvider`-gated surfaces cannot come back through a cached list.
  if (!get(aiEnabled)) {
    aiProviders.set([]);
    aiProvidersDetecting.set(false);
    return;
  }
  aiProvidersDetecting.set(true);
  try {
    await api.aiRefreshDetection();
    const providers = await api.aiGetProviders();
    aiProviders.set(providers);
  } finally {
    aiProvidersDetecting.set(false);
  }
}

// ─── Master switch ───

/** Load the AI master switch from persisted config. Call before any AI work. */
export async function loadAiEnabled(): Promise<void> {
  try {
    const value = await api.getAiEnabled();
    // Only an explicit `false` turns AI off. Anything else (an unreadable
    // config, or a test harness answering `undefined` for a command it does
    // not mock) keeps the optimistic default rather than hiding AI.
    aiEnabled.set(value !== false);
  } catch {
    // Unreadable config: keep the optimistic default rather than hide AI.
  }
}

/**
 * Persist the AI master switch and apply it: turning it on runs detection so
 * the provider-gated surfaces appear; turning it off drops the detected
 * providers so they disappear. Reverts the store if persisting fails.
 */
export async function setAiEnabled(enabled: boolean): Promise<void> {
  const previous = get(aiEnabled);
  aiEnabled.set(enabled);
  try {
    await api.setAiEnabled(enabled);
  } catch (e) {
    aiEnabled.set(previous);
    throw e;
  }
  if (enabled) {
    await detectAiProviders();
  } else {
    aiProviders.set([]);
  }
}

/** Load the preferred AI provider from persisted config. */
export async function loadPreferredProvider(): Promise<void> {
  const pref = await api.aiGetPreferredProvider();
  preferredAiProvider.set(pref as AiProviderKind | null);
}

/** Set and persist the preferred AI provider. Pass `null` to reset to auto-detect. */
export async function setPreferredProvider(provider: AiProviderKind | null): Promise<void> {
  await api.aiSetPreferredProvider(provider);
  preferredAiProvider.set(provider);
}

/** Refresh AI status for the current repo. */
export async function refreshRepoAiStatus(): Promise<void> {
  try {
    const status = await api.aiGetRepoStatus();
    repoAiStatus.set(status);
  } catch {
    repoAiStatus.set([]);
  }
}

// ─── Headless Actions ───

export async function aiGenerateCommitMessage(provider?: string): Promise<number> {
  const p = provider ?? resolveDefaultProvider();
  return api.aiGenerateCommitMessage(p);
}

export async function aiAnalyzeCode(
  content: string,
  question: string,
  provider?: string,
): Promise<number> {
  const p = provider ?? resolveDefaultProvider();
  return api.aiAnalyzeCode(p, content, question);
}

export async function aiGeneratePrDescription(provider?: string): Promise<number> {
  const p = provider ?? resolveDefaultProvider();
  return api.aiGeneratePrDescription(p);
}

export async function aiReviewCode(diff: string, provider?: string): Promise<number> {
  const p = provider ?? resolveDefaultProvider();
  return api.aiReviewCode(p, diff);
}

export async function aiReviewPr(diff: string, provider?: string): Promise<number> {
  const p = provider ?? resolveDefaultProvider();
  return api.aiReviewPr(p, diff);
}

// ─── Interactive Launch ───

export async function aiLaunchInteractive(provider?: string): Promise<number> {
  const p = provider ?? resolveDefaultProvider();
  return api.aiLaunchInteractive(p);
}

export async function aiLaunchWorktree(
  provider?: string,
  name?: string,
): Promise<number | null> {
  const p = provider ?? resolveDefaultProvider();
  return api.aiLaunchWorktree(p, name);
}

// ─── Introspection (re-export from API) ───

// Thin wrappers rather than `= api.fn` aliases: an alias reads the export
// at module-evaluation time, which throws under a component test that
// mocks `$lib/api/tauri` with only the wrappers it cares about. This store
// is now imported by the shell chrome (Sidebar, StatusBar, TerminalView)
// for `aiEnabled`, so it loads in far more tests than before.
export const aiListWorktrees: typeof api.aiListWorktrees = (...args) =>
  api.aiListWorktrees(...args);
export const aiCleanupWorktree: typeof api.aiCleanupWorktree = (...args) =>
  api.aiCleanupWorktree(...args);
export const aiGetConfigFiles: typeof api.aiGetConfigFiles = (...args) =>
  api.aiGetConfigFiles(...args);

// ─── Helpers ───

function resolveDefaultProvider(): string {
  const providers = get(aiProviders);
  if (providers.length === 0) {
    throw new Error("No AI provider detected");
  }
  const preferred = get(preferredAiProvider);
  if (preferred && providers.some((p) => p.kind === preferred)) {
    return preferred;
  }
  return providers[0].kind;
}

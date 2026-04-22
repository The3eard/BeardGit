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

/** Whether at least one AI provider is installed. */
export const hasAiProvider = derived(aiProviders, (p) => p.length > 0);

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
  aiProvidersDetecting.set(true);
  try {
    await api.aiRefreshDetection();
    const providers = await api.aiGetProviders();
    aiProviders.set(providers);
  } finally {
    aiProvidersDetecting.set(false);
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

export const aiListSessions = api.aiListSessions;
export const aiListWorktrees = api.aiListWorktrees;
export const aiCleanupWorktree = api.aiCleanupWorktree;
export const aiGetConfigFiles = api.aiGetConfigFiles;

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

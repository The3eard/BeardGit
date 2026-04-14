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

/** Whether at least one AI provider is installed. */
export const hasAiProvider = derived(aiProviders, (p) => p.length > 0);

/** The first detected provider kind, used as default. */
export const defaultAiProvider = derived(
  aiProviders,
  (p): AiProviderKind | null => (p.length > 0 ? p[0].kind : null),
);

// ─── Detection ───

/** Scan PATH for AI tool binaries and update store. */
export async function detectAiProviders(): Promise<void> {
  await api.aiRefreshDetection();
  const providers = await api.aiGetProviders();
  aiProviders.set(providers);
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
  return providers[0].kind;
}

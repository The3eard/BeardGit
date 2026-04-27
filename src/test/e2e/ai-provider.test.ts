/**
 * E2E: AI provider workflow
 *
 * Tests detectAiProviders(), refreshRepoAiStatus(), and headless action
 * functions (aiGenerateCommitMessage, aiAnalyzeCode) via the ai store with
 * mocked IPC.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { mockInvokeResponse } from "../setup";
import type { AvailableAiProvider, RepoAiStatus, TaskId } from "$lib/types";

import {
  aiProviders,
  repoAiStatus,
  hasAiProvider,
  defaultAiProvider,
  detectAiProviders,
  refreshRepoAiStatus,
  aiGenerateCommitMessage,
  aiAnalyzeCode,
  aiGeneratePrDescription,
  aiReviewCode,
} from "$lib/stores/ai";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const MOCK_PROVIDERS: AvailableAiProvider[] = [
  { kind: "claude_code", binary_path: "/usr/local/bin/claude", version: "1.2.3" },
  { kind: "codex", binary_path: "/usr/local/bin/codex", version: "0.9.0" },
];

const MOCK_REPO_STATUS: RepoAiStatus[] = [
  { kind: "claude_code", has_config: true, session_count: 2, worktree_count: 1 },
];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("AI provider workflow", () => {
  beforeEach(() => {
    aiProviders.set([]);
    repoAiStatus.set([]);
  });

  // ── Initial derived state ────────────────────────────────────────────

  it("hasAiProvider is false when no providers are loaded", () => {
    expect(get(hasAiProvider)).toBe(false);
  });

  it("defaultAiProvider is null when no providers are loaded", () => {
    expect(get(defaultAiProvider)).toBeNull();
  });

  // ── detectAiProviders ───────────────────────────────────────────────

  it("detectAiProviders calls refresh then get_providers and populates store", async () => {
    mockInvokeResponse("ai_refresh_detection", undefined);
    mockInvokeResponse("ai_get_providers", MOCK_PROVIDERS);

    await detectAiProviders();

    const providers = get(aiProviders);
    expect(providers).toHaveLength(2);
    expect(providers[0].kind).toBe("claude_code");
    expect(providers[0].binary_path).toBe("/usr/local/bin/claude");
    expect(providers[0].version).toBe("1.2.3");
  });

  it("hasAiProvider becomes true after detectAiProviders()", async () => {
    mockInvokeResponse("ai_refresh_detection", undefined);
    mockInvokeResponse("ai_get_providers", MOCK_PROVIDERS);

    await detectAiProviders();

    expect(get(hasAiProvider)).toBe(true);
  });

  it("defaultAiProvider returns first provider kind after detection", async () => {
    mockInvokeResponse("ai_refresh_detection", undefined);
    mockInvokeResponse("ai_get_providers", MOCK_PROVIDERS);

    await detectAiProviders();

    expect(get(defaultAiProvider)).toBe("claude_code");
  });

  it("detectAiProviders clears providers when none found", async () => {
    // Pre-populate to verify it gets cleared
    aiProviders.set(MOCK_PROVIDERS);

    mockInvokeResponse("ai_refresh_detection", undefined);
    mockInvokeResponse("ai_get_providers", []);

    await detectAiProviders();

    expect(get(aiProviders)).toHaveLength(0);
    expect(get(hasAiProvider)).toBe(false);
  });

  // ── refreshRepoAiStatus ──────────────────────────────────────────────

  it("refreshRepoAiStatus populates repoAiStatus store", async () => {
    mockInvokeResponse("ai_get_repo_status", MOCK_REPO_STATUS);

    await refreshRepoAiStatus();

    const status = get(repoAiStatus);
    expect(status).toHaveLength(1);
    expect(status[0].kind).toBe("claude_code");
    expect(status[0].has_config).toBe(true);
    expect(status[0].session_count).toBe(2);
    expect(status[0].worktree_count).toBe(1);
  });

  it("refreshRepoAiStatus sets empty array on error", async () => {
    repoAiStatus.set(MOCK_REPO_STATUS);
    mockInvokeResponse("ai_get_repo_status", () => { throw new Error("backend error"); });

    await refreshRepoAiStatus();

    expect(get(repoAiStatus)).toHaveLength(0);
  });

  // ── headless actions ─────────────────────────────────────────────────

  it("aiGenerateCommitMessage returns a TaskId", async () => {
    aiProviders.set(MOCK_PROVIDERS);
    const TASK_ID: TaskId = 42;
    mockInvokeResponse("ai_generate_commit_message", TASK_ID);

    const taskId = await aiGenerateCommitMessage();

    expect(taskId).toBe(42);
  });

  it("aiGenerateCommitMessage uses explicit provider when supplied", async () => {
    aiProviders.set(MOCK_PROVIDERS);
    const TASK_ID: TaskId = 7;
    mockInvokeResponse("ai_generate_commit_message", TASK_ID);

    const taskId = await aiGenerateCommitMessage("codex");

    expect(taskId).toBe(7);
  });

  it("aiGenerateCommitMessage throws when no provider detected", async () => {
    // aiProviders is empty (cleared in beforeEach)
    await expect(aiGenerateCommitMessage()).rejects.toThrow("No AI provider detected");
  });

  it("aiAnalyzeCode returns a TaskId", async () => {
    aiProviders.set(MOCK_PROVIDERS);
    const TASK_ID: TaskId = 99;
    mockInvokeResponse("ai_analyze_code", TASK_ID);

    const taskId = await aiAnalyzeCode("const x = 1;", "Is this idiomatic?");

    expect(taskId).toBe(99);
  });

  it("aiGeneratePrDescription returns a TaskId", async () => {
    aiProviders.set(MOCK_PROVIDERS);
    const TASK_ID: TaskId = 15;
    mockInvokeResponse("ai_generate_pr_description", TASK_ID);

    const taskId = await aiGeneratePrDescription();

    expect(taskId).toBe(15);
  });

  it("aiReviewCode returns a TaskId", async () => {
    aiProviders.set(MOCK_PROVIDERS);
    const TASK_ID: TaskId = 23;
    mockInvokeResponse("ai_review_code", TASK_ID);

    const taskId = await aiReviewCode("diff --git a/foo.ts ...");

    expect(taskId).toBe(23);
  });

  // ── Provider with only one provider ─────────────────────────────────

  it("defaultAiProvider returns the single provider when only one exists", async () => {
    mockInvokeResponse("ai_refresh_detection", undefined);
    mockInvokeResponse("ai_get_providers", [MOCK_PROVIDERS[0]]);

    await detectAiProviders();

    expect(get(defaultAiProvider)).toBe("claude_code");
    expect(get(aiProviders)).toHaveLength(1);
  });
});

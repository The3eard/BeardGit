/**
 * Shared action helpers for the AI conversations v2 UI.
 *
 * Parallel to `aiSessionActions.ts` but keyed on `AiConversation`
 * (transcript-first) rather than `AiSession` (PID-scan). Covers the two
 * things the v2 UI wants to do:
 *
 * - Focus an already-open BeardGit-owned AI terminal — covers tab,
 *   composite segment, and background-run cases.
 * - Resume an on-disk conversation in a fresh PTY.
 *
 * `resumeConversation` delegates to `resumeAiConversationTab` in
 * `tabs.ts`, which owns the Rust call and the promote/segment/standalone
 * placement rules. Keeping the boolean signature even though every
 * branch currently returns `true` future-proofs the API: Phase 6 adds
 * Codex/OpenCode parity, and if one of those providers grows a "resume
 * not allowed while running" case we already have the shape to surface
 * it.
 *
 * Intentionally does NOT re-export `dismissConversation` — that's a
 * store-level concern and lives in `aiConversations.ts`.
 */

import { get } from "svelte/store";
import { openTabs, activeTabIndex, resumeAiConversationTab } from "./tabs";
import { providerName } from "$lib/data/ai-providers";
import { selectedBackgroundSessionId } from "./aiBackground";
import { selectedConversationId } from "./aiConversations";
import type { AiConversation, Tab } from "$lib/types";
import type { ActiveTerminal } from "./aiActiveTerminals";

/**
 * Focus a BeardGit-owned AI terminal.
 *
 * Behaviour per kind:
 * - `"tab"`     — set `activeTabIndex` to the tab's index.
 * - `"segment"` — set `activeTabIndex` AND rewrite the composite's
 *   `activeSegmentIndex` so the user lands on the terminal rather than
 *   the project segment. Tabs are immutable, so we build a new array —
 *   same pattern as `focusSessionTab` in `aiSessionActions.ts`.
 * - `"bg"`      — set `selectedBackgroundSessionId` so the detail pane
 *   surfaces the run, AND clear `selectedConversationId` because the
 *   detail pane branches on conversation-selection first — without the
 *   clear, the user would click a bg-run row and see the previous
 *   conversation stuck on screen.
 *
 * Returns `true` in every branch today (all three are successful
 * focuses). The boolean signature is there so callers don't special-case
 * — future branches may want to signal "nothing to focus" without
 * throwing.
 */
export function focusTerminal(active: ActiveTerminal): boolean {
  if (active.kind === "tab") {
    activeTabIndex.set(active.tabIndex);
    return true;
  }
  if (active.kind === "segment") {
    const tabs = get(openTabs);
    const composite = tabs[active.tabIndex];
    if (!composite || composite.kind !== "composite") {
      // Defensive: list snapshot is stale (tab was closed between
      // derivation and action). Fall back to just setting the tab index.
      activeTabIndex.set(active.tabIndex);
      return true;
    }
    const next: Tab[] = [...tabs];
    next[active.tabIndex] = {
      ...composite,
      activeSegmentIndex: active.segmentIndex,
    };
    openTabs.set(next);
    activeTabIndex.set(active.tabIndex);
    return true;
  }
  // kind === "bg"
  selectedConversationId.set(null);
  selectedBackgroundSessionId.set(active.session.id);
  return true;
}

/**
 * Resume a conversation in a new terminal tab.
 *
 * Delegates to `resumeAiConversationTab`, which calls
 * `ai_resume_conversation` on the Rust side. Returns `true` when the
 * provider supports `--resume` and a PTY attached; `false` when the
 * provider returned `null` (no resume command wired). Exceptions from
 * the Rust side bubble up unchanged.
 *
 * Note: Claude's `--resume` forks into a new UUID on every call — so the
 * Phase-5 button label is "Resume in new terminal", not "Continue". Each
 * resume is a brand-new conversation that shares the parent's history,
 * which is exactly why the transcript listing can show "resumed from
 * <parent>" breadcrumbs.
 */
export async function resumeConversation(
  conversation: AiConversation,
): Promise<boolean> {
  return resumeAiConversationTab(
    conversation.cwd,
    providerName(conversation.provider),
    conversation.provider,
    conversation.id,
  );
}

/**
 * Active AI terminals — BeardGit-owned processes currently running an AI
 * provider CLI.
 *
 * This store aggregates three disjoint sources:
 * 1. Standalone terminal tabs whose `terminal.provider` is set.
 * 2. Composite tab segments (`LinkedSegment` of type `"terminal"`) whose
 *    `info.provider` is set.
 * 3. Background runs (`aiBackgroundRuns`) whose status is `running` or
 *    `queued`.
 *
 * Unlike `AiConversation` — which is a transcript on disk and may or may
 * not have a live process — every entry here is a PTY or task we spawned
 * and can focus or cancel. The v2 UI surfaces this list alongside the
 * conversation list so the user sees "what's running right now" separate
 * from "what transcripts exist".
 *
 * Per spec: we do NOT dedupe across sources. A bg-run whose PTY has also
 * been adopted as a segment will appear twice, and that's intended — each
 * row is a different affordance.
 */

import { derived, writable } from "svelte/store";
import { openTabs } from "./tabs";
import {
  aiBackgroundRuns,
  selectedBackgroundSessionId,
} from "./aiBackground";
import { selectedConversationId } from "./aiConversations";
import type { TerminalTabInfo, AiSession } from "$lib/types";

/**
 * A BeardGit-owned terminal currently running an AI provider CLI.
 *
 * Discriminated by `kind`:
 * - `"tab"` — standalone terminal tab.
 * - `"segment"` — AI terminal segment inside a composite tab.
 * - `"bg"` — background run (queued or running).
 */
export type ActiveTerminal =
  | { kind: "tab"; tabIndex: number; info: TerminalTabInfo }
  | {
      kind: "segment";
      tabIndex: number;
      segmentIndex: number;
      info: TerminalTabInfo;
    }
  | { kind: "bg"; session: AiSession };

/**
 * All BeardGit-owned AI terminals, in walk order:
 * 1. Tabs (both standalone and composite segments, in tab order).
 * 2. Background runs whose status is `queued` or `running`.
 *
 * Cross-source duplicates are kept — see module-level note.
 */
export const activeAiTerminals = derived(
  [openTabs, aiBackgroundRuns],
  ([$tabs, $runs]) => {
    const out: ActiveTerminal[] = [];

    for (let i = 0; i < $tabs.length; i++) {
      const tab = $tabs[i];
      if (tab.kind === "terminal") {
        if (tab.terminal.provider) {
          out.push({ kind: "tab", tabIndex: i, info: tab.terminal });
        }
        continue;
      }
      if (tab.kind === "composite") {
        for (let s = 0; s < tab.segments.length; s++) {
          const seg = tab.segments[s];
          if (seg.type === "terminal" && seg.info.provider) {
            out.push({
              kind: "segment",
              tabIndex: i,
              segmentIndex: s,
              info: seg.info,
            });
          }
        }
        continue;
      }
      // `project` tabs never contribute — they have no terminal.
    }

    for (const session of $runs.values()) {
      const state = session.background_status?.state;
      if (state === "running" || state === "queued") {
        out.push({ kind: "bg", session });
      }
    }

    return out;
  },
);

/**
 * Convenience count for i18n pluralisation in Phase 5 ("1 AI terminal" vs
 * "3 AI terminals"). Exposed as a helper rather than a derived store so
 * callers can plug it into their own reactive statements without a second
 * subscription.
 */
export function countActiveAiTerminals(list: ActiveTerminal[]): number {
  return list.length;
}

/**
 * Currently selected active terminal (tab / segment / bg), or null.
 *
 * Added in the "AI sessions list trim" slice so tab/segment rows — which
 * previously had no detail branch — gain a selection target used by
 * `AiSessionDetail.svelte`. Mutually exclusive with
 * `selectedConversationId` and `selectedBackgroundSessionId`; use
 * `selectAiSessionRow` to write any of the three so the others clear.
 */
export const selectedActiveTerminal = writable<ActiveTerminal | null>(null);

/**
 * Mutually-exclusive selection across the three AI-session selection
 * stores. Callers set the kind of row they're selecting and this helper
 * writes the right store while clearing the other two in lockstep, so the
 * detail pane never renders stale selection from a previous branch.
 *
 * The discriminant matches the three detail-pane branches in
 * `AiSessionDetail.svelte`:
 * - `"conversation"` → writes `selectedConversationId`
 * - `"background"`   → writes `selectedBackgroundSessionId`
 * - `"active"`       → writes `selectedActiveTerminal`
 */
export type AiSessionSelection =
  | { kind: "conversation"; id: string }
  | { kind: "background"; id: string }
  | { kind: "active"; active: ActiveTerminal };

/**
 * Apply an `AiSessionSelection`, clearing the other two stores.
 *
 * The ordering is deliberate: clear the two other stores BEFORE setting
 * the new one so a subscriber that reacts on the new value never observes
 * more than one non-null selection simultaneously.
 */
export function selectAiSessionRow(selection: AiSessionSelection): void {
  if (selection.kind === "conversation") {
    selectedBackgroundSessionId.set(null);
    selectedActiveTerminal.set(null);
    selectedConversationId.set(selection.id);
    return;
  }
  if (selection.kind === "background") {
    selectedConversationId.set(null);
    selectedActiveTerminal.set(null);
    selectedBackgroundSessionId.set(selection.id);
    return;
  }
  // kind === "active"
  selectedConversationId.set(null);
  selectedBackgroundSessionId.set(null);
  selectedActiveTerminal.set(selection.active);
}

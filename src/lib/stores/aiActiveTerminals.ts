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

import { derived } from "svelte/store";
import { openTabs } from "./tabs";
import { aiBackgroundRuns } from "./aiBackground";
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

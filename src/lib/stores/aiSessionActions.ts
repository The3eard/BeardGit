/**
 * Shared action helpers for AI Sessions UI.
 *
 * Both `AiSessionList.svelte` and `AiSessionDetail.svelte` need to drive the
 * exact same behaviour for the "focus an already-open terminal", "resume an
 * external session in a new PTY", and "decide which tier of button to show"
 * flows. Keeping a single implementation in this module eliminates the
 * accidental drift that produced the Phase-10 composite-focus bug (the list
 * had a dangling `// TODO: set activeSegmentIndex` comment and the detail
 * pane didn't offer the action at all).
 *
 * The helpers are deliberately pure-ish: `getSessionTier` is a read, the
 * other two mutate the `openTabs` / `activeTabIndex` stores and rely on
 * `resumeAiSessionTab` (in `tabs.ts`) for the PTY spawn half of "Open
 * Terminal".
 */

import { get } from "svelte/store";
import { openTabs, activeTabIndex, resumeAiSessionTab } from "./tabs";
import { providerName } from "$lib/data/ai-providers";
import type { AiSession, Tab, TerminalTabInfo } from "$lib/types";

/**
 * Result of `getSessionTier`.
 *
 * - `"focus"` — an existing tab already hosts this session's terminal.
 * - `"resume"` — the session is still active but no tab hosts it yet.
 * - `"ended"` — the session is no longer active; no spawn/focus action.
 */
export type SessionTier = "focus" | "resume" | "ended";

/**
 * Decide which action button tier applies to `session`.
 *
 * An optional `tabs` array lets callers pass a snapshot (avoids re-reading
 * the store on every iteration when the caller already has a `$derived`
 * handle). Defaults to `get(openTabs)` for the common Svelte 5 case.
 */
export function getSessionTier(
  session: AiSession,
  tabs: Tab[] = get(openTabs),
): SessionTier {
  if (!session.is_active) return "ended";
  for (const tab of tabs) {
    if (tab.kind === "terminal" && matchesSession(tab.terminal, session)) {
      return "focus";
    }
    if (tab.kind === "composite") {
      for (const seg of tab.segments) {
        if (seg.type === "terminal" && matchesSession(seg.info, session)) {
          return "focus";
        }
      }
    }
  }
  return "resume";
}

/**
 * Switch to the tab (and, for composite tabs, the segment) that hosts
 * `session`'s terminal. Returns `true` on a match, `false` when no tab
 * owns the session.
 *
 * The composite-match branch rewrites the tab's `activeSegmentIndex` so
 * the user lands on the terminal rather than the project segment — this
 * is the Phase-10 bug from the spec.
 */
export function focusSessionTab(session: AiSession): boolean {
  const tabs = get(openTabs);
  for (let i = 0; i < tabs.length; i++) {
    const tab = tabs[i];
    if (tab.kind === "terminal" && matchesSession(tab.terminal, session)) {
      activeTabIndex.set(i);
      return true;
    }
    if (tab.kind === "composite") {
      for (let s = 0; s < tab.segments.length; s++) {
        const seg = tab.segments[s];
        if (seg.type === "terminal" && matchesSession(seg.info, session)) {
          const next = [...tabs];
          next[i] = { ...tab, activeSegmentIndex: s };
          openTabs.set(next);
          activeTabIndex.set(i);
          return true;
        }
      }
    }
  }
  return false;
}

/**
 * Adapter over `resumeAiSessionTab` so both the list row and the detail
 * pane call one helper. Resolves to `true` when the provider supports
 * `--resume` and a PTY was attached to a tab; `false` when the provider
 * has no resume command. Exceptions from the Rust side bubble up.
 */
export async function resumeSession(session: AiSession): Promise<boolean> {
  return resumeAiSessionTab(
    session.cwd,
    providerName(session.provider),
    session.provider,
    session.id,
  );
}

/**
 * Compare a tab's terminal metadata with a session by `(provider, cwd)`.
 * Trailing-slash normalisation prevents false negatives when one side
 * came from the filesystem and the other from a spawn request.
 */
function matchesSession(info: TerminalTabInfo, session: AiSession): boolean {
  return (
    info.provider === session.provider &&
    info.cwd.replace(/\/+$/, "") === session.cwd.replace(/\/+$/, "")
  );
}

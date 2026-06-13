/**
 * Provider store — multi-provider CI integration (GitLab + GitHub).
 *
 * Manages provider connections, CI run listing with server-side filtering,
 * run detail with stage/job expansion, and live job log streaming.
 *
 * Polling intervals:
 * - CI run list: 15 s
 * - CI run detail: 10 s (auto-stops when status is terminal)
 * - Job log: 3 s (auto-stops when job status is terminal)
 */

import { writable, derived, get } from "svelte/store";
import type { ProviderStatusResponse, CiRun, CiRunDetail, ProviderKind } from "../types";
import * as api from "../api/tauri";
// Import from `tabs` (and `repoConfig`) directly — `projects.ts` itself
// imports from this module, so re-importing `activeProject` through
// `./projects` would form a cycle that leaves the `derived` constructor
// below observing `undefined` at module-init time (see Phase 9).
import { activeProjectFromTab } from "./tabs";
import { repoConfig } from "./repoConfig";
import { remotes } from "./remotes";

/** Full provider connection status (all providers + active index). */
export const providerStatus = writable<ProviderStatusResponse>({ providers: [], active_index: null });

export const activeProvider = derived(providerStatus, ($s) =>
  $s.active_index !== null ? $s.providers[$s.active_index] ?? null : null
);
export const isConnected = derived(providerStatus, ($s) => $s.providers.length > 0);
export const hasActiveProvider = derived(providerStatus, ($s) => $s.active_index !== null);
export const ciRuns = writable<CiRun[]>([]);
export const hasMoreCiRuns = writable(false);
const PAGE_SIZE = 20;
export const selectedCiRunId = writable<number | null>(null);
export const selectedCiRun = writable<CiRunDetail | null>(null);
export const loadingDetail = writable(false);
export const jobLog = writable<string | null>(null);
export const loadingJobLog = writable(false);
export const jobLogUnavailable = writable(false);
export const selectedJobId = writable<number | null>(null);
export const selectedJobSteps = derived(
  [selectedCiRun, selectedJobId],
  ([$run, $jobId]) => {
    if (!$run || !$jobId) return null;
    for (const stage of $run.stages) {
      for (const job of stage.jobs) {
        if (job.id === $jobId) return job.steps ?? null;
      }
    }
    return null;
  }
);
export const isConnecting = writable(false);
export const providerError = writable<string | null>(null);

// ---------------------------------------------------------------------------
// CI Polling
// ---------------------------------------------------------------------------
//
// Three independent polling loops (list / run-detail / job-log) share the
// same lightweight scheduler so we can:
//
// 1. **Pause when the window is hidden** — laptops on battery shouldn't
//    keep firing `git diff` / forge HTTP calls every 3 s while tucked away
//    in another desktop. We listen to `visibilitychange` once at module
//    init and resume each active loop when the page becomes visible again
//    (firing one tick immediately so the user doesn't see stale data).
//
// 2. **Skip overlapping ticks** — if a previous fetch is still in flight,
//    the next tick is dropped instead of piling up concurrent requests.
//    Helps when the provider API is slow (cold cache, GH search rate limit)
//    so we never have N>1 in-flight calls of the same kind.
//
// We deliberately keep three separate `setInterval`s rather than coalescing
// into a single bucket loop: the cadences (15 s / 10 s / 3 s) are
// independent and only the active panel's loop is typically running, so
// the wake-up cost is negligible compared to the readability win.

const noop = async () => {};

interface PollLoop {
  id: ReturnType<typeof setInterval> | null;
  intervalMs: number;
  tick: () => Promise<void>;
  inFlight: boolean;
}

const ciRunListLoop: PollLoop = { id: null, intervalMs: 15_000, tick: noop, inFlight: false };
const ciRunDetailLoop: PollLoop = { id: null, intervalMs: 10_000, tick: noop, inFlight: false };
const jobLogLoop: PollLoop = { id: null, intervalMs: 3_000, tick: noop, inFlight: false };

function startLoop(loop: PollLoop) {
  stopLoop(loop);
  if (typeof document !== "undefined" && document.hidden) {
    // Will be resumed by the visibilitychange listener.
    return;
  }
  loop.id = setInterval(() => {
    if (loop.inFlight) return;
    if (typeof document !== "undefined" && document.hidden) return;
    loop.inFlight = true;
    loop.tick().catch(() => { /* swallow polling errors */ }).finally(() => {
      loop.inFlight = false;
    });
  }, loop.intervalMs);
}

function stopLoop(loop: PollLoop) {
  if (loop.id !== null) {
    clearInterval(loop.id);
    loop.id = null;
  }
}

if (typeof document !== "undefined") {
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) {
      stopLoop(ciRunListLoop);
      stopLoop(ciRunDetailLoop);
      stopLoop(jobLogLoop);
    } else {
      // Resume + fire one immediate tick so the UI catches up after a
      // long hidden interval. Loops with `tick === noop` were stopped
      // explicitly and stay paused.
      for (const loop of [ciRunListLoop, ciRunDetailLoop, jobLogLoop]) {
        if (loop.tick !== noop) {
          startLoop(loop);
          if (!loop.inFlight) {
            loop.inFlight = true;
            loop.tick().catch(() => {}).finally(() => { loop.inFlight = false; });
          }
        }
      }
    }
  });
}

export function startCiRunListPolling(refreshFn?: () => Promise<void>) {
  ciRunListLoop.tick = refreshFn ?? (async () => { await loadCiRuns(); });
  startLoop(ciRunListLoop);
}

export function stopCiRunListPolling() {
  ciRunListLoop.tick = noop;
  stopLoop(ciRunListLoop);
}

export function startCiRunDetailPolling(runId: number) {
  ciRunDetailLoop.tick = async () => {
    const detail = await api.getCiRunDetail(runId);
    selectedCiRun.set(detail);
    const status = detail.run.status;
    if (status !== 'running' && status !== 'pending' && status !== 'queued') {
      stopCiRunDetailPolling();
    }
  };
  startLoop(ciRunDetailLoop);
}

export function stopCiRunDetailPolling() {
  ciRunDetailLoop.tick = noop;
  stopLoop(ciRunDetailLoop);
}

export function startJobLogPolling(jobId: number) {
  jobLogLoop.tick = async () => {
    const log = await api.getJobLog(jobId);
    jobLog.set(log);
    jobLogUnavailable.set(false);
  };
  startLoop(jobLogLoop);
}

export function stopJobLogPolling() {
  jobLogLoop.tick = noop;
  stopLoop(jobLogLoop);
}

export function stopAllPolling() {
  stopCiRunListPolling();
  stopCiRunDetailPolling();
  stopJobLogPolling();
}

export async function checkStatus() {
  const status = await api.getProviderStatus();
  providerStatus.set(status);
}

export async function tryAutoConnect() {
  try {
    await api.tryAutoConnect();
    await checkStatus();
  } catch { /* ignore auto-connect failures */ }
}

export async function connect(kind: ProviderKind, instanceUrl: string, token: string) {
  isConnecting.set(true);
  providerError.set(null);
  try {
    await api.connectProvider(kind, instanceUrl, token);
    await checkStatus();
  } catch (e) {
    providerError.set(String(e));
    throw e;
  } finally {
    isConnecting.set(false);
  }
}

export async function disconnect(instanceUrl: string) {
  await api.disconnectProvider(instanceUrl);
  await checkStatus();
  ciRuns.set([]);
  selectedCiRun.set(null);
}

export async function loadCiRuns(branch?: string, source?: string, status?: string) {
  currentPage = 1;
  const list = await api.listCiRuns(branch, source, status, PAGE_SIZE);
  ciRuns.set(list);
  hasMoreCiRuns.set(list.length >= PAGE_SIZE);
}

let currentPage = 1;

export async function loadMoreCiRuns(branch?: string, source?: string, status?: string) {
  currentPage++;
  const list = await api.listCiRuns(branch, source, status, PAGE_SIZE, currentPage);
  if (list.length > 0) {
    const current = get(ciRuns);
    ciRuns.set([...current, ...list]);
    hasMoreCiRuns.set(list.length >= PAGE_SIZE);
  } else {
    hasMoreCiRuns.set(false);
  }
}

export async function loadCiRunDetail(runId: number) {
  selectedCiRunId.set(runId);
  selectedCiRun.set(null);
  loadingDetail.set(true);
  stopCiRunDetailPolling();
  stopJobLogPolling();
  jobLog.set(null);
  jobLogUnavailable.set(false);
  selectedJobId.set(null);
  try {
    const detail = await api.getCiRunDetail(runId);
    if (get(selectedCiRunId) !== runId) return;
    selectedCiRun.set(detail);
    if (['running', 'pending', 'queued'].includes(detail.run.status)) {
      startCiRunDetailPolling(runId);
    }
  } finally {
    if (get(selectedCiRunId) === runId) {
      loadingDetail.set(false);
    }
  }
}

export async function loadJobLog(jobId: number, jobStatus?: string) {
  stopJobLogPolling();
  loadingJobLog.set(true);
  jobLog.set(null);
  jobLogUnavailable.set(false);
  selectedJobId.set(jobId);
  try {
    const log = await api.getJobLog(jobId);
    // Staleness guard: a rapid click on another job may have superseded this
    // request while awaiting. Don't clobber the newer selection's log.
    if (get(selectedJobId) !== jobId) return;
    jobLog.set(log);
  } catch {
    // GitHub API returns error for running jobs — logs not yet available
    if (get(selectedJobId) !== jobId) return;
    jobLog.set(null);
    jobLogUnavailable.set(true);
  } finally {
    if (get(selectedJobId) === jobId) {
      loadingJobLog.set(false);
    }
  }
  if (jobStatus === 'running' || jobStatus === 'queued' || jobStatus === 'pending') {
    startJobLogPolling(jobId);
  }
}

// ---------------------------------------------------------------------------
// CI/CD control actions (Phase 8.4)
// ---------------------------------------------------------------------------

/**
 * Trigger a new CI run. On success, refreshes the run list so the new run
 * appears near the top (if it has already been registered server-side).
 */
export async function triggerWorkflow(
  workflowId: string,
  gitRef: string,
  inputs: Record<string, string>,
): Promise<void> {
  await api.triggerWorkflow(workflowId, gitRef, inputs);
  try { await loadCiRuns(); } catch { /* ignore */ }
}

/** Retry all jobs of a completed run; refresh detail after. */
export async function retryCiRun(runId: number | string): Promise<void> {
  await api.retryCiRun(String(runId));
  const current = get(selectedCiRunId);
  if (current != null) {
    try { await loadCiRunDetail(current); } catch { /* ignore */ }
  }
}

/** Retry only the failed jobs of a completed run; refresh detail after. */
export async function retryCiFailedJobs(runId: number | string): Promise<void> {
  await api.retryCiFailedJobs(String(runId));
  const current = get(selectedCiRunId);
  if (current != null) {
    try { await loadCiRunDetail(current); } catch { /* ignore */ }
  }
}

/** Retry a specific failed job; refresh detail after. */
export async function retryCiJob(jobId: number | string): Promise<void> {
  await api.retryCiJob(String(jobId));
  const current = get(selectedCiRunId);
  if (current != null) {
    try { await loadCiRunDetail(current); } catch { /* ignore */ }
  }
}

/** Cancel an in-progress run; refresh detail after. */
export async function cancelCiRun(runId: number | string): Promise<void> {
  await api.cancelCiRun(String(runId));
  const current = get(selectedCiRunId);
  if (current != null) {
    try { await loadCiRunDetail(current); } catch { /* ignore */ }
  }
}

/** Fetch the list of workflow definitions for the current project. */
export async function listCiWorkflows(): Promise<import("../types").Workflow[]> {
  return api.listCiWorkflows();
}

// ---------------------------------------------------------------------------
// Per-project provider selection (Phase 9 — statusbar forge filter)
// ---------------------------------------------------------------------------

/**
 * Resolved provider selection for the active project, or `null` when
 * the project has no associated forge.
 *
 * Used by `ForgeSlot.svelte` to render exactly one pill (GitHub or
 * GitLab) — never both — so the statusbar reflects the current
 * project's provider rather than the union of all connected providers.
 *
 * Derivation rules, applied in order:
 *   1. Explicit `repoConfig.provider === "github"` → GitHub.
 *   2. Explicit `repoConfig.provider === "gitlab"` → GitLab.
 *   3. `activeProject.remotes[origin]` URL heuristic:
 *        - `github.com` anywhere in the URL → GitHub
 *        - `gitlab.com` or any `gitlab.<domain>` host → GitLab
 *   4. Otherwise → `null` (render nothing).
 *
 * The resolved kind is then matched against `providerStatus.providers`
 * and the corresponding `ConnectedProvider` is attached. If the user's
 * project points at a provider they haven't authed yet, the store is
 * `null` — rendering nothing is better than a dead pill.
 *
 * Custom-domain self-hosted GitLab (e.g. `code.example.org`) won't
 * match the heuristic — those users must set `repoConfig.provider`
 * explicitly via the repo-settings dialog.
 */
export const projectProvider = derived(
  [activeProjectFromTab, providerStatus, repoConfig, remotes],
  ([$project, $status, $config, $remotes]) => {
    if (!$project) return null;

    const pickByKind = (kind: "github" | "gitlab") => {
      const p = $status.providers.find((x) => x.kind === kind);
      return p ? { kind, provider: p } : null;
    };

    if ($config?.provider === "github") return pickByKind("github");
    if ($config?.provider === "gitlab") return pickByKind("gitlab");

    // ProjectInfo doesn't actually carry remotes today — the cast is a
    // legacy shape. Fall back to the live remotes store (refreshed on
    // project activation + remotes_changed mutations) so the heuristic
    // works without an explicit repoConfig.provider override.
    const origin =
      (
        $project as { remotes?: Array<{ name: string; url: string }> }
      ).remotes?.find((r) => r.name === "origin")?.url ??
      ($remotes ?? []).find((r) => r.name === "origin")?.url;
    if (origin) {
      if (/github\.com[:/]/.test(origin)) return pickByKind("github");
      if (/gitlab\.(com|[\w.-]+)[:/]/.test(origin)) return pickByKind("gitlab");
    }
    return null;
  },
);

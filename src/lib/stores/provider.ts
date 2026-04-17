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

let ciRunListTimer: ReturnType<typeof setInterval> | null = null;
let ciRunDetailTimer: ReturnType<typeof setInterval> | null = null;
let jobLogTimer: ReturnType<typeof setInterval> | null = null;

export function startCiRunListPolling(refreshFn?: () => Promise<void>) {
  stopCiRunListPolling();
  const fn = refreshFn ?? (async () => { await loadCiRuns(); });
  ciRunListTimer = setInterval(async () => {
    try {
      await fn();
    } catch { /* ignore polling errors */ }
  }, 15000);
}

export function stopCiRunListPolling() {
  if (ciRunListTimer) {
    clearInterval(ciRunListTimer);
    ciRunListTimer = null;
  }
}

export function startCiRunDetailPolling(runId: number) {
  stopCiRunDetailPolling();
  ciRunDetailTimer = setInterval(async () => {
    try {
      const detail = await api.getCiRunDetail(runId);
      selectedCiRun.set(detail);
      const status = detail.run.status;
      if (status !== 'running' && status !== 'pending' && status !== 'queued') {
        stopCiRunDetailPolling();
      }
    } catch { /* ignore */ }
  }, 10000);
}

export function stopCiRunDetailPolling() {
  if (ciRunDetailTimer) {
    clearInterval(ciRunDetailTimer);
    ciRunDetailTimer = null;
  }
}

export function startJobLogPolling(jobId: number) {
  stopJobLogPolling();
  jobLogTimer = setInterval(async () => {
    try {
      const log = await api.getJobLog(jobId);
      jobLog.set(log);
      jobLogUnavailable.set(false);
    } catch { /* ignore — will retry next tick */ }
  }, 3000);
}

export function stopJobLogPolling() {
  if (jobLogTimer) {
    clearInterval(jobLogTimer);
    jobLogTimer = null;
  }
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
    jobLog.set(log);
  } catch {
    // GitHub API returns error for running jobs — logs not yet available
    jobLog.set(null);
    jobLogUnavailable.set(true);
  } finally {
    loadingJobLog.set(false);
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

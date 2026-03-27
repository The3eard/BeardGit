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
    } catch { /* ignore */ }
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
  try {
    const log = await api.getJobLog(jobId);
    jobLog.set(log);
    if (jobStatus === 'running' || jobStatus === 'queued' || jobStatus === 'pending') {
      startJobLogPolling(jobId);
    }
  } finally {
    loadingJobLog.set(false);
  }
}

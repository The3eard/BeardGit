/**
 * Tauri IPC wrappers — single source of truth for all Rust backend calls.
 *
 * Every function maps 1:1 to a `#[tauri::command]` in `app-core/src/commands.rs`.
 * Parameter names use camelCase (Tauri auto-converts to snake_case on the Rust side).
 * Return types match the corresponding Rust structs in `src/lib/types/index.ts`.
 *
 * Organized by feature domain:
 * - Repository & graph
 * - Staging & commits
 * - Branches
 * - Stash
 * - Tags
 * - Conflict detection
 * - Remote operations (fetch/pull/push)
 * - Provider auth
 * - CI runs & job logs
 * - Locale
 * - Background tasks
 * - Multi-project tabs
 */

import { invoke } from "@tauri-apps/api/core";
import type { RepoInfo, GraphViewport, CommitInfo, CommitFileChange, BranchInfo, FileStatus, FileDiff, ProviderUser, ProviderStatusResponse, CiRun, CiRunDetail, TaskInfo, TaskId, TaskOutputLine, ProjectInfo, RecentRepo, RemoteInfo, StatusSummary, StashEntry, TagInfo, CommitStats, ConflictStatus, ThemeMeta, ThemeData, WorktreeInfo, HunkSelection, BlameLine, FileHistoryEntry } from "../types";

export async function openRepo(path: string): Promise<RepoInfo> {
  return invoke<RepoInfo>("open_repo", { path });
}

export async function getGraphViewport(offset: number, limit: number): Promise<GraphViewport> {
  return invoke<GraphViewport>("get_graph_viewport", { offset, limit });
}

export async function searchCommits(
  branch?: string, author?: string, message?: string, sha?: string, maxCount?: number
): Promise<GraphViewport> {
  return invoke<GraphViewport>("search_commits", {
    branch: branch ?? null, author: author ?? null,
    message: message ?? null, sha: sha ?? null,
    maxCount: maxCount ?? null,
  });
}

export async function getCommitDetail(oid: string): Promise<CommitInfo> {
  return invoke<CommitInfo>("get_commit_detail", { oid });
}

export async function getCommitRow(oid: string): Promise<number | null> {
  return invoke<number | null>("get_commit_row", { oid });
}

export async function getStatusSummary(): Promise<StatusSummary> {
  return invoke<StatusSummary>("get_status_summary");
}

export async function getCommitFiles(oid: string): Promise<CommitFileChange[]> {
  return invoke<CommitFileChange[]>("get_commit_files", { oid });
}

export async function getDiffBetweenCommits(fromOid: string, toOid: string): Promise<CommitFileChange[]> {
  return invoke<CommitFileChange[]>("get_diff_between_commits", { fromOid, toOid });
}

export async function getCommitFileDiff(oid: string, path: string): Promise<FileDiff[]> {
  return invoke<FileDiff[]>("get_commit_file_diff", { oid, path });
}

export async function getBranches(): Promise<BranchInfo[]> {
  return invoke<BranchInfo[]>("get_branches");
}

export async function getBranchCommits(branchName: string, limit: number): Promise<CommitInfo[]> {
  return invoke<CommitInfo[]>("get_branch_commits", { branchName, limit });
}

export async function getFileStatuses(): Promise<FileStatus[]> {
  return invoke<FileStatus[]>("get_file_statuses");
}

export async function stageFiles(paths: string[]): Promise<void> {
  return invoke("stage_files", { paths });
}

export async function unstageFiles(paths: string[]): Promise<void> {
  return invoke("unstage_files", { paths });
}

export async function stageAll(): Promise<void> {
  return invoke("stage_all");
}

export async function unstageAll(): Promise<void> {
  return invoke("unstage_all");
}

/** Stage selected hunks or individual lines from the working directory. */
export async function stageHunks(path: string, selections: HunkSelection[]): Promise<void> {
  return invoke<void>("stage_hunks", { path, selections });
}

/** Unstage selected hunks or individual lines from the index. */
export async function unstageHunks(path: string, selections: HunkSelection[]): Promise<void> {
  return invoke<void>("unstage_hunks", { path, selections });
}

/** Discard selected hunks or individual lines from the working directory. */
export async function discardHunks(path: string, selections: HunkSelection[]): Promise<void> {
  return invoke<void>("discard_hunks", { path, selections });
}

export async function createCommit(message: string, name: string, email: string): Promise<string> {
  return invoke<string>("create_commit", { message, name, email });
}

export async function createBranch(name: string): Promise<void> {
  return invoke("create_branch", { name });
}

export async function deleteBranch(name: string): Promise<void> {
  return invoke("delete_branch", { name });
}

export async function checkoutBranch(name: string): Promise<void> {
  return invoke("checkout_branch", { name });
}

export async function getDiffWorkdir(): Promise<FileDiff[]> {
  return invoke<FileDiff[]>("get_diff_workdir");
}

export async function getDiffIndex(): Promise<FileDiff[]> {
  return invoke<FileDiff[]>("get_diff_index");
}

export async function mergeBranch(branch: string): Promise<string> {
  return invoke<string>("merge_branch", { branch });
}

export async function cherryPick(oid: string): Promise<string> {
  return invoke<string>("cherry_pick", { oid });
}

export async function revertCommit(oid: string): Promise<string> {
  return invoke<string>("revert_commit", { oid });
}

export async function resetToCommit(oid: string, mode: string): Promise<void> {
  return invoke<void>("reset_to_commit", { oid, mode });
}

export async function amendCommit(message: string): Promise<void> {
  return invoke<void>("amend_commit", { message });
}

export async function getHeadMessage(): Promise<string> {
  return invoke<string>("get_head_message");
}

export async function stashPush(message: string | null): Promise<string> {
  return invoke<string>("stash_push", { message });
}

export async function stashPop(index: number | null): Promise<string> {
  return invoke<string>("stash_pop", { index });
}

export async function stashList(): Promise<string[]> {
  return invoke<string[]>("stash_list");
}

export async function stashApply(index: number | null): Promise<string> {
  return invoke<string>("stash_apply", { index });
}

export async function stashApplyFile(index: number, path: string): Promise<string> {
  return invoke<string>("stash_apply_file", { index, path });
}

export async function stashDrop(index: number | null): Promise<string> {
  return invoke<string>("stash_drop", { index });
}

export async function stashEntries(): Promise<StashEntry[]> {
  return invoke<StashEntry[]>("stash_entries");
}

export async function stashShowParsed(index: number | null): Promise<FileDiff[]> {
  return invoke<FileDiff[]>("stash_show_parsed", { index });
}

// ---------------------------------------------------------------------------
// Tags
// ---------------------------------------------------------------------------

export async function listTags(): Promise<TagInfo[]> {
  return invoke<TagInfo[]>("list_tags");
}

export async function createTag(name: string, target: string, message: string | null): Promise<void> {
  return invoke("create_tag", { name, target, message });
}

export async function deleteTag(name: string): Promise<void> {
  return invoke("delete_tag", { name });
}

export async function pushTag(tagName: string | null, remote: string): Promise<number> {
  return invoke<number>("push_tag", { tagName, remote });
}

export async function getCommitStats(oid: string): Promise<CommitStats> {
  return invoke<CommitStats>("get_commit_stats", { oid });
}

export async function listTagsPaginated(perPage: number, page: number): Promise<TagInfo[]> {
  return invoke<TagInfo[]>("list_tags_paginated", { perPage, page });
}

export async function searchTags(query: string): Promise<TagInfo[]> {
  return invoke<TagInfo[]>("search_tags", { query });
}

// ---------------------------------------------------------------------------
// Conflict detection
// ---------------------------------------------------------------------------

export async function getConflictStatus(): Promise<ConflictStatus> {
  return invoke<ConflictStatus>("get_conflict_status");
}

export async function abortOperation(): Promise<string> {
  return invoke<string>("abort_operation");
}

export async function continueOperation(): Promise<string> {
  return invoke<string>("continue_operation");
}

// ---------------------------------------------------------------------------
// Remote operations
// ---------------------------------------------------------------------------

export async function fetchRemote(remote: string): Promise<number> {
  return invoke<number>("fetch_remote", { remote });
}

export async function pullRemote(remote: string, branch: string): Promise<number> {
  return invoke<number>("pull_remote", { remote, branch });
}

export async function pushRemote(remote: string, branch: string): Promise<number> {
  return invoke<number>("push_remote", { remote, branch });
}

export async function getRemotes(): Promise<RemoteInfo[]> {
  return invoke<RemoteInfo[]>("get_remotes");
}

export async function renameRemote(oldName: string, newName: string): Promise<void> {
  return invoke<void>("rename_remote", { oldName, newName });
}

export async function removeRemote(name: string): Promise<void> {
  return invoke<void>("remove_remote", { name });
}

// ---------------------------------------------------------------------------
// Provider auth
// ---------------------------------------------------------------------------

export async function connectProvider(kind: string, instanceUrl: string, token: string): Promise<ProviderUser> {
  return invoke<ProviderUser>("connect_provider", { kind, instanceUrl, token });
}

export async function disconnectProvider(instanceUrl: string): Promise<void> {
  return invoke<void>("disconnect_provider", { instanceUrl });
}

export async function getProviderStatus(): Promise<ProviderStatusResponse> {
  return invoke<ProviderStatusResponse>("get_provider_status");
}

export async function tryAutoConnect(): Promise<ProviderUser[]> {
  return invoke<ProviderUser[]>("try_auto_connect");
}

// ---------------------------------------------------------------------------
// CI runs
// ---------------------------------------------------------------------------

export async function listCiRuns(
  branch?: string,
  source?: string,
  status?: string,
  perPage?: number,
  page?: number,
): Promise<CiRun[]> {
  return invoke<CiRun[]>("list_ci_runs", { branch, source, status, perPage, page });
}

export async function getCiRunDetail(runId: number): Promise<CiRunDetail> {
  return invoke<CiRunDetail>("get_ci_run_detail", { runId });
}

export async function getJobLog(jobId: number): Promise<string> {
  return invoke<string>("get_job_log", { jobId });
}

export async function detectProject(): Promise<void> {
  return invoke<void>("detect_project");
}

export async function preprocessJobLog(rawText: string, providerKind: string): Promise<string> {
  return invoke<string>("preprocess_job_log", { rawText, providerKind });
}

// ---------------------------------------------------------------------------
// Locale
// ---------------------------------------------------------------------------

export async function getLocale(): Promise<string> {
  return invoke<string>("get_locale");
}

export async function setLocaleConfig(locale: string): Promise<void> {
  return invoke<void>("set_locale", { locale });
}

export async function getUserIdentities(): Promise<string[]> {
  return invoke<string[]>("get_user_identities");
}

// ---------------------------------------------------------------------------
// Background tasks
// ---------------------------------------------------------------------------

export async function getTasks(): Promise<TaskInfo[]> {
  return invoke<TaskInfo[]>("get_tasks");
}

export async function getTaskOutput(taskId: TaskId): Promise<TaskOutputLine[]> {
  return invoke<TaskOutputLine[]>("get_task_output", { taskId });
}

export async function cancelTask(taskId: TaskId): Promise<void> {
  return invoke<void>("cancel_task", { taskId });
}

// ---------------------------------------------------------------------------
// Multi-project tabs
// ---------------------------------------------------------------------------

export async function openProject(path: string): Promise<ProjectInfo> {
  return invoke<ProjectInfo>("open_project", { path });
}

export async function closeProject(index: number): Promise<void> {
  return invoke<void>("close_project", { index });
}

export async function switchProject(index: number): Promise<RepoInfo> {
  return invoke<RepoInfo>("switch_project", { index });
}

export async function getOpenProjects(): Promise<ProjectInfo[]> {
  return invoke<ProjectInfo[]>("get_open_projects");
}

export async function getActiveProjectIndex(): Promise<number | null> {
  return invoke<number | null>("get_active_project_index");
}

export async function getRecentRepos(): Promise<RecentRepo[]> {
  return invoke<RecentRepo[]>("get_recent_repos");
}

export async function restoreProjects(): Promise<ProjectInfo[]> {
  return invoke<ProjectInfo[]>("restore_projects");
}

// ---------------------------------------------------------------------------
// Theme
// ---------------------------------------------------------------------------

export async function listThemes(): Promise<ThemeMeta[]> {
  return invoke<ThemeMeta[]>("list_themes");
}

export async function getTheme(name: string): Promise<ThemeData> {
  return invoke<ThemeData>("get_theme", { name });
}

export async function setTheme(name: string): Promise<void> {
  return invoke<void>("set_theme", { name });
}

export async function getThemeAuto(): Promise<boolean> {
  return invoke<boolean>("get_theme_auto");
}

export async function setThemeAuto(enabled: boolean): Promise<void> {
  return invoke<void>("set_theme_auto", { enabled });
}

export async function getUiScale(): Promise<number> {
  return invoke<number>("get_ui_scale");
}

export async function setUiScale(scale: number): Promise<void> {
  return invoke<void>("set_ui_scale", { scale });
}

// ---------------------------------------------------------------------------
// Raw file content (for CodeMirror diff views)
// ---------------------------------------------------------------------------

/** Returns raw file content at a specific commit. */
export async function getFileAtCommit(oid: string, path: string): Promise<string> {
  return invoke<string>("get_file_at_commit", { oid, path });
}

/** Returns raw file content from the working directory. */
export async function getFileWorkdir(path: string): Promise<string> {
  return invoke<string>("get_file_workdir", { path });
}

/** Returns raw file content from the index (staged version). */
export async function getFileIndex(path: string): Promise<string> {
  return invoke<string>("get_file_index", { path });
}

// ---------------------------------------------------------------------------
// Worktrees
// ---------------------------------------------------------------------------

/** List all worktrees for the active repository, including the main worktree. */
export async function listWorktrees(): Promise<WorktreeInfo[]> {
  return invoke<WorktreeInfo[]>("list_worktrees");
}

/**
 * Create a new linked worktree at `path` on `branch`.
 * Set `createBranch` to true to create a new branch with `-b`.
 */
export async function createWorktree(path: string, branch: string, createBranch: boolean): Promise<void> {
  return invoke<void>("create_worktree", { path, branch, createBranch });
}

/**
 * Remove a linked worktree at `path`.
 * Set `force` to true to remove locked or dirty worktrees.
 */
export async function removeWorktree(path: string, force: boolean): Promise<void> {
  return invoke<void>("remove_worktree", { path, force });
}

// ---------------------------------------------------------------------------
// Blame & file history
// ---------------------------------------------------------------------------

/** Get per-line blame information for a file, optionally at a specific commit. */
export async function blameFile(path: string, oid?: string): Promise<BlameLine[]> {
  return invoke<BlameLine[]>("blame_file", { path, oid: oid ?? null });
}

/** Get the commit history for a specific file with rename tracking. */
export async function fileHistory(path: string, limit?: number): Promise<FileHistoryEntry[]> {
  return invoke<FileHistoryEntry[]>("file_history", { path, limit: limit ?? null });
}

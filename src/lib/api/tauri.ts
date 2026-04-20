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
import type { RepoInfo, GraphViewport, CommitInfo, CommitFileChange, BranchInfo, FileStatus, FileDiff, ProviderUser, ProviderStatusResponse, CiRun, CiRunDetail, TaskInfo, TaskId, TaskOutputLine, ProjectInfo, RecentRepo, RemoteInfo, StatusSummary, StashEntry, TagInfo, CommitStats, ConflictStatus, ConflictFileContents, ThemeMeta, ThemeData, WorktreeInfo, HunkSelection, BlameLine, FileHistoryEntry, RebaseCommit, RebaseAction, GraphColumnConfig, ReflogEntry, CleanItem, ConfigEntry, ConfigScope, PatchPreview, SubmoduleInfo, MrPr, MrPrDetail, MrPrDiffFile, Label, ProjectSnapshot, AvailableAiProvider, RepoAiStatus, AiSession, AiWorktree, AiConfigFile, BisectState, CliAuthStatus, DebugInfo, Issue, IssueDetail, IssueState, Milestone, Workflow, TriggerResult, Release, ReleaseAsset, ReleaseDetail, CreateReleaseInput, EditReleasePatch, StartBackgroundRunRequest, StartBackgroundRunResponse, AiBackgroundSettings } from "../types";

export async function openRepo(path: string): Promise<RepoInfo> {
  return invoke<RepoInfo>("open_repo", { path });
}

export async function getGraphViewport(offset: number, limit: number): Promise<GraphViewport> {
  return invoke<GraphViewport>("get_graph_viewport", { offset, limit });
}

/**
 * Rebuild the active project's cached graph layout from the current
 * repository state. Called after a local mutation (commit / amend) or
 * after a completed Fetch / Pull / Push task so the next
 * `getGraphViewport` call slices a layout that includes the new commits
 * / refs. Under the hood this re-runs the `load_or_build_layout`
 * pipeline, which correctly misses the persistent on-disk cache when
 * HEAD or refs have moved.
 */
export async function refreshGraphLayout(): Promise<void> {
  return invoke<void>("refresh_graph_layout");
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

export async function createCommit(message: string): Promise<string> {
  return invoke<string>("create_commit", { message });
}

export async function createBranch(name: string): Promise<void> {
  return invoke("create_branch", { name });
}

export async function createBranchAt(name: string, oid: string): Promise<void> {
  return invoke("create_branch_at", { name, oid });
}

export async function checkoutDetached(oid: string): Promise<void> {
  return invoke("checkout_detached", { oid });
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

export async function rebaseBranch(onto: string): Promise<string> {
  return invoke<string>("rebase_branch", { onto });
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

/** Get the ours/theirs/base content of a conflicted file from the index. */
export async function getConflictFileContents(path: string): Promise<ConflictFileContents> {
  return invoke<ConflictFileContents>("get_conflict_file_contents", { path });
}

/** Write resolved content to disk and mark the file as resolved. */
export async function writeResolvedFile(path: string, content: string): Promise<void> {
  return invoke<void>("write_resolved_file", { path, content });
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

// -- CI/CD control (Phase 8.4) --

export async function triggerWorkflow(
  workflowId: string,
  gitRef: string,
  inputs: Record<string, string>,
): Promise<TriggerResult> {
  return invoke<TriggerResult>("trigger_workflow", { workflowId, gitRef, inputs });
}

export async function retryCiRun(runId: string): Promise<void> {
  return invoke<void>("retry_ci_run", { runId });
}

export async function retryCiFailedJobs(runId: string): Promise<void> {
  return invoke<void>("retry_ci_failed_jobs", { runId });
}

export async function retryCiJob(jobId: string): Promise<void> {
  return invoke<void>("retry_ci_job", { jobId });
}

export async function cancelCiRun(runId: string): Promise<void> {
  return invoke<void>("cancel_ci_run", { runId });
}

export async function listCiWorkflows(): Promise<Workflow[]> {
  return invoke<Workflow[]>("list_ci_workflows");
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

/**
 * Cancel a running task by its string id.
 *
 * Used by the unified tasks drawer, where task ids are string-shaped
 * across AI runs, git ops, and auto-update downloads. Wraps the
 * `task_cancel` Rust IPC command which parses the id into a `TaskId`
 * and fires the underlying `CancellationToken`.
 *
 * @param id — task id as emitted in the `task://update` event payload.
 */
export async function taskCancel(id: string): Promise<void> {
  return invoke<void>("task_cancel", { id });
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

export async function getProjectSnapshot(path: string): Promise<ProjectSnapshot | null> {
  return invoke<ProjectSnapshot | null>("get_project_snapshot", { path });
}

export async function saveProjectSnapshot(snapshot: ProjectSnapshot): Promise<void> {
  return invoke<void>("save_project_snapshot", { snapshot });
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

/** Resolve the startup theme from saved config + OS preference. */
export async function resolveStartupTheme(): Promise<ThemeData> {
  return invoke<ThemeData>("resolve_startup_theme");
}

export async function getUiScale(): Promise<number> {
  return invoke<number>("get_ui_scale");
}

export async function setUiScale(scale: number): Promise<void> {
  return invoke<void>("set_ui_scale", { scale });
}

/**
 * Return whether the app should silently probe for updates on startup.
 * Default `true`. Persisted in `AppConfig::auto_check_updates`.
 */
export async function getAutoCheckUpdates(): Promise<boolean> {
  return invoke<boolean>("get_auto_check_updates");
}

/** Persist the `auto_check_updates` preference. */
export async function setAutoCheckUpdates(enabled: boolean): Promise<void> {
  return invoke<void>("set_auto_check_updates", { enabled });
}

/**
 * Return whether the per-OS re-authorization notice has been dismissed.
 * `os` must be `"macos"` or `"windows"` — Linux never shows the dialog.
 */
export async function getReauthDismissed(os: string): Promise<boolean> {
  return invoke<boolean>("get_reauth_dismissed", { os });
}

/** Persist the re-authorization-notice dismissal for a single OS. */
export async function setReauthDismissed(
  os: string,
  dismissed: boolean,
): Promise<void> {
  return invoke<void>("set_reauth_dismissed", { os, dismissed });
}

export async function getGraphColumns(): Promise<GraphColumnConfig[]> {
  return invoke<GraphColumnConfig[]>("get_graph_columns");
}

export async function setGraphColumns(columns: GraphColumnConfig[]): Promise<void> {
  return invoke<void>("set_graph_columns", { columns });
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

/** Lock a linked worktree, preventing accidental removal. */
export async function lockWorktree(path: string, reason?: string): Promise<void> {
  return invoke<void>("worktree_lock", { path, reason: reason ?? null });
}

/** Unlock a previously locked worktree. */
export async function unlockWorktree(path: string): Promise<void> {
  return invoke<void>("worktree_unlock", { path });
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

// ---------------------------------------------------------------------------
// Interactive rebase
// ---------------------------------------------------------------------------

/** Get commits between base (exclusive) and HEAD in rebase order (oldest first). */
export async function getRebaseCommits(baseOid: string): Promise<RebaseCommit[]> {
  return invoke<RebaseCommit[]>("get_rebase_commits", { baseOid });
}

/** Start an interactive rebase with pre-defined actions. */
export async function startInteractiveRebase(baseOid: string, actions: RebaseAction[]): Promise<void> {
  return invoke<void>("start_interactive_rebase", { baseOid, actions });
}

// ---------------------------------------------------------------------------
// Reflog
// ---------------------------------------------------------------------------

/** Get the HEAD reflog entries, limited to the given count (default 100). */
export async function getReflog(limit?: number): Promise<ReflogEntry[]> {
  return invoke<ReflogEntry[]>("get_reflog", { limit: limit ?? null });
}

// Clean (untracked file removal)
// ---------------------------------------------------------------------------

/** Preview untracked/ignored files that would be removed by git clean. */
export async function cleanDryRun(
  includeDirectories: boolean,
  includeIgnored: boolean,
  onlyIgnored: boolean,
): Promise<CleanItem[]> {
  return invoke<CleanItem[]>("clean_dry_run", {
    includeDirectories,
    includeIgnored,
    onlyIgnored,
  });
}

/** Permanently remove the specified paths from the working directory. */
export async function cleanPaths(paths: string[]): Promise<number> {
  return invoke<number>("clean_paths", { paths });
}

// Git config
// ---------------------------------------------------------------------------

/** List all config entries at the given scope. */
export async function listConfig(scope: ConfigScope): Promise<ConfigEntry[]> {
  return invoke<ConfigEntry[]>("list_config", { scope });
}

/** Set a config key to a value at the given scope. */
export async function setConfig(scope: ConfigScope, key: string, value: string): Promise<void> {
  return invoke<void>("set_config", { scope, key, value });
}

/** Remove a config key at the given scope. */
export async function unsetConfig(scope: ConfigScope, key: string): Promise<void> {
  return invoke<void>("unset_config", { scope, key });
}

/** Add a new value for a config key at the given scope (multi-value append). */
export async function addConfig(scope: ConfigScope, key: string, value: string): Promise<void> {
  return invoke<void>("add_config", { scope, key, value });
}

// Gitignore management
// ---------------------------------------------------------------------------

/** Read the content of the repository's .gitignore file. */
export async function readGitignore(): Promise<string> {
  return invoke<string>("read_gitignore");
}

/** Write the full content of the repository's .gitignore file. */
export async function writeGitignore(content: string): Promise<void> {
  return invoke<void>("write_gitignore", { content });
}

/** Add a single pattern to the repository's .gitignore file. */
export async function addGitignorePattern(pattern: string): Promise<void> {
  return invoke<void>("add_gitignore_pattern", { pattern });
}

// Patch management
// ---------------------------------------------------------------------------

/** Save raw patch text to a file on disk. */
export async function savePatchToFile(path: string, content: string): Promise<void> {
  return invoke<void>("save_patch_to_file", { path, content });
}

/** Create .patch files from commits via git format-patch. */
export async function createCommitPatches(oids: string[], outputDir: string): Promise<string[]> {
  return invoke<string[]>("create_commit_patches", { oids, outputDir });
}

/** Create a patch from working tree changes (staged or all). */
export async function createWorkingTreePatch(stagedOnly: boolean): Promise<string> {
  return invoke<string>("create_working_tree_patch", { stagedOnly });
}

/** Preview a patch file: stats + clean-apply check. */
export async function previewPatch(path: string): Promise<PatchPreview> {
  return invoke<PatchPreview>("preview_patch", { path });
}

/** Apply a patch file. Set threeWay=true for 3-way merge fallback. */
export async function applyPatch(path: string, threeWay: boolean): Promise<string> {
  return invoke<string>("apply_patch", { path, threeWay });
}

// Submodules
// ---------------------------------------------------------------------------

/** List all submodules in the active repository. */
export async function listSubmodules(): Promise<SubmoduleInfo[]> {
  return invoke<SubmoduleInfo[]>("list_submodules");
}

/** Initialize a submodule. */
export async function initSubmodule(path: string): Promise<void> {
  return invoke<void>("init_submodule", { path });
}

/** Update a single submodule (background task). */
export async function updateSubmodule(path: string): Promise<TaskId> {
  return invoke<TaskId>("update_submodule", { path });
}

/** Update all submodules (background task). */
export async function updateAllSubmodules(): Promise<TaskId> {
  return invoke<TaskId>("update_all_submodules");
}

/** Deinitialize a submodule. */
export async function deinitSubmodule(path: string, force: boolean): Promise<void> {
  return invoke<void>("deinit_submodule", { path, force });
}

/** Add a new submodule to the repository. */
export async function addSubmodule(url: string, path: string): Promise<void> {
  return invoke<void>("add_submodule", { url, path });
}

/** Remove a submodule completely (deinit + rm). */
export async function removeSubmodule(path: string): Promise<void> {
  return invoke<void>("remove_submodule", { path });
}

/** Get the absolute filesystem path of a submodule. */
export async function submoduleAbsPath(submodulePath: string): Promise<string> {
  return invoke<string>("submodule_abs_path", { submodulePath });
}

// ---------------------------------------------------------------------------
// CLI provider auth
// ---------------------------------------------------------------------------

/** Check if the CLI tool is authenticated. */
export async function isCliAuthenticated(kind: string): Promise<boolean> {
  return invoke<boolean>("is_cli_authenticated", { kind });
}

/** Start CLI OAuth login flow (opens browser). */
export async function cliLogin(kind: string, instanceUrl?: string): Promise<ProviderUser> {
  return invoke<ProviderUser>("cli_login", { kind, instanceUrl: instanceUrl ?? null });
}

/** Check auth status for both gh and glab CLIs. */
export async function cliCheckAuthStatus(): Promise<CliAuthStatus[]> {
  return invoke<CliAuthStatus[]>("cli_check_auth_status");
}

/** Get the shell command to launch an interactive auth flow. */
export async function cliGetAuthCommand(tool: string): Promise<string> {
  return invoke<string>("cli_get_auth_command", { tool });
}

/** Get the shell command to log out of a CLI tool. */
export async function cliGetLogoutCommand(tool: string): Promise<string> {
  return invoke<string>("cli_get_logout_command", { tool });
}

// ---------------------------------------------------------------------------
// MR/PR management
// ---------------------------------------------------------------------------

/** List merge requests / pull requests. */
export async function listMrPrs(stateFilter?: string, limit?: number): Promise<MrPr[]> {
  return invoke<MrPr[]>("list_mr_prs", {
    stateFilter: stateFilter ?? null,
    limit: limit ?? null,
  });
}

/** Get detailed info about a single MR/PR. */
export async function getMrPrDetail(number: number): Promise<MrPrDetail> {
  return invoke<MrPrDetail>("get_mr_pr_detail", { number });
}

/** Get changed files in a MR/PR diff. */
export async function getMrPrDiff(number: number): Promise<MrPrDiffFile[]> {
  return invoke<MrPrDiffFile[]>("get_mr_pr_diff", { number });
}

/** Create a new MR/PR. */
export async function createMrPr(
  source: string, target: string, title: string, body: string,
  draft: boolean, labels: string[], reviewers: string[]
): Promise<MrPr> {
  return invoke<MrPr>("create_mr_pr", { source, target, title, body, draft, labels, reviewers });
}

/** Edit a MR/PR. */
export async function editMrPr(number: number, title?: string, body?: string): Promise<void> {
  return invoke<void>("edit_mr_pr", { number, title: title ?? null, body: body ?? null });
}

/** Merge a MR/PR. */
export async function mergeMrPr(number: number, strategy: string): Promise<void> {
  return invoke<void>("merge_mr_pr", { number, strategy });
}

/** Close a MR/PR. */
export async function closeMrPr(number: number): Promise<void> {
  return invoke<void>("close_mr_pr", { number });
}

/** Approve a MR/PR. */
export async function approveMrPr(number: number): Promise<void> {
  return invoke<void>("approve_mr_pr", { number });
}

/** Request changes on a MR/PR. */
export async function requestChangesMrPr(number: number, body: string): Promise<void> {
  return invoke<void>("request_changes_mr_pr", { number, body });
}

/** Add a general comment to a MR/PR. */
export async function addMrPrComment(number: number, body: string): Promise<void> {
  return invoke<void>("add_mr_pr_comment", { number, body });
}

/** Add an inline comment on a specific file and line. */
export async function addMrPrInlineComment(number: number, path: string, line: number, body: string): Promise<void> {
  return invoke<void>("add_mr_pr_inline_comment", { number, path, line, body });
}

// Phase 8.2 — MR/PR enhancements

/** Add labels to an existing MR/PR. */
export async function addMrPrLabels(number: number, labels: string[]): Promise<void> {
  return invoke<void>("add_mr_pr_labels", { number, labels });
}

/** Remove labels from an existing MR/PR. */
export async function removeMrPrLabels(number: number, labels: string[]): Promise<void> {
  return invoke<void>("remove_mr_pr_labels", { number, labels });
}

/** Add reviewers to an existing MR/PR. */
export async function addMrPrReviewers(number: number, reviewers: string[]): Promise<void> {
  return invoke<void>("add_mr_pr_reviewers", { number, reviewers });
}

/** Remove reviewers from an existing MR/PR. */
export async function removeMrPrReviewers(number: number, reviewers: string[]): Promise<void> {
  return invoke<void>("remove_mr_pr_reviewers", { number, reviewers });
}

/** Mark a draft MR/PR as ready for review. */
export async function markMrPrReady(number: number): Promise<void> {
  return invoke<void>("mark_mr_pr_ready", { number });
}

/** Convert a ready MR/PR back to draft. */
export async function markMrPrDraft(number: number): Promise<void> {
  return invoke<void>("mark_mr_pr_draft", { number });
}

/** Reopen a previously closed MR/PR. */
export async function reopenMrPr(number: number): Promise<void> {
  return invoke<void>("reopen_mr_pr", { number });
}

/** Mark a GitLab discussion as resolved. GitHub returns an error. */
export async function resolveDiscussion(number: number, discussionId: string): Promise<void> {
  return invoke<void>("resolve_discussion", { number, discussionId });
}

/** Mark a GitLab discussion as unresolved. GitHub returns an error. */
export async function unresolveDiscussion(number: number, discussionId: string): Promise<void> {
  return invoke<void>("unresolve_discussion", { number, discussionId });
}

/** List all repository labels (for the label picker UI). */
export async function listLabels(): Promise<Label[]> {
  return invoke<Label[]>("list_labels");
}

/** Check out a MR/PR branch locally. Returns a task ID; the parsed result comes via the `mr-pr-checked-out` event. */
export async function checkoutMrPrLocally(number: number): Promise<TaskId> {
  return invoke<TaskId>("checkout_mr_pr_locally", { number });
}

// ─── Issues (Phase 8.3) ──────────────────────────────────────────────

/**
 * List issues for the current repo, optionally filtered.
 *
 * All filter args except `limit` are optional; `state` accepts `"open"` or
 * `"closed"` (omit for all states).
 */
export async function listIssues(
  state?: IssueState,
  author?: string,
  assignee?: string,
  label?: string,
  milestone?: number,
  text?: string,
  limit: number = 50,
): Promise<Issue[]> {
  return invoke<Issue[]>("list_issues", {
    stateFilter: state,
    author,
    assignee,
    label,
    milestone,
    text,
    limit,
  });
}

/** Fetch full detail (body + comments) for a single issue. */
export async function getIssue(number: number): Promise<IssueDetail> {
  return invoke<IssueDetail>("get_issue", { number });
}

/** Create a new issue. Returns the created issue's summary. */
export async function createIssue(
  title: string,
  body: string,
  labels: string[],
  assignees: string[],
  milestone: number | null,
): Promise<Issue> {
  return invoke<Issue>("create_issue", {
    title,
    body,
    labels,
    assignees,
    milestone,
  });
}

/** Edit an issue's title and/or body. */
export async function editIssue(
  number: number,
  title?: string,
  body?: string,
): Promise<void> {
  return invoke<void>("edit_issue", { number, title, body });
}

/** Close an open issue. */
export async function closeIssue(number: number): Promise<void> {
  return invoke<void>("close_issue", { number });
}

/** Reopen a closed issue. */
export async function reopenIssue(number: number): Promise<void> {
  return invoke<void>("reopen_issue", { number });
}

/** Post a general comment on an issue. */
export async function addIssueComment(
  number: number,
  body: string,
): Promise<void> {
  return invoke<void>("add_issue_comment", { number, body });
}

/** Add labels to an issue. */
export async function addIssueLabels(
  number: number,
  labels: string[],
): Promise<void> {
  return invoke<void>("add_issue_labels", { number, labels });
}

/** Remove labels from an issue. */
export async function removeIssueLabels(
  number: number,
  labels: string[],
): Promise<void> {
  return invoke<void>("remove_issue_labels", { number, labels });
}

/** Add assignees to an issue. */
export async function addIssueAssignees(
  number: number,
  assignees: string[],
): Promise<void> {
  return invoke<void>("add_issue_assignees", { number, assignees });
}

/** Remove assignees from an issue. */
export async function removeIssueAssignees(
  number: number,
  assignees: string[],
): Promise<void> {
  return invoke<void>("remove_issue_assignees", { number, assignees });
}

/** Set (or clear with `null`) the milestone on an issue. */
export async function setIssueMilestone(
  number: number,
  milestoneId: number | null,
): Promise<void> {
  return invoke<void>("set_issue_milestone", { number, milestoneId });
}

/** List all milestones for the current repo. */
export async function listMilestones(): Promise<Milestone[]> {
  return invoke<Milestone[]>("list_milestones");
}

// ─── Releases (Phase 8.5) ────────────────────────────────────────────

/** List releases for the current repository, newest first. */
export async function listReleases(limit: number = 30): Promise<Release[]> {
  return invoke<Release[]>("list_releases", { limit });
}

/** Fetch full detail (body + assets) for a single release by tag. */
export async function getReleaseDetail(tag: string): Promise<ReleaseDetail> {
  return invoke<ReleaseDetail>("get_release_detail", { tag });
}

/** List just the asset records for a release. */
export async function listReleaseAssets(tag: string): Promise<ReleaseAsset[]> {
  return invoke<ReleaseAsset[]>("list_release_assets", { tag });
}

/** Create a new release from a `CreateReleaseInput`. */
export async function createRelease(input: CreateReleaseInput): Promise<Release> {
  return invoke<Release>("create_release", { input });
}

/** Edit a release's title, body, and/or draft/prerelease flags. */
export async function editRelease(
  tag: string,
  patch: EditReleasePatch,
): Promise<void> {
  return invoke<void>("edit_release", { tag, patch });
}

/** Delete a release. The underlying tag is not removed. */
export async function deleteRelease(tag: string): Promise<void> {
  return invoke<void>("delete_release", { tag });
}

/** Publish a draft release. GitHub only — GitLab returns an error. */
export async function publishRelease(tag: string): Promise<void> {
  return invoke<void>("publish_release", { tag });
}

/** Delete a single release asset by ID. */
export async function deleteReleaseAsset(
  tag: string,
  assetId: number,
): Promise<void> {
  return invoke<void>("delete_release_asset", { tag, assetId });
}

/**
 * Upload a binary asset to a release.
 *
 * Non-blocking: returns a TaskId immediately. Subscribe to task events to
 * track progress and completion. Re-fetch the release detail on success to
 * see the new asset row.
 */
export async function uploadReleaseAsset(
  tag: string,
  assetPath: string,
  label?: string,
): Promise<TaskId> {
  return invoke<TaskId>("upload_release_asset", { tag, assetPath, label });
}

/**
 * Atomic create-tag + push + create-release.
 *
 * Runs tag creation and push as a TaskManager task. On success emits a
 * `release-created` event with the created `Release`; on release-step
 * failure emits `release-create-failed` with `{ tag, error }`.
 */
export async function createTagAndRelease(
  tag: string,
  sourceRef: string,
  remote: string,
  input: CreateReleaseInput,
): Promise<TaskId> {
  return invoke<TaskId>("create_tag_and_release", {
    tag,
    sourceRef,
    remote,
    input,
  });
}

// ── Sidebar ─────────────────────────────────────────────────────────

/** Get persisted sidebar collapsed state. */
export async function getSidebarCollapsed(): Promise<boolean> {
  return invoke<boolean>("get_sidebar_collapsed");
}

/** Persist sidebar collapsed state. */
export async function setSidebarCollapsed(collapsed: boolean): Promise<void> {
  return invoke<void>("set_sidebar_collapsed", { collapsed });
}

// ── Terminal ──────────────────────────────────────────────────────────

/** Spawn a new terminal session in the given directory. */
export async function terminalSpawn(
  cwd: string,
  cols: number,
  rows: number,
): Promise<number> {
  return invoke<number>("terminal_spawn", { cwd, cols, rows });
}

/** Write input bytes to a terminal session (base64-encoded). */
export async function terminalWrite(
  id: number,
  data: string,
): Promise<void> {
  return invoke<void>("terminal_write", { id, data });
}

/** Resize a terminal session. */
export async function terminalResize(
  id: number,
  cols: number,
  rows: number,
): Promise<void> {
  return invoke<void>("terminal_resize", { id, cols, rows });
}

/** Kill a terminal session. */
export async function terminalKill(id: number): Promise<void> {
  return invoke<void>("terminal_kill", { id });
}

/**
 * Tell the backend which terminal session is currently visible, so the
 * foreground-process polling thread only polls that session. Pass `null`
 * when no terminal is focused.
 */
export async function terminalSetActive(id: number | null): Promise<void> {
  return invoke<void>("terminal_set_active", { id });
}

// ─── AI Provider ───

export async function aiGetProviders(): Promise<AvailableAiProvider[]> {
  return invoke<AvailableAiProvider[]>("ai_get_providers");
}

export async function aiGetRepoStatus(): Promise<RepoAiStatus[]> {
  return invoke<RepoAiStatus[]>("ai_get_repo_status");
}

export async function aiRefreshDetection(): Promise<void> {
  return invoke<void>("ai_refresh_detection");
}

export async function aiGenerateCommitMessage(provider: string): Promise<TaskId> {
  return invoke<TaskId>("ai_generate_commit_message", { provider });
}

export async function aiAnalyzeCode(provider: string, content: string, question: string): Promise<TaskId> {
  return invoke<TaskId>("ai_analyze_code", { provider, content, question });
}

export async function aiGeneratePrDescription(provider: string): Promise<TaskId> {
  return invoke<TaskId>("ai_generate_pr_description", { provider });
}

export async function aiReviewCode(provider: string, diff: string): Promise<TaskId> {
  return invoke<TaskId>("ai_review_code", { provider, diff });
}

export async function aiReviewPr(provider: string, diff: string): Promise<TaskId> {
  return invoke<TaskId>("ai_review_pr", { provider, diff });
}

export async function aiLaunchInteractive(provider: string): Promise<number> {
  return invoke<number>("ai_launch_interactive", { provider });
}

export async function aiLaunchWorktree(provider: string, name?: string): Promise<number | null> {
  return invoke<number | null>("ai_launch_worktree", { provider, name: name ?? null });
}

export async function aiListSessions(): Promise<AiSession[]> {
  return invoke<AiSession[]>("ai_list_sessions");
}

export async function aiListWorktrees(): Promise<AiWorktree[]> {
  return invoke<AiWorktree[]>("ai_list_worktrees");
}

export async function aiCleanupWorktree(provider: string, worktreePath: string): Promise<void> {
  return invoke<void>("ai_cleanup_worktree", { provider, worktree_path: worktreePath });
}

export async function aiGetConfigFiles(): Promise<AiConfigFile[]> {
  return invoke<AiConfigFile[]>("ai_get_config_files");
}

export async function aiGetPreferredProvider(): Promise<string | null> {
  return invoke<string | null>("ai_get_preferred_provider");
}

export async function aiSetPreferredProvider(provider: string | null): Promise<void> {
  return invoke<void>("ai_set_preferred_provider", { provider });
}

/** Start watching AI config directories for live-reload events. */
export async function aiWatchConfigDirs(): Promise<void> {
  return invoke<void>("ai_watch_config_dirs");
}

/** Stop the AI config directory watcher. */
export async function aiStopConfigWatcher(): Promise<void> {
  return invoke<void>("ai_stop_config_watcher");
}

/** Resume an existing AI session in a new terminal tab. Returns null if the provider does not support resume. */
export async function aiResumeSession(provider: string, sessionId: string): Promise<number | null> {
  return invoke<number | null>("ai_resume_session", { provider, sessionId });
}

export async function aiReadConfigFile(path: string): Promise<string> {
  return invoke<string>("ai_read_config_file", { path });
}

export async function aiWriteConfigFile(path: string, content: string): Promise<void> {
  return invoke<void>("ai_write_config_file", { path, content });
}

export async function aiCreateConfigFile(kind: string, scope: string, name: string): Promise<AiConfigFile> {
  return invoke<AiConfigFile>("ai_create_config_file", { kind, scope, name });
}

// ─── AI Background Worktree ─────────────────────────────────────────

/** Kick off a new headless AI run inside a freshly-created worktree. */
export async function aiStartBackgroundRun(
  request: StartBackgroundRunRequest,
): Promise<StartBackgroundRunResponse> {
  return invoke<StartBackgroundRunResponse>("ai_start_background_run", { request });
}

/** Request cancellation of a running or queued AI background session. */
export async function aiCancelBackgroundRun(sessionId: string): Promise<void> {
  return invoke<void>("ai_cancel_background_run", { sessionId });
}

/** List every known AI background run (queued, running, or terminal). */
export async function aiListBackgroundRuns(): Promise<AiSession[]> {
  return invoke<AiSession[]>("ai_list_background_runs");
}

/** Fetch a single background run by session id; `null` if not found. */
export async function aiGetBackgroundRun(sessionId: string): Promise<AiSession | null> {
  return invoke<AiSession | null>("ai_get_background_run", { sessionId });
}

/** Remove the worktree + branch created for a terminal-state background run. */
export async function aiDiscardBackgroundRunWorktree(sessionId: string): Promise<void> {
  return invoke<void>("ai_discard_background_run_worktree", { sessionId });
}

/** Attach a new PTY terminal to the worktree of a background run. */
export async function aiOpenBackgroundTerminal(sessionId: string): Promise<number> {
  return invoke<number>("ai_open_background_terminal", { sessionId });
}

/** Read persisted AI background settings. */
export async function aiBackgroundGetSettings(): Promise<AiBackgroundSettings> {
  return invoke<AiBackgroundSettings>("ai_background_get_settings");
}

/** Persist AI background settings (concurrency, worktree root, auto-accept). */
export async function aiBackgroundSetSettings(settings: AiBackgroundSettings): Promise<void> {
  return invoke<void>("ai_background_set_settings", { settings });
}

// ─── Bisect ─────────────────────────────────────────────────────────

/** Start a bisect session, optionally providing bad and good commits. */
export async function bisectStart(bad?: string, good?: string): Promise<string> {
  return invoke<string>("bisect_start", { bad: bad ?? null, good: good ?? null });
}

/** Mark a commit (or current HEAD) as good. */
export async function bisectGood(commit?: string): Promise<string> {
  return invoke<string>("bisect_good", { commit: commit ?? null });
}

/** Mark a commit (or current HEAD) as bad. */
export async function bisectBad(commit?: string): Promise<string> {
  return invoke<string>("bisect_bad", { commit: commit ?? null });
}

/** Skip the current commit. */
export async function bisectSkip(): Promise<string> {
  return invoke<string>("bisect_skip");
}

/** Reset (end) the bisect session. */
export async function bisectReset(): Promise<string> {
  return invoke<string>("bisect_reset");
}

/** Get the current bisect session state. */
export async function bisectGetState(): Promise<BisectState> {
  return invoke<BisectState>("bisect_get_state");
}

/** Get the bisect log. */
export async function bisectGetLog(): Promise<string> {
  return invoke<string>("bisect_get_log");
}

/** Run an automated bisect with a test command. */
export async function bisectRunAuto(testCommand: string): Promise<string> {
  return invoke<string>("bisect_run_auto", { testCommand });
}

// ─── Debug / Logging ────────────────────────────────────────────────

/** Get debug information (version, OS, git version, log path). */
export async function getDebugInfo(): Promise<DebugInfo> {
  return invoke<DebugInfo>("get_debug_info");
}

/** Get the log file directory path. */
export async function getLogPath(): Promise<string> {
  return invoke<string>("get_log_path");
}

/** Open the log directory in the system file manager. */
export async function openLogDirectory(): Promise<void> {
  return invoke<void>("open_log_directory");
}

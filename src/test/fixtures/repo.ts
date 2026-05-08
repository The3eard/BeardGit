/**
 * Factories for repo-level fixtures: RepoInfo, StatusSummary,
 * ProjectInfo, RecentRepo, RemoteInfo.
 *
 * Every factory returns a fully-populated, sensible default and
 * accepts a `Partial<T>` for overrides — keeps individual tests
 * focussed on the field that matters for the scenario.
 */

import type {
  ConflictStatus,
  ProjectInfo,
  ProjectSnapshot,
  RecentRepo,
  RemoteInfo,
  RepoInfo,
  StatusSummary,
} from "../../lib/types";

export function makeRepoInfo(overrides: Partial<RepoInfo> = {}): RepoInfo {
  return {
    path: "/Users/test/projects/sample",
    head_branch: "feat/example",
    head_oid: "abc123def4567890abc123def4567890abc123de",
    branch_count: 5,
    ...overrides,
  };
}

export function makeStatusSummary(
  overrides: Partial<StatusSummary> = {},
): StatusSummary {
  return {
    ahead: 0,
    behind: 0,
    staged: 0,
    unstaged: 0,
    untracked: 0,
    conflicted: 0,
    stash_count: 0,
    ...overrides,
  };
}

export function makeProjectInfo(
  overrides: Partial<ProjectInfo> = {},
): ProjectInfo {
  return {
    path: "/Users/test/projects/sample",
    name: "sample",
    head_branch: "feat/example",
    change_count: 0,
    is_worktree: false,
    ...overrides,
  };
}

export function makeRecentRepo(
  overrides: Partial<RecentRepo> = {},
): RecentRepo {
  return {
    path: "/Users/test/projects/sample",
    name: "sample",
    ...overrides,
  };
}

export function makeRemoteInfo(
  overrides: Partial<RemoteInfo> = {},
): RemoteInfo {
  return {
    name: "origin",
    url: "git@github.com:adolfofuentes/sample.git",
    ...overrides,
  };
}

export function makeProjectSnapshot(
  overrides: Partial<ProjectSnapshot> = {},
): ProjectSnapshot {
  return {
    path: "/Users/test/projects/sample",
    head_branch: "feat/example",
    ahead: 0,
    behind: 0,
    staged: 0,
    unstaged: 0,
    untracked: 0,
    conflicted: 0,
    stash_count: 0,
    change_count: 0,
    graph_viewport_cache: null,
    ...overrides,
  };
}

export function makeConflictStatus(
  overrides: Partial<ConflictStatus> = {},
): ConflictStatus {
  return {
    state: "none",
    conflicted_files: [],
    can_continue: false,
    ...overrides,
  };
}

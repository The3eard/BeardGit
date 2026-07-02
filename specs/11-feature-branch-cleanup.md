# Spec 11 — Branch cleanup: [gone] detection + bulk delete of merged branches

**Priority:** P1 (feature) · **Effort:** M (2–4 days) · **Branch:** `feat/branch-cleanup` · **Depends on:** —

## Problem

Repo hygiene is a top reason people reach for a git GUI, and BeardGit has none of it: no detection of branches whose upstream is gone (deleted on the remote after a merge), no "merged into X" listing, and deletion is strictly one-at-a-time (`delete_branch` with a `-d`/`-D` force flag, `crates/app-core/src/commands/branch.rs:87`). After a few weeks of PR-driven work every repo accumulates dozens of stale locals, and the cleanup happens in a terminal (`git branch -vv | grep gone | …`) — exactly the tab-switch the product exists to eliminate.

## Goal (success criteria)

- The branches view shows a **[gone]** indicator on branches whose configured upstream no longer exists.
- A "Clean up branches…" action lists gone + fully-merged branches with checkboxes and deletes the selection in one command, with correct safety rails (below).
- Deleting N branches emits **one** `project-mutated` event (one `MutationGuard` around the batch), not N refreshes.

## Design

**Backend:**
- Extend the branch listing (`crates/git-engine/src/repository.rs::branches`) with `upstream_gone: bool` — git2 exposes this as "branch has upstream *configured* but the upstream branch lookup fails" (config entry `branch.<name>.remote/merge` present, ref missing). Zero extra cost beyond the lookup already done for ahead/behind.
- `list_merged_branches(into: Option<String>) -> Vec<String>` — locals whose tip is an ancestor of `into` (default: the repo's default branch, falling back to HEAD); git2 `graph_descendant_of`. Exclude: current branch, the target itself, branches checked out in **any worktree** (git itself refuses; surface rather than fail mid-batch).
- `delete_branches(names: Vec<String>, force: bool) -> BatchDeleteResult { deleted, failed: Vec<(name, reason)> }` — iterate with per-branch error capture (one refusal must not abort the rest), wrapped in a single `MutationGuard` (`refs_changed`). Gone branches typically need `force` (their merge was often a squash, so `-d` refuses) — pass per-selection.

**Frontend:**
- `[gone]` chip on branch rows (theme-tokened, same visual family as the existing sync-state indicators).
- "Clean up branches…" in the branches-view header menu → dialog with two grouped sections (Gone / Merged into `<default>`), checkboxes (gone pre-checked, merged unchecked by default — squash-merge workflows make "merged" incomplete, so don't oversell it), per-row last-commit date + ahead count as a second-thought signal, and a red confirm button ("Delete 7 branches"). Route through `runMutation`; result toast reports `deleted`/`failed` counts, failures expandable.
- Optional prompt integration: after a fetch/pull that prunes remotes, if new gone branches appeared, the existing toast system offers "3 branches lost their upstream — clean up?" (skippable; a setting can disable the nudge).

**Safety rails:** never list current branch, default branch, or worktree-checked-out branches; batch delete is `-d` unless the row's force checkbox (auto-set for gone-with-unpushed-commits? No — keep it explicit: gone rows show "not fully merged" warning when `-d` would refuse, and the dialog re-lists them needing force confirmation). Everything remains recoverable via the existing reflog view — say so in the dialog footer.

## Files to touch

- `crates/git-engine/src/repository.rs`, `crates/app-core/src/commands/branch.rs` (+ contract files `tauri.ts`, `types/`).
- `src/lib/components/branches/` (chip + dialog), `src/lib/stores/branches.ts`.

## Verification

1. Rust tests: fixture repo — upstream deleted → `upstream_gone`; merged/unmerged/worktree-checked-out exclusion; batch with one protected branch → partial result, others deleted, single mutation event (mutation-events test harness).
2. Vitest: dialog store logic (grouping, pre-check rules).
3. Manual: real repo with squash-merged PRs — prune, clean up, verify reflog recovery of one deleted branch.

## Out of scope

- Deleting the *remote* branches (exists already via `delete_remote_branch` per-branch; batch-remote-delete is a follow-up).
- Auto-cleanup on a schedule (against the app's "you're in control" grain).
- Stale-by-age detection (dates shown as signal; no age-based auto-selection).

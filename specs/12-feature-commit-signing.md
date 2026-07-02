# Spec 12 — Commit signing (SSH/GPG) and verified badges

**Priority:** P2 (feature; P1 for signing-mandated teams) · **Effort:** M–L (phased) · **Branch:** `feat/commit-signing` · **Depends on:** —

## Problem

BeardGit can neither create a signed commit nor display signature status. There is no `gpgsign`/`signingkey`/`--gpg-sign`/`verify-commit` anywhere in `crates/` or `src/`. GitHub and GitLab increasingly enforce signed commits via branch protection — the exact forge-centric workflows BeardGit targets — so for those teams, every commit made in BeardGit is rejected on push, breaking the one-window promise at its most basic operation. A user with `commit.gpgsign=true` in their global git config gets **silently unsigned commits** today (if commit creation goes through git2, which ignores that config).

## Goal (success criteria)

- With `commit.gpgsign=true` + `gpg.format=ssh` (or gpg) configured, a commit created in BeardGit is signed: `git log --show-signature -1` verifies, and GitHub shows "Verified".
- The commit box indicates signing status before committing ("Will be signed (SSH)" subtle chip), and surfaces signing *failures* with the stderr (wrong key path, gpg agent locked) instead of a generic error.
- Commit detail view shows signature presence (+ verification result where cheaply obtainable).
- Users with no signing config see zero change.

## Design

**Phase 1 — sign at creation by honoring git config (the 90% win).**
Route commit creation through the git CLI when signing is configured: `create_commit` / `amend_commit` (`crates/app-core/src/commands/commit.rs`) check `commit.gpgsign` + `gpg.format` (one git2 config read); if signing is on, shell out to `git commit` (the CLI handles ssh/gpg/x509, agents, and passphrase prompts via its own machinery) instead of the git2 path. This matches the repo's stated hybrid policy — "git2 for reads, CLI for complex writes" — signing *is* a complex write. Failure mode: capture stderr and surface it (typed error `signing_failed` if Spec 05's envelope has landed). Same treatment for the other commit-creating paths: merge commits, revert, cherry-pick (CLI paths likely already sign — verify with tests), and the AI worktree commit path in the provider crates (`attribution.rs` — verify how those commit).
Note for interactive squash/reword: the rebase already runs with a non-interactive `GIT_EDITOR` (26.6.2 fix) — signing via gpg with a passphrase-locked key can still block; document that agent-based setups (ssh-agent / gpg-agent) are the supported mode, and time out with a clear error otherwise.

**Phase 2 — commit-box awareness + settings surface.**
- Commit box chip via a cheap `get_signing_config` command (enabled, format, key present?). "Key present" = file-exists for ssh (`user.signingkey` path) / `gpg --list-keys` hit for gpg — diagnostic only, never blocks.
- Git settings view (`GitSettings.svelte` already edits config keys): group `commit.gpgsign`, `gpg.format`, `user.signingkey`, `gpg.ssh.allowedSignersFile` with a "Test signing" button that signs a throwaway blob and reports the result.

**Phase 3 — badges in history.**
- `get_commit_signature(oid)`: git2 `Commit::extract_signature` → presence + format (cheap, no verification). Shown in commit detail as an "S" chip.
- Actual *verification* (badge turns green/red) shells to `git verify-commit` — expensive, so **lazy**: only for the commit open in the detail pane, cached per-OID in memory. Do not verify in the graph — presence-only there, if anything (presence requires an object read per commit; consider detail-view-only for v1 and skip graph chips entirely).
- Signed tags: `create_tag` gains `-s` when `tag.gpgSign` or explicit toggle (small; ride along).

## Files to touch

- `crates/git-engine/src/` commit paths + `cli.rs` (signed-commit runner), `crates/app-core/src/commands/commit.rs`, `tag.rs`.
- `src/lib/components/changes/` (commit box chip), `settings/GitSettings.svelte`, commit detail component; contract files.

## Verification

1. Rust integration tests: fixture with ssh signing configured (generate a throwaway ed25519 key in the test) — commit through the engine, `git verify-commit` passes; `gpgsign=false` → byte-identical behavior to today (snapshot the git2 path is still used).
2. Amend, merge, revert, cherry-pick, interactive-rebase squash all produce signed commits under signing config (per-op tests).
3. Manual: real repo pushed to GitHub with branch protection requiring signatures — push succeeds, badge shows Verified.

## Out of scope

- In-app key generation/management (diagnostics only; generation is roadmap "SSH/credential diagnostics").
- Verifying *other people's* signatures against a trust store / allowed-signers UI (badge shows git's verdict where run; no trust management).
- x509/smimesign beyond whatever the CLI passthrough gives for free.

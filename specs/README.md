# BeardGit improvement specs

Produced from a full codebase audit (2026-07-02): four parallel deep-dives — Rust backend health, Svelte frontend health, performance hot paths, and product feature gaps — plus a CI/release review. Key quantitative claims (bisect blocking, 20K fingerprint walk, uncapped diff paths, raw-`invoke` drift, missing Vitest in CI) were re-verified against the source before writing these specs.

Each spec is sized to the repo's workflow: branch from `beta`, one concern per branch, CHANGELOG entry on completion, full CI gate green.

## Prioritized roadmap

| # | Spec | Priority | Effort | Theme |
|---|------|----------|--------|-------|
| 01 | [Async hygiene: bisect + AI coordinator](01-async-hygiene-bisect.md) | **P0** | S | Correctness/perf |
| 02 | [CI runs the tests it claims to](02-ci-test-gaps.md) | **P0** | S | Quality infra |
| 03 | [Cap every diff/file-content path](03-diff-size-caps.md) | **P0** | M | Performance |
| 04 | [Incremental mutation→graph pipeline](04-mutation-graph-pipeline.md) | **P0** | L | Performance |
| 05 | [Typed, enforced IPC contract](05-typed-ipc-contract.md) | P1 | M | Architecture |
| 06 | [Watcher learns .gitignore](06-watcher-gitignore.md) | P1 | M | Performance |
| 07 | [O(limit) deep scroll + indexed viewport](07-deep-scroll-and-viewport.md) | P1 | M | Performance |
| 08 | [Per-repo state container](08-repo-state-container.md) | P1 | L | Architecture |
| 10 | [Compare view: ref vs ref](10-feature-compare-refs.md) | P1 | M | Feature (backend already exists) |
| 11 | [Branch cleanup: [gone] + bulk delete](11-feature-branch-cleanup.md) | P1 | M | Feature |
| 09 | [Backend decomposition (repo_config, AI)](09-backend-decomposition.md) | P2 | L | Architecture |
| 12 | [Commit signing + verified badges](12-feature-commit-signing.md) | P2 | M–L | Feature |

**Suggested sequencing.** Start with 01+02 (days, immediate payoff, and 02's bench harness is a dependency for measuring 04/07). Then 03 (worst user-facing IPC offender). 04 is the flagship perf work — phased, each phase lands alone. 05 Phase 1 (ESLint guard + requests migration) is a day and stops ongoing drift — slot it early. Features 10/11 are high-leverage, self-contained, and good palate cleansers between perf phases.

## Quick wins (too small for a spec)

- **Panic hook**: no `std::panic::set_hook` in `src-tauri` — a Rust panic dies silently in a no-telemetry app. Install a hook that writes to the existing `tracing` log file (`storage::logging`) so "it crashed" reports are diagnosable. (~1 hour)
- **Root `CLAUDE.md` CI claim**: says CI runs `npm test` — it doesn't (fixed properly by Spec 02, but the doc line can be corrected immediately).
- **`open_repo` double status walk**: `change_count` re-walks statuses already computed (`commands/repository.rs`) — folded into Spec 04 Phase B, but standalone-committable.
- **Storage cache hardening**: unstable `DefaultHasher` cache key + unversioned `commits_cache` — Spec 09 Phase D, standalone-committable. (~2 hours)
- **`theme.rs` (1 811 lines)** split into types / color-math / embedded-theme registry.

## Surveyed, noted, not spec'd (roadmap candidates)

Feature gaps (from the capability-matrix audit), in rough value order for the "whole repo in one window" positioning:

- **Add a remote to an existing repo** — the fork/`upstream` workflow is impossible from the UI; rename/remove exist, add doesn't (only inside clone/init pipelines). Small feature, conspicuous hole.
- **Content/pickaxe history search** (`git log -S/-G`) — `search_commits` is metadata-only; "which commit introduced this string" is the natural power feature for a 100K-commit graph.
- **Conventional-commit helper in the commit box** — type/scope scaffold; the conventional format currently exists only for AI-drafted messages.
- **Undo safety net** — one-click undo of reset/rebase/discard (GitButler/Tower-style operation journal); today recovery is manual via reflog.
- **Image diff preview** — binary diffs dead-end at a placeholder; side-by-side/onion-skin image compare is a GUI-only differentiator.
- **Git LFS awareness** — pointer detection, tracked-pattern view, lock management.
- **Repo maintenance & health** — gc/prune/fsck, size diagnostics; pairs with Spec 11 as a "Repo health" surface.
- **`git am` mailbox import; signed tags** (signed tags ride along in Spec 12).

Frontend debt acknowledged but deliberately not scheduled: god-component splits (`+page.svelte` 1 553, `GitGraph.svelte` 1 281, `MrPrDetail` 1 057 — extract mutation orchestration when next touching each), a `runQuery` read-path counterpart to `runMutation` (23 ad-hoc loading flags today), the 77 `svelte-ignore` a11y suppressions + shared focus-trap primitive, and testing-library coverage for the largest components (the requests feature gets its smoke tests via Spec 05 Phase 1).

## Already healthy — don't "fix"

The audits flagged these as strengths; changes here would be churn:

- Layering is respected: all 299 `#[tauri::command]`s in app-core; contract crates dep-free (CI-guarded); near-zero dead code (2 `#[allow(dead_code)]`, 3 TODOs in ~60K lines).
- Production `unwrap` discipline is excellent (the scary raw counts are test code).
- Every crate has tests (app-core 337, git-engine 220, storage 135, …); frontend utils/stores well covered (157 test files). Weakest: `watcher` (4 tests — Spec 06 fixes that incidentally).
- git2-for-reads / CLI-for-writes policy is consistent and documented; no duplicated logic across the two paths.
- CI polling is exemplary (visibility-gated, overlap-skipping, auto-stopping — `stores/provider.ts`).
- The mutation-events backbone and the graph viewport slicing are the right architecture — Specs 03/04/07 optimize *within* them, not around them.

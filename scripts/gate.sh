#!/usr/bin/env bash
#
# The local CI gate, in one place, versioned.
#
# This file exists because the gate used to live only in `CLAUDE.md`, which
# is gitignored in this repo (`.gitignore` → `**/CLAUDE.md`). The list of
# checks, and the reasons behind them, therefore existed on exactly one
# machine and travelled to no other clone. The lesson that produced this
# script is the same one that produced `rust-toolchain.toml`: when the
# definition of "green" lives somewhere CI cannot see and other clones do
# not get, it drifts, and the drift is invisible until something is red for
# eight merges.
#
# Run it before claiming green:
#
#     npm run gate              # everything
#     npm run gate -- --fast    # skip the slow suites (see FAST_SKIP below)
#
# Auto-fix-and-claim-green is not acceptable — fix the underlying issue.
#
# Note the division of labour with CI: CI runs these same checks, but split
# across parallel jobs and a 3-OS matrix (`.github/workflows/ci.yml`), so it
# is deliberately *not* a call to this script. Two things follow. Adding a
# check here does not add it to CI, and `tests/visual` runs here and not in
# CI at all — its 98 baselines are macOS-only, so CI only asserts the specs
# still load.
set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1

FAST=0
[[ "${1:-}" == "--fast" ]] && FAST=1

# The two slowest, skipped by --fast. Everything else runs either way.
FAST_SKIP="cargo test --workspace|npm run test:visual"

FAILED=()
PASSED=0
SKIPPED=0

run() {
  local label="$1" cmd="$2"
  if [[ $FAST -eq 1 ]] && [[ "$cmd" =~ ^($FAST_SKIP)$ ]]; then
    printf '  \033[2m—  %-28s (skipped by --fast)\033[0m\n' "$label"
    SKIPPED=$((SKIPPED + 1))
    return
  fi

  local out status
  out=$(eval "$cmd" 2>&1)
  status=$?
  if [[ $status -eq 0 ]]; then
    printf '  \033[32m✓\033[0m  %s\n' "$label"
    PASSED=$((PASSED + 1))
  else
    printf '  \033[31m✗\033[0m  %s\n' "$label"
    # Keep the tail rather than the head: compilers and test runners put the
    # summary and the failing assertions at the end.
    printf '%s\n' "$out" | tail -25 | sed 's/^/       /'
    FAILED+=("$label")
  fi
}

printf '\nRust\n'
run "cargo fmt"            "cargo fmt --all -- --check"
run "cargo clippy"         "cargo clippy --workspace --all-targets -- -D warnings"
run "cargo test"           "cargo test --workspace"

printf '\nFrontend\n'
run "svelte-check"         "npm run check"
run "vitest"               "npm test"
run "lint"                 "npm run lint"

printf '\nContracts\n'
run "IPC contract"         "npm run check:ipc"
run "instrument fields"    "npm run check:instrument"
run "icon glyphs"          "npm run check:glyphs"
run "toolchain pin"        "npm run check:toolchain"

printf '\nPlaywright\n'
run "specs load"           "npm run check:specs"
run "visual baselines"     "npm run test:visual"

printf '\nSecurity\n'
# cargo audit blocks: those crates ship inside the app. The npm dev tree is
# informational and lives in CI only — see .github/workflows/security.yml.
run "cargo audit"          "cargo audit"
run "npm audit (prod)"     "npm audit --omit=dev --audit-level=moderate"

printf '\n'
if [[ ${#FAILED[@]} -gt 0 ]]; then
  printf '\033[31m✗ %d failed\033[0m, %d passed' "${#FAILED[@]}" "$PASSED"
  [[ $SKIPPED -gt 0 ]] && printf ', %d skipped' "$SKIPPED"
  printf ':\n'
  for f in "${FAILED[@]}"; do printf '    %s\n' "$f"; done
  printf '\n'
  exit 1
fi

printf '\033[32m✓ %d checks passed\033[0m' "$PASSED"
if [[ $SKIPPED -gt 0 ]]; then
  printf ', \033[33m%d skipped — not green until those run\033[0m' "$SKIPPED"
fi
printf '\n\n'

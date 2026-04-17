#!/usr/bin/env bash
# Run the E2E suite end-to-end inside the beardgit-e2e container.
# Lives in the repo so iterating on the flow does not require image rebuilds.
set -euo pipefail

cd /workspace

echo "==> npm ci"
npm ci

echo "==> download bundled CLI binaries (linux x64)"
node scripts/download-cli-binaries.js --target x86_64-unknown-linux-gnu

echo "==> paraglide compile"
npx @inlang/paraglide-js compile --project ./project.inlang --outdir ./src/lib/paraglide

echo "==> frontend build (with E2E test hooks enabled)"
# VITE_BEARDGIT_E2E=true toggles the window.__E2E__ surface in +layout.svelte
# (used by e2e/helpers/project.ts). Production builds omit the env var so the
# test API never ships.
VITE_BEARDGIT_E2E=true npm run build

echo "==> npx tauri build --debug --no-bundle"
# Build the app via tauri CLI rather than plain `cargo build`. The
# tauri CLI runs the full production path (beforeBuildCommand compiles
# the frontend, tauri-build bundles frontendDist into the binary via
# the asset protocol) but with --debug it keeps cargo's debug profile
# so iteration stays fast. Plain `cargo build --release` leaves the
# webview pointing at `devUrl: http://localhost:1420` because the
# release binary's asset protocol resources are only populated by the
# tauri CLI, not by cargo alone.
VITE_BEARDGIT_E2E=true npx tauri build --debug --no-bundle

echo "==> start Xvfb on :99"
Xvfb :99 -screen 0 1920x1080x24 -ac +extension GLX +render -noreset &
XVFB_PID=$!
export DISPLAY=:99
sleep 1

echo "==> start tauri-driver on 127.0.0.1:4444"
tauri-driver > /tmp/tauri-driver.log 2>&1 &
TAURI_DRIVER_PID=$!
sleep 2

cleanup() {
    echo "==> cleanup"
    kill "$TAURI_DRIVER_PID" 2>/dev/null || true
    kill "$XVFB_PID" 2>/dev/null || true
    echo "---- tauri-driver.log ----"
    cat /tmp/tauri-driver.log || true
}
trap cleanup EXIT

echo "==> run wdio"
# `tauri build --debug` emits the binary to target/debug/ (debug profile)
# but WITH the frontend bundled in as asset-protocol resources.
CI=true BEARDGIT_BUILD_TYPE=debug npx wdio run e2e/wdio.conf.ts

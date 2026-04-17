#!/usr/bin/env bash
# One-shot E2E runner — builds the docker image if needed, then runs
# the full wdio suite against a linux/amd64 Tauri build. Works on
# macOS (via docker desktop) and linux (native).
#
# Usage:
#   e2e/run.sh              # full suite
#   e2e/run.sh --rebuild    # force image rebuild (pick up Dockerfile changes)
#   e2e/run.sh --shell      # open bash in the container instead
#
# Named volumes keep cargo/node_modules warm across runs so the first
# build is slow (~10 min) but subsequent runs iterate in 1–2 min.

set -euo pipefail

IMAGE="beardgit-e2e:amd64"
REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

REBUILD=false
SHELL_MODE=false
for arg in "$@"; do
    case "$arg" in
    --rebuild) REBUILD=true ;;
    --shell) SHELL_MODE=true ;;
    *)
        echo "Unknown flag: $arg" >&2
        exit 2
        ;;
    esac
done

if ! docker info >/dev/null 2>&1; then
    echo "Docker is not running — start Docker Desktop (or dockerd) and retry." >&2
    exit 1
fi

if $REBUILD || ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
    echo "==> Building $IMAGE"
    docker build --platform linux/amd64 -f "$REPO_ROOT/e2e/Dockerfile" -t "$IMAGE" "$REPO_ROOT"
fi

RUN_ARGS=(
    --rm
    --platform linux/amd64
    -v "$REPO_ROOT:/workspace"
    -v beardgit-e2e-cargo-registry-amd64:/root/.cargo/registry
    -v beardgit-e2e-cargo-git-amd64:/root/.cargo/git
    -v beardgit-e2e-target-amd64:/workspace/target
    -v beardgit-e2e-node_modules-amd64:/workspace/node_modules
)

if $SHELL_MODE; then
    exec docker run -it "${RUN_ARGS[@]}" "$IMAGE" bash
fi

exec docker run "${RUN_ARGS[@]}" "$IMAGE" bash e2e/run-in-docker.sh

#!/bin/bash
# Create fixture repos for E2E testing
set -e

FIXTURES_DIR="$(cd "$(dirname "$0")" && pwd)"

# 1. Simple repo with a few commits
setup_simple() {
    local dir="$FIXTURES_DIR/simple-repo"
    rm -rf "$dir"
    mkdir -p "$dir" && cd "$dir"
    git init
    git config user.name "Test User"
    git config user.email "test@example.com"

    echo "# Simple Repo" > README.md
    git add README.md && git commit -m "Initial commit"

    echo "Hello" > hello.txt
    git add hello.txt && git commit -m "Add hello.txt"

    git checkout -b feature/test
    echo "Feature work" > feature.txt
    git add feature.txt && git commit -m "Add feature work"

    git checkout main
    echo "Main work" > main.txt
    git add main.txt && git commit -m "Add main work"
}

# 2. Repo with merge conflict
setup_conflict() {
    local dir="$FIXTURES_DIR/conflict-repo"
    rm -rf "$dir"
    mkdir -p "$dir" && cd "$dir"
    git init
    git config user.name "Test User"
    git config user.email "test@example.com"

    echo "Line 1" > conflict.txt
    git add conflict.txt && git commit -m "Initial"

    git checkout -b branch-a
    echo "Branch A change" > conflict.txt
    git add conflict.txt && git commit -m "Branch A"

    git checkout main
    git checkout -b branch-b
    echo "Branch B change" > conflict.txt
    git add conflict.txt && git commit -m "Branch B"

    git checkout main
}

# 3. Repo for bisect testing
setup_bisect() {
    local dir="$FIXTURES_DIR/bisect-repo"
    rm -rf "$dir"
    mkdir -p "$dir" && cd "$dir"
    git init
    git config user.name "Test User"
    git config user.email "test@example.com"

    for i in $(seq 1 10); do
        echo "commit $i" > "file$i.txt"
        git add . && git commit -m "Commit $i"
    done
}

echo "Setting up fixture repos..."
setup_simple
setup_conflict
setup_bisect
echo "Done! Fixture repos created in $FIXTURES_DIR"

#!/usr/bin/env bash
# Create deterministic fixture repositories for E2E testing.
#
# Idempotent: deletes and recreates all fixtures on each run.
# Uses fixed GIT_AUTHOR_DATE and GIT_COMMITTER_DATE for reproducibility.
set -euo pipefail

FIXTURES_DIR="$(cd "$(dirname "$0")" && pwd)"

# Common git config for all fixture repos
setup_git_config() {
  git config user.name "Test User"
  git config user.email "test@beardgit.dev"
}

# Fixed timestamp helper (increments per commit for ordering)
TIMESTAMP_BASE=1700000000
COMMIT_NUM=0

make_commit() {
  local msg="$1"
  COMMIT_NUM=$((COMMIT_NUM + 1))
  local ts=$((TIMESTAMP_BASE + COMMIT_NUM * 100))
  export GIT_AUTHOR_DATE="@$ts +0000"
  export GIT_COMMITTER_DATE="@$ts +0000"
  git add -A
  git commit -m "$msg" --allow-empty-message
  unset GIT_AUTHOR_DATE GIT_COMMITTER_DATE
}

# ─── simple-repo ───
# 10 commits, 3 branches (main + 2 feature), 2 tags

echo "Creating simple-repo fixture..."
rm -rf "$FIXTURES_DIR/simple-repo"
mkdir -p "$FIXTURES_DIR/simple-repo"
cd "$FIXTURES_DIR/simple-repo"
git init -b main
setup_git_config
COMMIT_NUM=0

echo "# Simple Repo" > README.md
make_commit "Initial commit"

mkdir -p src
echo "console.log('hello');" > src/main.ts
make_commit "Add main entry point"

echo "body { margin: 0; }" > src/style.css
make_commit "Add styles"

echo "export function add(a, b) { return a + b; }" > src/utils.ts
make_commit "Add utility functions"

echo "import { add } from './utils';" >> src/main.ts
make_commit "Import utils in main"

# Tag the stable point
git tag -a v1.0.0 -m "Release v1.0.0"

echo "# Contributing" > CONTRIBUTING.md
make_commit "Add contributing guide"

echo "node_modules/" > .gitignore
make_commit "Add gitignore"

# Create feature/auth branch
git checkout -b feature/auth
echo "export function login() {}" > src/auth.ts
make_commit "Add auth module"

echo "export function logout() {}" >> src/auth.ts
make_commit "Add logout function"

# Create feature/docs branch from main
git checkout main
git checkout -b feature/docs
echo "# API Docs" > docs.md
make_commit "Add API documentation"

# Back to main for final commit
git checkout main

# Second tag
git tag -a v1.1.0 -m "Release v1.1.0"

echo "  simple-repo: done ($(git log --all --oneline | wc -l | tr -d ' ') commits, $(git branch | wc -l | tr -d ' ') branches, 2 tags)"

# ─── conflict-repo ───
# Two branches with conflicting changes in shared.txt

echo "Creating conflict-repo fixture..."
rm -rf "$FIXTURES_DIR/conflict-repo"
mkdir -p "$FIXTURES_DIR/conflict-repo"
cd "$FIXTURES_DIR/conflict-repo"
git init -b main
setup_git_config
COMMIT_NUM=0

echo "line 1" > shared.txt
make_commit "Initial shared file"

git checkout -b branch-a
echo "line 1 from branch-a" > shared.txt
make_commit "Edit shared file on branch-a"

git checkout main
git checkout -b branch-b
echo "line 1 from branch-b" > shared.txt
make_commit "Edit shared file on branch-b"

git checkout main

echo "  conflict-repo: done (ready for merge conflict testing)"

# ─── bisect-repo ───
# 20 commits. Commit #12 introduces the "bug" (creates a file named "bug.txt").
# Tests can bisect for the commit that introduced bug.txt.

echo "Creating bisect-repo fixture..."
rm -rf "$FIXTURES_DIR/bisect-repo"
mkdir -p "$FIXTURES_DIR/bisect-repo"
cd "$FIXTURES_DIR/bisect-repo"
git init -b main
setup_git_config
COMMIT_NUM=0

for i in $(seq 1 20); do
  echo "content from commit $i" > "file$i.txt"
  if [ "$i" -eq 12 ]; then
    echo "This is the bug" > bug.txt
  fi
  make_commit "Commit $i"
done

echo "  bisect-repo: done (20 commits, bug introduced at commit 12)"

# ─── Summary ───
echo ""
echo "All fixture repos created in $FIXTURES_DIR"
echo "  simple-repo:   10 commits, 3 branches, 2 tags"
echo "  conflict-repo: 3 commits, 3 branches, merge conflict ready"
echo "  bisect-repo:   20 commits, bug at commit 12"

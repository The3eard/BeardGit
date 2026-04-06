# BeardGit

**A fast, polished desktop Git client with GitLab and GitHub CI integration.**

Built with Tauri v2 (Rust) and Svelte 5. Native performance. Cross-platform. Handles 100K+ commits without breaking a sweat.

---

## Features

### Git Operations

- **Visual commit graph** — canvas-based rendering with branch lanes, merge curves, and virtual scrolling. Handles 100K+ commits smoothly.
- **Staging area** — stage and unstage individual files, write commit messages, commit changes.
- **Branch management** — create, rename, delete, checkout, merge, cherry-pick. Folder tree view with context menus and per-branch commit history.
- **Stash management** — push, pop, apply, drop. Per-file apply. Diff preview for any stash entry.
- **Tag management** — paginated list, search, create (annotated or lightweight), delete, push to remote. File-level diff and clickable parent commits in tag detail.
- **Remote operations** — fetch, pull, push run as background tasks with live streaming output.
- **Conflict detection** — detects in-progress merge, rebase, cherry-pick, and revert states. Shows an abort/continue toolbar until the conflict is resolved.
- **Side-by-side diff viewer** — word-level change highlighting, resizable panel, conflict marker support.
- **Clickable ref badges** — merge commit detail shows clickable branch/tag badges that open the diff for that ref directly.

### Graph Intelligence

- **Sync-state-aware lines** — line style encodes state at a glance: thick (pushed to remote), thin (local-only), dashed (fetched but not merged).
- **HEAD branch highlighting** — current branch lane gets a thicker line and a background tint.
- **Author highlighting** — your commits appear in bold, matched against git config email and connected provider accounts.
- **Lane click selection** — click any branch lane to focus it; unrelated lanes dim, keeping context clear.
- **Lane hover feedback** — hover over a lane for a visual highlight before committing to a selection.
- **Clickable parent OIDs** — navigate directly to any parent commit from the detail panel.
- **Theme-ready color system** — all graph colors use semantic tokens via `GraphTheme`; full theme support built in.

### CI Pipeline Integration

- **Multi-provider** — GitLab pipelines and GitHub Actions in the same app, switchable per project.
- **Auto-detect provider** — detects GitLab or GitHub from the git remote URL automatically.
- **Real-time polling** — pipeline list refreshes every 15 seconds; job details every 10 seconds; job logs every 3 seconds.
- **ANSI log viewer** — full 256-color and true-color rendering of CI job output, with configurable theme.
- **Log preprocessing** — strips timestamps and section markers, adds line numbers for readability.
- **Server-side filtering** — filter by branch, status, and source; default filter pre-sets to the current branch on load.
- **Self-hosted support** — GitLab CE/EE and GitHub Enterprise endpoints configurable per project.

### Multi-Project Tabs

- Open multiple repositories simultaneously as tabs.
- Lazy loading — inactive tabs do not consume resources.
- Tab state persists across app restarts.
- Starship-style title bar showing project name, current branch, and a live status summary (ahead/behind counts, staged/unstaged/untracked/conflicted/stash indicators).

### Background Task System

- Async task spawn and cancel with live output streaming.
- Status bar indicator for at-a-glance task health.
- Popover for a quick summary without leaving the current view.
- Expandable full panel with complete task logs and history.
- Fetch, pull, and push automatically trigger graph refresh on completion.

### Authentication and Security

- **AES-256-GCM encrypted credentials** — tokens stored encrypted on disk with a machine-derived key.
- **PAT validation** — validates personal access tokens against GitLab and GitHub before saving.
- **Multi-provider auto-reconnect** — saved providers reconnect automatically on startup.
- **Self-hosted support** — works with custom GitLab and GitHub Enterprise base URLs.

### Internationalization

- English (en-US) and Spanish (es-ES) built in.
- Compile-time typed message functions via Paraglide.js v2 — no runtime key lookups, no missing-string surprises.

### UI and UX

- Custom app icon.
- Fira Code (variable, with ligatures) for all monospace content.
- Symbols Nerd Font Mono for icons throughout the interface.
- Responsive layout using viewport-relative sizing; minimum window size 900x600.
- Shared component library: `SplitView`, `FileChangeList`, `CommitDetail`, `ConfirmDialog`, `ContextMenu`.
- Right-click context menus on branches, tags, and commits.
- Filesystem watcher auto-refreshes statuses and diffs when files change on disk.

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Shell | Tauri v2 |
| Backend | Rust (10 crates), libgit2, SQLite |
| Frontend | Svelte 5, TypeScript, Canvas 2D, Vite |
| i18n | Paraglide.js v2 |
| CI | GitHub Actions |

---

## Architecture

Three-layer design: **Rust Core** (10 crates) → **Tauri IPC** → **Svelte Frontend**

| Crate | Purpose |
|-------|---------|
| `git-engine` | Git operations via libgit2 + bundled git CLI |
| `graph-builder` | Commit DAG construction, lane assignment, virtual scroll viewport |
| `provider` | `CiProvider` trait, unified CI types (`CiRun`, `CiJob`, `CiStatus`), remote URL parser |
| `gitlab-api` | GitLab REST v4 client implementing `CiProvider` |
| `github-api` | GitHub REST API client implementing `CiProvider` |
| `auth` | PAT validation for GitLab and GitHub, AES-256-GCM encrypted credential store |
| `storage` | SQLite via rusqlite, JSON config, TOML theme loader |
| `task-runner` | Background task manager, `TaskEventSink` trait, async spawn/cancel with output streaming |
| `watcher` | Debounced filesystem events via the `notify` crate |
| `app-core` | Tauri command handlers, event emitters, `AppState` |

---

## Installation

Pre-built binaries are available for each tagged release:

| Platform | Architecture | Format |
|----------|-------------|--------|
| macOS | Apple Silicon (arm64) | `.dmg` |
| macOS | Intel (x64) | `.dmg` |
| Linux | x64 | `.AppImage`, `.deb` |
| Windows | x64 | `.msi`, `.exe` |

Download the latest version from [GitHub Releases](https://github.com/The3eard/BeardGit/releases). Pick the installer that matches your platform, run it, and you are good to go.

---

## Building from Source

### Prerequisites

- **Rust** — install via [rustup](https://rustup.rs)
- **Node.js 20+** — install from [nodejs.org](https://nodejs.org)

Platform-specific dependencies:

<details>
<summary>macOS</summary>

```sh
xcode-select --install
```
</details>

<details>
<summary>Linux (Debian / Ubuntu)</summary>

```sh
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```
</details>

<details>
<summary>Linux (Arch)</summary>

```sh
sudo pacman -S --needed webkit2gtk-4.1 base-devel curl wget file \
  openssl appmenu-gtk-module libappindicator-gtk3 librsvg xdotool
```
</details>

<details>
<summary>Linux (Fedora)</summary>

```sh
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel
sudo dnf group install "c-development"
```
</details>

<details>
<summary>Windows</summary>

1. Install [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) and select "Desktop development with C++".
2. Install [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).
</details>

### Build and Run

```sh
git clone git@github.com:The3eard/BeardGit.git
cd BeardGit
npm install
npm run tauri dev
```

First build compiles all Rust crates and takes roughly 3-5 minutes. Subsequent runs are fast.

To build a release binary:

```sh
npm run tauri build
```

---

## Branch Strategy

| Branch | Purpose |
|--------|---------|
| `main` | Stable releases |
| `beta` | Development and beta updates |

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). All contributors must sign a CLA before their changes can be merged.

---

## License

[CC BY-NC-SA 4.0](LICENSE.md) — free for non-commercial use with attribution and share-alike.

---

## Author

Adolfo Fuentes

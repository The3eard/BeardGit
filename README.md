<p align="center">
  <img src="src-tauri/icons/app-icon.svg" alt="BeardGit" width="160" />
</p>

<h1 align="center">BeardGit</h1>

<p align="center">
  <strong>The Git client that keeps up with your repo — and your pull requests, issues, pipelines, and releases.</strong>
</p>

<p align="center">
  Native desktop app for macOS, Linux, and Windows. Built with <a href="https://tauri.app">Tauri&nbsp;2</a> (Rust) and <a href="https://svelte.dev">Svelte&nbsp;5</a>.
  <br />
  Canvas-based graph, AI integration, bundled <code>gh</code> and <code>glab</code>, multi-project tabs — and no Electron.
</p>

<p align="center">
  <a href="https://github.com/The3eard/BeardGit/releases"><img alt="Latest release" src="https://img.shields.io/github/v/release/The3eard/BeardGit?include_prereleases&color=58a6ff&labelColor=0d1117&style=for-the-badge"></a>
  <a href="LICENSE.md"><img alt="License" src="https://img.shields.io/badge/license-CC%20BY--NC--SA%204.0-58a6ff?style=for-the-badge&labelColor=0d1117"></a>
  <a href="https://github.com/The3eard/BeardGit/actions"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/The3eard/BeardGit/ci.yml?branch=main&label=CI&style=for-the-badge&labelColor=0d1117"></a>
  <img alt="Platforms" src="https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-58a6ff?style=for-the-badge&labelColor=0d1117">
</p>

<p align="center">
  <a href="#installation">Install</a> ·
  <a href="#highlights">Highlights</a> ·
  <a href="#building-from-source">Build from source</a> ·
  <a href="#contributing">Contribute</a>
</p>

---

## Why BeardGit

BeardGit is the client I wanted and could never find: **fast like a CLI, rich like a web UI, quiet like a native app**. It pulls the daily workflow into one window — graph, staging, branches, pull requests, issues, pipelines, releases, terminals, and AI assistants — without ever feeling heavy.

- **Canvas graph that scales.** 100K+ commits render smoothly. Branch lanes, merge curves, sync-state lines, and author highlighting, all driven by a viewport-sliced renderer.
- **Forge-native.** Create, edit, merge, approve, and comment on MR/PRs. Manage issues, labels, milestones, assignees. Trigger and retry CI pipelines. Publish releases and upload assets — all without leaving the app.
- **Bundled CLIs.** `gh` and `glab` ship with the installer on every platform. No setup, no PATH dance.
- **Zero-nag AI integration.** Claude Code, Codex, and OpenCode detect automatically. Launch them in a PTY terminal, or let them draft commit messages and review staged changes.
- **Real terminals, in the app.** xterm.js with WebGL, fed by a native Rust PTY. OSC 7 auto-links the terminal to the matching project tab. Composite tabs keep project and shell paired.
- **Honest performance.** Virtual scroll, lazy CodeMirror grammars, xterm instance pool, rayon for graph construction, debounced fs events. No Electron overhead.
- **Secure by default.** PATs stored with AES-256-GCM under a machine-derived key. CLI OAuth supported via `gh` / `glab`.

---

## Highlights

### Git, done right

Visual canvas graph for 100K+ commits. Staging with hunk/line granularity. Branch, tag, stash, and worktree management with context menus everywhere. Three-way merge editor with accept / ignore / undo, rebase (interactive + non-interactive), revert, amend, reset, cherry-pick, and git-bisect with a visual workflow and auto-bisect mode. File history with rename detection, blame with gutter annotations, reflog with recovery actions, clean with preview, patches, submodules — the full toolbox.

### Pull requests, issues, pipelines, releases

A clean `ForgeProvider` abstraction wraps `gh` and `glab` to give you full lifecycle control of MR/PRs (labels, reviewers, draft/ready, reopen, discussion resolution, local checkout), issues (with milestones and assignees), CI/CD actions (trigger, retry, retry-failed-only, per-job retry, cancel), and releases (including asset upload streamed via the task system). Auto-detects the provider from the git remote and works with self-hosted GitLab and GitHub Enterprise.

### AI providers, first class

A provider-neutral AI layer with Claude Code, Codex, and OpenCode built in. Detect installed providers, show their version, let the user pick a default. Generate commit messages, review staged changes, review a PR — all gated on "there's actually something to talk about" so you don't get empty replies. Launch the interactive CLI in a PTY terminal with the right flags, worktree, or session resume.

### Terminals and multi-project tabs

Composite tabs combine a project and its linked terminals (or worktrees) in a single pill. Instant switching via a viewport cache. OSC 7 shell integration so navigating to a project path inside a terminal auto-links that terminal to the matching tab. Foreground process polling detects when `claude` / `codex` / `opencode` start and updates the tab label + brand icon on the fly.

### Observability and quality

Structured file logging via `tracing` with daily rotation and 7-day auto-purge. Tracing spans across every git write and every Tauri command, with sensitive payloads redacted. Trait-crate purity enforced at CI (no runtime deps allowed in contract crates). Full WebdriverIO + tauri-driver E2E suite runnable locally in Docker or on CI via `xvfb`.

---

## Tech stack

| Layer | Stack |
|---|---|
| Shell | Tauri 2 |
| Core | Rust — 17 crates, libgit2, SQLite, `tracing`, `tokio`, `reqwest` |
| Frontend | Svelte 5, TypeScript, Canvas 2D, Vite, Paraglide 2 (i18n) |
| Integrations | `gh` and `glab` (bundled), Claude Code, Codex, OpenCode |
| CI | GitHub Actions — fmt, clippy, tests, svelte-check, vitest, E2E |

### Architecture in one glance

Three layers with strict boundaries. Only `app-core` depends on Tauri — every other crate is a reusable library.

| Crate | Role |
|---|---|
| `git-engine` | Hybrid git — `git2` for reads, system `git` for writes |
| `graph-builder` | Pure DAG construction and lane assignment |
| `forge-provider` | `ForgeProvider` trait + shared forge types (contract-only) |
| `cli-provider` | `GitHubCli` / `GitLabCli` impls of `ForgeProvider` via `gh` / `glab` |
| `provider` | `CiProvider` trait + CI types + shared HTTP helpers |
| `gitlab-api` / `github-api` | REST implementations of `CiProvider` |
| `ai-provider` | `AiProvider` trait + shared AI types |
| `claude-code` / `codex` / `opencode` | `AiProvider` implementations |
| `auth` | AES-256-GCM encrypted credential store with machine-bound key |
| `storage` | SQLite via rusqlite, JSON config, TOML theme loader, logging |
| `task-runner` | Async task manager with streaming output and cancellation |
| `terminal` | PTY session manager via `portable-pty` with OSC 7 integration |
| `watcher` | Debounced filesystem + AI config + sessions watchers |
| `app-core` | 200+ Tauri command handlers, `AppState`, event bridge |

---

## Installation

Pre-built installers are published on every tagged release:

| Platform | Architecture | Format |
|---|---|---|
| macOS | Apple Silicon | `.dmg` |
| Linux | x64 | `.AppImage` |
| Windows | x64 | `.exe` |

> Download the latest version from the [Releases page](https://github.com/The3eard/BeardGit/releases), pick the installer that matches your platform, and run it. `gh` and `glab` are bundled in every installer; no extra setup needed.

### First launch — unsigned builds

BeardGit is currently distributed without Apple or Microsoft code-signing certificates, so both operating systems will flag the app the first time you open it. The app is safe; the warnings exist because the binaries are not notarized/signed. Follow the steps below to allow the app to run — you only need to do this once per install.

<details>
<summary><strong>macOS — "BeardGit is damaged" or "cannot be opened because the developer cannot be verified"</strong></summary>

macOS quarantines downloaded apps that are not signed with an Apple Developer certificate. Pick one of the following:

**Option A — Terminal (fastest).** After dragging BeardGit to `/Applications`, run:

```sh
xattr -dr com.apple.quarantine /Applications/BeardGit.app
```

Then open the app normally.

**Option B — Right-click.** In Finder, right-click `BeardGit.app` → **Open** → click **Open** in the confirmation dialog.

**Option C — System Settings.** Try to open the app once (it will be blocked), then go to **System Settings → Privacy & Security**, scroll to the message _"BeardGit was blocked to protect your Mac"_ and click **Open Anyway**.

</details>

<details>
<summary><strong>Windows — "Windows protected your PC" (SmartScreen)</strong></summary>

Windows SmartScreen warns on executables that are not signed with a Microsoft-recognised code-signing certificate.

When the blue dialog appears:

1. Click **More info**.
2. Click **Run anyway**.

The warning will not reappear for that installer on the same machine.

</details>

---

## Building from source

### Prerequisites

- **Git 2.x+** — required at runtime for write operations (merge, rebase, push, pull, cherry-pick, stash).
- **Rust stable** — install via [rustup](https://rustup.rs).
- **Node.js 22+** — install from [nodejs.org](https://nodejs.org).

<details>
<summary><strong>macOS</strong></summary>

```sh
xcode-select --install
```
</details>

<details>
<summary><strong>Linux (Debian / Ubuntu)</strong></summary>

```sh
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```
</details>

<details>
<summary><strong>Linux (Arch)</strong></summary>

```sh
sudo pacman -S --needed webkit2gtk-4.1 base-devel curl wget file \
  openssl appmenu-gtk-module libappindicator-gtk3 librsvg xdotool
```
</details>

<details>
<summary><strong>Linux (Fedora)</strong></summary>

```sh
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel
sudo dnf group install "c-development"
```
</details>

<details>
<summary><strong>Windows</strong></summary>

1. Install [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) and select **Desktop development with C++**.
2. Install the [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).
</details>

### Build and run

```sh
git clone git@github.com:The3eard/BeardGit.git
cd BeardGit
npm install
npm run tauri dev
```

First build compiles every Rust crate and takes roughly 3–5 minutes. Subsequent runs are fast.

To build a release bundle for your platform:

```sh
npm run tauri build
```

### Running the E2E suite locally

```sh
npm run e2e:docker
```

Requires Docker Desktop (macOS) or dockerd (Linux). The first run builds the image (~10 min). After that, named volumes keep cargo and node_modules warm and iteration is ~1–2 min per cycle. See [`e2e/README.md`](e2e/README.md) for full details.

---

## Branch strategy

| Branch | Purpose |
|---|---|
| `main` | Stable releases |
| `beta` | Development, auto-build matrix |

---

## Contributing

Pull requests welcome. See [CONTRIBUTING.md](CONTRIBUTING.md). All contributors must sign a short CLA before their changes can be merged.

If you find a bug, [open an issue](https://github.com/The3eard/BeardGit/issues). If you're unsure whether something's a bug, a limitation, or an opportunity for a plugin, open it anyway — we'd rather over-triage.

---

## License

[CC BY-NC-SA 4.0](LICENSE.md) — free for non-commercial use with attribution and share-alike. See the license file for the full terms.

---

<p align="center">
  Made with <code>cargo</code>, coffee, and stubbornness by <a href="https://github.com/The3eard">Adolfo Fuentes</a>.
</p>

# Contributing to BeardGit

## Welcome

Thank you for your interest in contributing to BeardGit, a cross-platform desktop git client built with Tauri 2, Rust, and Svelte 5. Whether you are fixing a bug, adding a feature, improving documentation, or reporting an issue, your contribution is valued.

This project is licensed under **CC BY-NC-SA 4.0**. All contributors are required to sign a **Contributor License Agreement (CLA)** before their first pull request can be accepted. See the [CLA](#contributor-license-agreement) section below for details.

---

## Getting Started

### Prerequisites

- **Rust** (stable toolchain) — install via [rustup](https://rustup.rs/).
- **Node.js v22 or later** and npm — install from [nodejs.org](https://nodejs.org).
- **System dependencies for Tauri 2** — see the [Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/) for your platform:
  - **Linux**: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`, and related packages.
  - **macOS**: Xcode Command Line Tools.
  - **Windows**: Microsoft Visual Studio C++ Build Tools, WebView2.

### Clone and Build

```sh
git clone git@github.com:The3eard/BeardGit.git
cd BeardGit
npm install
cargo check --workspace
npm run tauri dev
```

`cargo check --workspace` verifies that the entire Rust workspace compiles. `npm run tauri dev` launches the application in development mode with hot-reload for the Svelte frontend. The first build compiles every Rust crate and takes roughly 3–5 minutes; subsequent runs are fast.

---

## Project Structure

The repository is organized as a Cargo workspace with 18 focused crates, plus the Svelte frontend and Tauri shell. Only `app-core` depends on Tauri — every other crate is a reusable library that can be lifted into a different host (CLI, daemon, alternative UI) without modification.

| Path | Description |
|---|---|
| `crates/git-engine` | Hybrid git operations — `git2` for reads, system `git` for writes |
| `crates/graph-builder` | Pure DAG construction and lane assignment for the canvas graph |
| `crates/forge-provider` | `ForgeProvider` trait + shared forge types (contract-only crate) |
| `crates/cli-provider` | `GitHubCli` / `GitLabCli` implementations of `ForgeProvider` via `gh` / `glab` |
| `crates/provider` | `CiProvider` trait, unified CI types, shared HTTP helpers |
| `crates/gitlab-api` | GitLab REST v4 implementation of `CiProvider` |
| `crates/github-api` | GitHub REST implementation of `CiProvider` |
| `crates/ai-provider` | `AiProvider` trait + shared AI types |
| `crates/claude-code` | `AiProvider` implementation for Claude Code CLI |
| `crates/codex` | `AiProvider` implementation for OpenAI Codex CLI |
| `crates/opencode` | `AiProvider` implementation for OpenCode CLI |
| `crates/auth` | AES-256-GCM encrypted credential store with machine-bound key |
| `crates/storage` | SQLite via rusqlite, JSON config, TOML theme loader, file logging |
| `crates/task-runner` | Async task manager with streaming output and cancellation |
| `crates/terminal` | PTY session manager via `portable-pty` with OSC 7 integration |
| `crates/watcher` | Debounced filesystem + AI config + sessions watchers |
| `crates/mutation-events` | Lightweight event bus for cross-feature notifications |
| `crates/app-core` | 200+ Tauri command handlers, `AppState`, event bridge |
| `src/` | Svelte 5 frontend application |
| `src-tauri/` | Tauri 2 application shell, capabilities, and platform configuration |
| `messages/` | Paraglide source catalogs (`en-US.json`, `es-ES.json`) |

---

## Development Workflow

1. **Branch from `beta`.** All feature and fix work starts on a short-lived branch off `beta`. Use a descriptive prefix that matches what you're doing:

   - `feat/<thing>` — a new user-visible feature.
   - `fix/<thing>` — a bug fix.
   - `chore/<thing>` — maintenance, dependency bumps, CI tweaks.
   - `docs/<thing>` — documentation only.
   - `refactor/<thing>` — internal restructuring without behavior change.

   `main` is the stable mirror; you should never branch directly from it or commit to it.

2. **Use Conventional Commits.** All commit messages must follow the [Conventional Commits](https://www.conventionalcommits.org/) specification. Common prefixes:

   - `feat:` — a new feature.
   - `fix:` — a bug fix.
   - `chore:` — maintenance, dependency updates, CI changes.
   - `docs:` — documentation changes.
   - `refactor:` — code restructuring without behavior changes.
   - `test:` — adding or updating tests.
   - `style:` — formatting / lint-only changes.

3. **Run the full quality bar** before pushing. CI enforces each of these:

   ```sh
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   npx svelte-check
   npx vitest run
   npx stylelint "src/**/*.{svelte,css}"
   npx eslint src
   ```

   The `paraglide` bindings in `src/lib/paraglide/` are auto-compiled by the Vite plugin during `dev`/`build`, but if you edit `messages/*.json` directly, regenerate them with:

   ```sh
   npx @inlang/paraglide-js compile --project ./project.inlang --outdir ./src/lib/paraglide
   ```

4. **Open a Pull Request against `beta`.** Fill out the PR template, describe what your change does and why, and reference any related issues. PRs against `main` are not accepted — `main` only moves forward via integration merges from `beta`.

5. A maintainer will review your PR. Please be responsive to feedback and keep your branch rebased on `beta`.

6. **One feature per branch, merge as it lands.** Don't batch unrelated features on a long-lived branch. Each PR merges to `beta` with `--no-ff` and the branch is deleted (locally and on the remote) as soon as it lands.

---

## Code Style

### Rust

- Format with `cargo fmt --all` before committing. CI rejects unformatted code.
- Resolve all `cargo clippy --workspace --all-targets -- -D warnings` lints. Clippy is enforced in CI.
- Prefer trait-crate purity in the contract crates (`forge-provider`, `provider`, `ai-provider`): no runtime dependencies allowed; CI fails the build if you introduce one.

### TypeScript / Svelte

- Format with **Prettier**. Run `npx prettier --write .` or configure your editor to format on save.
- Theme tokens only — no hardcoded color literals in component CSS or templates. The `eslint-plugin-beardgit/no-hex-in-svelte` rule plus `stylelint` block hex literals everywhere except the four documented sources of truth (`src/lib/stores/theme.ts`, `src/lib/utils/status.ts`, `src/lib/ui/brand-colors.ts`, `src/app.css`). For rare unavoidable cases, use a `// beardgit:allow-hex: <reason>` (or `<!-- beardgit:allow-hex: ... -->`) comment.
- Reuse the shared UI primitives in `src/lib/components/ui/` — `Button`, `IconButton`, `Card`, `Dialog`, `Field`, `FormRow`, `SearchInput`, `CategoryNav`, `SettingSection` — instead of re-styling buttons or panels per component.
- All user-facing strings live in `messages/{en-US,es-ES}.json`; reference them via `m.<key>()` from `$lib/paraglide/messages`. Do not hardcode English strings in templates.
- Follow the existing conventions in the `src/` directory for component structure and naming.

---

## Contributor License Agreement

All contributors must sign the project's Contributor License Agreement before their first pull request can be merged. This ensures that contributions can be distributed under the project's license terms.

Please review and sign the [CLA](CLA.md) before submitting your first PR. If you have questions about the CLA, open an issue or contact a maintainer.

---

## Labels

The project uses a structured label system to organize issues and pull requests. When creating or triaging issues, apply labels from the following categories:

- **`type::`** — classifies the kind of work
  - `type::feature`, `type::bug`, `type::chore`, `type::docs`, `type::refactor`

- **`priority::`** — indicates urgency
  - `priority::critical`, `priority::high`, `priority::medium`, `priority::low`

- **`component::`** — identifies the affected area of the codebase
  - `component::git-engine`, `component::graph-builder`, `component::forge-provider`, `component::cli-provider`, `component::provider`, `component::gitlab-api`, `component::github-api`, `component::ai-provider`, `component::claude-code`, `component::codex`, `component::opencode`, `component::auth`, `component::storage`, `component::task-runner`, `component::terminal`, `component::watcher`, `component::mutation-events`, `component::app-core`, `component::frontend`, `component::i18n`, `component::theme`

- **`status::`** — tracks progress
  - `status::needs-triage`, `status::ready`, `status::in-progress`, `status::needs-review`, `status::blocked`

Apply the most relevant labels when opening issues or pull requests. Maintainers may adjust labels during triage.

---

## Reporting Issues

Found a bug or have a feature request? Please open an issue on the [GitHub issue tracker](https://github.com/The3eard/BeardGit/issues) with the following information:

- **For bugs**: steps to reproduce, expected behavior, actual behavior, platform and OS version, BeardGit version (Settings → Advanced → Current version), and any relevant logs (Settings → Advanced → Open log directory) or screenshots.
- **For feature requests**: a clear description of the desired behavior, the use case it addresses, and any relevant context.

Apply the appropriate `type::` and `component::` labels when creating the issue. If you are unsure which labels to use, apply `status::needs-triage` and a maintainer will categorize it.

---

Thank you for contributing to BeardGit.

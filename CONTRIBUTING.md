# Contributing to BeardGit

## Welcome

Thank you for your interest in contributing to BeardGit, a cross-platform desktop git client built with Tauri v2, Rust, and Svelte. Whether you are fixing a bug, adding a feature, improving documentation, or reporting an issue, your contribution is valued.

This project is licensed under **CC BY-NC-SA 4.0**. All contributors are required to sign a **Contributor License Agreement (CLA)** before their first merge request can be accepted. See the [CLA](#contributor-license-agreement) section below for details.

---

## Getting Started

### Prerequisites

- **Rust** (stable toolchain) — install via [rustup](https://rustup.rs/)
- **Node.js** (v18 or later) and npm
- **System dependencies for Tauri v2** — see the [Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/) for your platform:
  - **Linux**: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`, and related packages
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Microsoft Visual Studio C++ Build Tools, WebView2

### Clone and Build

```sh
git clone git@github.com:The3eard/BeardGit.git
cd BeardGit
npm install
cargo check --workspace
npm run tauri dev
```

`cargo check --workspace` verifies that the entire Rust workspace compiles. `npm run tauri dev` launches the application in development mode with hot-reload for the Svelte frontend.

---

## Project Structure

The repository is organized as a Cargo workspace with several focused crates, plus the Svelte frontend and Tauri shell.

| Path | Description |
|---|---|
| `crates/git-engine` | Core git operations powered by libgit2 with CLI fallback |
| `crates/graph-builder` | Commit DAG construction and visual layout algorithms |
| `crates/provider` | `CiProvider` trait and unified CI types |
| `crates/gitlab-api` | GitLab REST v4 API client |
| `crates/github-api` | GitHub REST API client |
| `crates/auth` | PAT validation and encrypted credential storage |
| `crates/watcher` | Filesystem watcher for detecting repository changes |
| `crates/storage` | SQLite persistence, JSON configuration, TOML theme loading |
| `crates/task-runner` | Background task manager with streaming output |
| `crates/app-core` | Tauri command handlers bridging the frontend to Rust crates |
| `src/` | Svelte 5 frontend application |
| `src-tauri/` | Tauri v2 application shell and platform configuration |

---

## Development Workflow

1. **Branch from `main`.** Create a descriptive branch name, for example `feat/graph-zoom-controls` or `fix/auth-token-refresh`.

2. **Use conventional commits.** All commit messages must follow the [Conventional Commits](https://www.conventionalcommits.org/) specification. Common prefixes:
   - `feat:` — a new feature
   - `fix:` — a bug fix
   - `chore:` — maintenance tasks, dependency updates, CI changes
   - `docs:` — documentation changes
   - `refactor:` — code restructuring without behavior changes
   - `test:` — adding or updating tests

3. **Run the test suite** before pushing:
   ```sh
   cargo test --workspace
   ```

4. **Create a Merge Request** against `main`. Fill out the MR template, describe what your change does and why, and reference any related issues.

5. A maintainer will review your MR. Please be responsive to feedback and keep your branch up to date with `main`.

---

## Code Style

### Rust

- Format your code with `cargo fmt` before committing. The CI pipeline will reject unformatted code.
- Run `cargo clippy --workspace` and address all warnings. Clippy lints are enforced in CI.

### TypeScript / Svelte

- Format with **Prettier**. Run `npx prettier --write .` or configure your editor to format on save.
- Follow the existing conventions in the `src/` directory for component structure and naming.

---

## Contributor License Agreement

All contributors must sign the project's Contributor License Agreement before their first merge request can be merged. This ensures that contributions can be distributed under the project's license terms.

Please review and sign the [CLA](CLA.md) before submitting your first MR. If you have questions about the CLA, open an issue or contact a maintainer.

---

## Labels

The project uses a structured label system to organize issues and merge requests. When creating or triaging issues, apply labels from the following categories:

- **`type::`** — classifies the kind of work
  - `type::feature`, `type::bug`, `type::chore`, `type::docs`, `type::refactor`

- **`priority::`** — indicates urgency
  - `priority::critical`, `priority::high`, `priority::medium`, `priority::low`

- **`component::`** — identifies the affected area of the codebase
  - `component::git-engine`, `component::graph-builder`, `component::gitlab-api`, `component::task-runner`, `component::auth`, `component::watcher`, `component::storage`, `component::app-core`, `component::frontend`

- **`status::`** — tracks progress
  - `status::needs-triage`, `status::ready`, `status::in-progress`, `status::needs-review`, `status::blocked`

Apply the most relevant labels when opening issues or merge requests. Maintainers may adjust labels during triage.

---

## Reporting Issues

Found a bug or have a feature request? Please open an issue on the [GitHub issue tracker](https://github.com/The3eard/BeardGit/issues) with the following information:

- **For bugs**: steps to reproduce, expected behavior, actual behavior, platform and OS version, and any relevant logs or screenshots.
- **For feature requests**: a clear description of the desired behavior, the use case it addresses, and any relevant context.

Apply the appropriate `type::` and `component::` labels when creating the issue. If you are unsure which labels to use, apply `status::needs-triage` and a maintainer will categorize it.

---

Thank you for contributing to BeardGit.

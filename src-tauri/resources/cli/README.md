# CLI Binary Bundling

This directory is the target for bundled `gh` (GitHub CLI) and `glab` (GitLab CLI) binaries.

## How it works

In CI (build.yml and release.yml), the `scripts/download-cli-tools.sh` (Unix) or
`scripts/download-cli-tools.ps1` (Windows) script downloads the correct platform
binaries before the Tauri build step.

The binaries are registered in `tauri.conf.json` under `bundle.resources` so Tauri
includes them in the app bundle. At runtime, `resolve_cli_binary()` in
`crates/app-core/src/commands.rs` looks for them next to the executable first, then
falls back to the system PATH.

## Local development

During local development, `gh` and `glab` are resolved from your system PATH.
You do NOT need to download binaries into this directory for dev mode.

Install them via your package manager:

```bash
# macOS
brew install gh glab

# Linux (Debian/Ubuntu)
sudo apt install gh glab

# Windows
winget install GitHub.cli GitLab.Glab
```

## CI download scripts

- `scripts/download-cli-tools.sh` — Unix (macOS, Linux)
- `scripts/download-cli-tools.ps1` — Windows

These scripts download specific versions of gh and glab for the target platform
and place the binaries in `src-tauri/binaries/` for the Tauri build.

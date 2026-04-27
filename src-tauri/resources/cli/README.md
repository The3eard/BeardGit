# CLI Binary Bundling

BeardGit bundles `gh` (GitHub CLI) and `glab` (GitLab CLI) as Tauri sidecars.

## How it works

1. Versions are pinned in `cli-versions.json` at the repo root
2. `scripts/download-cli-binaries.js` downloads platform-specific binaries
3. Binaries are placed in `src-tauri/binaries/` with Tauri sidecar naming:
   - `gh-{target_triple}` (e.g. `gh-aarch64-apple-darwin`)
   - `glab-{target_triple}` (e.g. `glab-aarch64-apple-darwin`)
4. `tauri.conf.json` declares them under `bundle.externalBin`
5. At runtime, `resolve_cli_binary()` checks sidecar paths first, then system PATH

## CI

Both `build.yml` and `release.yml` run the download script before `tauri build`:

```
node scripts/download-cli-binaries.js --target ${{ matrix.target-triple }}
```

## Local development

During local development, `gh` and `glab` are resolved from your system PATH.
You do NOT need to download binaries into `src-tauri/binaries/` for dev mode.

To test the full bundled-sidecar resolution locally:

```bash
node scripts/download-cli-binaries.js
npm run tauri dev
```

Install via your package manager for PATH-based development:

```bash
# macOS
brew install gh glab

# Linux (Debian/Ubuntu)
sudo apt install gh glab

# Windows
winget install GitHub.cli GitLab.Glab
```

## Updating CLI versions

Edit `cli-versions.json` and push. CI downloads the new versions automatically.
No runtime auto-update — CLIs update with app releases.

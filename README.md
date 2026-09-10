<p align="right">
  <strong>English</strong> · <a href="README.es.md">Español</a>
</p>

<p align="center">
  <img src="docs/assets/og-github.png" alt="BeardGit — one window for your whole repo" />
</p>

<h1 align="center">BeardGit</h1>

<p align="center">
  <strong>One window for your whole repo.</strong>
  <br />
  The graph, your pull and merge requests, CI, issues, releases, terminals, AI runs, your <code>.http</code> requests and a file editor — in the same desktop app, so shipping a change stops being a tour of four of them.
  <br />
  Built on Tauri, not Electron. No account, no telemetry. macOS · Linux · Windows.
</p>

<p align="center">
  <a href="https://github.com/The3eard/BeardGit/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/The3eard/BeardGit?include_prereleases&color=d9924f&labelColor=151312&style=for-the-badge"></a>
  <a href="LICENSE.md"><img alt="License" src="https://img.shields.io/badge/license-CC%20BY--NC--SA%204.0-8a7f74?style=for-the-badge&labelColor=151312"></a>
  <a href="https://github.com/The3eard/BeardGit/actions"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/The3eard/BeardGit/ci.yml?branch=main&label=CI&style=for-the-badge&labelColor=151312"></a>
  <img alt="Platforms" src="https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-8a7f74?style=for-the-badge&labelColor=151312">
  <img alt="Themes" src="https://img.shields.io/badge/themes-31%20built--in-8a7f74?style=for-the-badge&labelColor=151312">
</p>

<p align="center">
  <a href="https://github.com/The3eard/BeardGit/releases/latest"><strong>Download ↓</strong></a>
  &nbsp;·&nbsp;
  <a href="https://the3eard.github.io/BeardGit/">Website</a>
  &nbsp;·&nbsp;
  <a href="https://the3eard.github.io/BeardGit/features/">Features</a>
  &nbsp;·&nbsp;
  <a href="https://the3eard.github.io/BeardGit/guide/">Guide</a>
  &nbsp;·&nbsp;
  <a href="CHANGELOG.md">Changelog</a>
</p>

---

## Why it exists

I kept the graph in one app, the merge request in a browser tab, the pipeline in another, and the API call I wanted to test in a fourth. None of those tools was bad — the jumping was. So BeardGit puts the things I touch to ship a change in one window. It isn't out to be the last Git client you install; plenty of them are older, bigger and better funded. It's the one where the next step is usually already on screen.

<p align="center">
  <img src="docs/assets/screenshots/graph-dark.png" alt="BeardGit's canvas commit graph with branch lanes, merge curves and the commit detail pane" width="100%" />
</p>

## What's in the window

- **A commit graph on canvas.** Six-figure histories scroll smoothly, because only the visible rows are drawn. Branch lanes, merge curves, ref badges by kind, author highlighting, and a search (`⌘F` or `/`) that re-lays out around the matches.
- **Staging down to the line.** Collapse hunks, expand a file to its full contents, stage or discard individual lines, Shift-select a range, keyboard through the list. Commits honour your `commit.gpgsign` setup — SSH, GPG or X.509 — and Settings has a *Test signing* button that shows the real error.
- **GitHub *and* GitLab, both properly.** Pull requests and merge requests with per-file diffs and inline review threads; issues, labels, milestones, pipelines, releases, repo settings. Self-hosted GitHub Enterprise and on-prem GitLab included, with auth checked per host so a VPN-only forge can't shadow a working one.
- **Branches as a folder tree, with stars.** Star the branches you actually work on and they rise to the top of their own level. A star belongs to the branch, not the ref, and lives in `.beardgit/favorites.json`. Plus branch cleanup for `[gone]` upstreams, tags, stashes, worktrees, nested submodules, reflog with recovery actions, and compare-any-two-refs.
- **AI that runs in a worktree.** Your own install of Claude Code, Codex or OpenCode, started on an `ai/<provider>/<slug>` branch in an isolated worktree, queued at a cap you set. Read the transcript, then merge, keep or discard. Your checkout never moves.
- **An `.http` workspace in the repo.** Plain files under `.beardgit/requests/`, committed with the code that calls them. Environments are commit-safe JSON; secrets are names in the file and encrypted values on your machine. Response history with a diff between any two runs.
- **A file editor for the two-line fix.** CodeMirror 6 with per-language snippets, JSON linting, inline colour pickers and a gitignore-aware tree. Save writes to disk; save with Shift also stages it.
- **Real terminals.** xterm.js over a native Rust PTY, with `TERM`, truecolor and a UTF-8 locale set properly, and a login shell on macOS so your `PATH` is the one you expect. OSC 7 links a terminal to its project tab.
- **31 themes, or write your own.** Three original families plus the classics. The theme reaches the editor's syntax colours, the diff backgrounds and the graph lanes. Yours is a TOML file with 18 colours; the rest is derived. Every bundled theme clears the WCAG AA contrast floor, enforced by a test.
- **A command palette.** `⌘⇧P` lists every view and every registered shortcut and runs it — the fastest way to learn the keyboard. `?` opens the full cheat sheet, generated from the same registry.
- **Multi-repo tabs that stay cheap.** Heavy state loads only for the active tab, and every section remembers its filters, scroll, pane widths and half-written drafts for the length of the session.
- **Two switches for the whole integration surface.** Settings → General → Integrations: one for GitHub/GitLab, one for AI. Off, the surfaces disappear and nothing runs behind them — no token validated at launch, no remote resolved against a forge API, no CLI probing. Both off leaves a git-only client whose only outbound request of its own is the update check, which has its own toggle.

The long version, view by view, is on the [features page](https://the3eard.github.io/BeardGit/features/).

## Install

| Platform | Build | Needs |
| --- | --- | --- |
| macOS | Apple Silicon · `.dmg` | Nothing — WKWebView ships with the OS |
| Linux | x64 · `.AppImage` | `libwebkit2gtk-4.1` |
| Windows | x64 · `.exe` | WebView2 Runtime (already on Windows 11) |

> **[→ Download the latest release](https://github.com/The3eard/BeardGit/releases/latest)**. `gh` and `glab` are bundled, so there is nothing else to install. You do need `git` on your `PATH` — every write goes through it.

<details>
<summary><strong>Getting past the first-launch warning</strong></summary>

The builds aren't code-signed, so macOS and Windows stop them once. Nothing is wrong with the download.

**macOS** — drag the app to `/Applications`, then either right-click → **Open**, or clear the quarantine flag:

```sh
xattr -dr com.apple.quarantine /Applications/BeardGit.app
```

**Windows** — SmartScreen → **More info** → **Run anyway**. It won't ask again on that machine.

**Linux** — `chmod +x BeardGit-*.AppImage && ./BeardGit-*.AppImage`

</details>

## Whether it fits you

**You'll probably like it if you** work on GitHub *or* GitLab and want both treated properly; test APIs against the repo you're looking at; run an AI CLI and want it in a worktree rather than loose in your checkout; prefer a native app to another Chromium runtime; or use Linux and are tired of being the afterthought platform.

**You probably won't if you** are happy in `git` and `lazygit`; need SVN, Mercurial, Perforce or Bitbucket; want a signed installer today, an SLA or a support line; or are on an Intel Mac.

## Rough edges, up front

- **The builds aren't signed.** One command on macOS, two clicks on Windows, once per install. Certificates cost money per year and this is a one-person project.
- **Three platforms, three formats.** No Intel Mac build, and no `.deb`, `.rpm` or `.msi`. Building from source covers the rest.
- **Two forges, not five.** Bitbucket, Gitea and Codeberg aren't supported — the provider layer is abstracted, but each still needs a real driver and tests.
- **`git` has to be on your `PATH`,** and the app doesn't check at startup: if it's missing, a commit fails rather than the launch.
- **One request per `.http` file, for now.** The parser understands several blocks; the panel runs the first one. It also won't reach `localhost` unless you start the app with `BEARDGIT_REQUESTS_ALLOW_PRIVATE=1`.
- **One developer, in the open.** Bugs get fixed when they're reported, and [the changelog](CHANGELOG.md) says which and why. If you need a maintained-by-committee product, Fork, Tower, GitKraken and lazygit are all good.

## Documentation

| | |
| --- | --- |
| [Guide](https://the3eard.github.io/BeardGit/guide/) | Install, connect a forge, set up an AI CLI, write `.http` requests and your own theme, the full keyboard table, where your data lives, troubleshooting |
| [Features](https://the3eard.github.io/BeardGit/features/) | Every view, one by one |
| [Changelog](CHANGELOG.md) | What changed in each release, and why |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Crate layout, branch strategy, the checks a change has to pass |
| [SECURITY.md](SECURITY.md) | Disclosure policy |

## Building from source

You need Rust (the version is pinned in `rust-toolchain.toml`), Node 22 and git.

```sh
git clone https://github.com/The3eard/BeardGit.git
cd BeardGit
npm install
npm run tauri dev
```

The first build compiles every Rust crate and takes a few minutes; after that it's quick. `npm run tauri build` produces the installer for your platform. Platform prerequisites are in the [guide](https://the3eard.github.io/BeardGit/guide/#build), and the layout of the workspace is in [CONTRIBUTING.md](CONTRIBUTING.md).

## Contributing

Pull requests are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md). Contributors sign a short CLA before their changes can be merged.

Found a bug? [Open an issue](https://github.com/The3eard/BeardGit/issues) with your OS, the version from Help → About, and what you did. If you're not sure whether it's a bug or a limitation, open it anyway — sorting that out is my job.

## License

[CC BY-NC-SA 4.0](LICENSE.md). Free to use, including at work on work code. The non-commercial clause is defensive: it stops someone repackaging and selling BeardGit itself.

---

<p align="center">
  <sub>If BeardGit saves you a tab, a ⭐ on the repo is how other people find it.</sub>
</p>

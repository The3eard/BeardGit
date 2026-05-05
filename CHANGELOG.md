# Changelog

All notable changes to BeardGit are documented here. Format follows [keepachangelog.com](https://keepachangelog.com).

## [0.1.12] — In-app mini editor + per-file discard — 2026-05-05

### Editor — edit repo files without leaving BeardGit

A new "Editor" sidebar entry (between Changes and Branches) opens a split-pane mini editor: file tree on the left, tabbed CodeMirror buffer on the right. The whole point is to close the loop on the most common "I see a diff → I want to fix it" workflow without bouncing out to an external editor. Every other surface that lists workdir files — Changes, the per-file context menu on Branches' commit detail, and Reflog detail — gets a new **Open in editor** item that switches the active view and opens the path as a new tab. PR/MR and graph commit-detail file lists stay diff-only on purpose, since those files are at-commit, not workdir; opening them would silently swap content the user expected to see.

The editor is built on the same CodeMirror 6 stack as the diff / merge views, but with a one-way contract that hard-isolates its lifecycle from prop reactivity — typing into the buffer never re-runs the init effect, never tears down the live `EditorView`, and never loses focus mid-keystroke. The parent owns external content swaps (file load, reload) by bumping a `loadVersion` counter that threads into a `revisionId` prop; the editor swallows fresh content exactly when an external write happens. Every other prop change (theme, extensions, even a new `onChange` closure on every parent re-render) is invisible to the editor's lifecycle, which uses `onMount` / `onDestroy` rather than reactive effects for mount + tear-down.

The tree and tabs are bookended by a polished editing UX: `EditorTabs` shows a dirty `●` and an external-change `⚠` indicator per tab, supports middle-click close, and routes dirty-tab close attempts through a `ConfirmDialog`. `EditorToolbar` exposes a Save button that morphs into "Save and stage" in real time while you hold Shift — a `window` keydown / keyup pair listens at the global scope (with a `blur` reset for the held-modifier-while-switching-app case) so the affordance is visible without having to read a tooltip. Mod+S / Mod+Shift+S keymaps reach the same `saveActive` helper. An external-change banner offers Reload / Keep-my-version actions when the watcher reports a workdir mutation that touches an open buffer.

The `FileTreeView` itself is a stateful `PathTree` with a `SearchInput` filter, a reload button, and a context menu (Open / Rename / Delete / New file here / New folder here / Copy path) hooked into a single combined `PathDialog` for the create / rename flows with Windows-illegal-character + path-traversal validation. The tree is gitignore-aware behind a Settings toggle (off by default — gitignored files stay visible so users can edit untracked or build-output files; the description spells this out). Folder / file glyphs are wired through a Nerd Font map (`file-icons.ts`) keyed by basename and extension covering ~50 file types from the bundled Symbols Nerd Font Mono set: Rust, TS / JS / TSX / JSX, Python, Go, Java / Kotlin, Swift, C / C++ / C#, Ruby, PHP, Svelte, Vue, HTML, CSS / SCSS, JSON, YAML, TOML, XML, Markdown, txt / rst, shell scripts, SQL, images / video / audio, archives, lock files, env files, plus special-cased basenames (`Cargo.toml`, `Cargo.lock`, `package.json`, `Dockerfile`, `Makefile`, `tsconfig.json`, `.gitignore`, `README*`, `LICENSE*`, `svelte.config.js`, `vite.config.*`).

A new sidebar **Editor** category in Settings hosts the full preference surface: ten toggleable CodeMirror extensions (autocomplete, close brackets, bracket matching, highlight active line, highlight selection matches, fold gutter, indent on input, line wrapping, rectangular selection, crosshair cursor) plus four behaviour fields (tab size, indent with tabs vs. spaces, respect-`.gitignore`-in-tree, large-file warning threshold). A second **Smart editing** section adds five further toggles for the heavier helpers — code snippets, keyword completion, JSON lint, inline color picker, and indent guides — all of them per-language, none of them require an LSP. Snippet packs cover the bread-and-butter patterns of Rust (`fn`, `impl`, `match`, `struct`, `enum`, `trait`, `derive`, `Result`, `Option`, `println!`, `dbg!`, …), TypeScript / JavaScript (`fn`, `arrow`, `class`, `interface`, `for`, `if`, `import`, `export`, `try`), Python (`def`, `class`, `for`, `if`, `try`, `with`), and Go (`func`, `if`, `for`, `struct`, `interface`, `package`, `defer`); keyword completion adds reserved-word suggestions for the same set plus C, C++, Java, and CSS. JSON lint hangs off `@codemirror/lint` with native `JSON.parse` round-trip plus curated rules for `package.json` (requires `name` + `version`), `tsconfig.json` (requires `compilerOptions` to be an object), and `.beardgit/requests/_env/*.json` (requires `vars` + `secrets`); no AJV. The color picker is `colorPicker` from `@replit/codemirror-css-color-picker`, the indent guides come from `@replit/codemirror-indentation-markers`, and the active-line gutter highlight piggybacks on the existing `highlight_active_line` pref via `highlightActiveLineGutter()` from `@codemirror/view`.

The autocomplete popup gets actual suggestions thanks to a global `completeAnyWord` source registered as a `languageData` entry; the language packs we ship for Rust / Python / Go / Java / etc. don't contribute completion data themselves, so without this the popup never opened on those buffers. HTML and CSS keep their built-in completion sources because language data merges instead of overriding.

The legacy-mode coverage of the editor's language pack picker grew via `@codemirror/legacy-modes`: TOML, Dockerfile, Makefile (recipe lines reuse the shell mode as the closest approximation), INI / Properties, Lua, Perl, R, and nginx configs all light up syntax highlighting now. `language-support.ts` does a basename-first lookup before the extension fallback, so `Dockerfile` / `Makefile` / `GNUmakefile` resolve correctly without a dotted suffix.

Backend (Rust): a new `crates/git-engine` workdir-CRUD module (`write_file_workdir`, `list_workdir_tree`, `create_workdir_path`, `rename_workdir_path`, `delete_workdir_path`) with a shared lexical path validator that refuses absolute paths, `..` segments, and anything that resolves outside the working tree. Reads cap at 2 MB (`read_workdir_file` returns a tagged `too_large` shape rather than slurping the bytes) and detect binaries with an 8 KB NUL sniff. All mutating commands run inside `with_mutation_guard(MutationKind::StagingChange)` so the watcher fan-out fires once on success and the Changes panel refreshes for free. `EditorPreferences` (in `crates/storage`) is the persisted struct backing all the toggles; existing `settings.json` files migrate transparently because every field uses `serde(default)`. The default sidebar nav order grows by one slot.

Per-project tab persistence: the open-tabs list (paths only, no buffer contents) is round-tripped through localStorage on project switch, so reopening the project this session — or after a restart — restores the same set. The `fileEditor` store subscribes to `project-mutated` so external file edits flag every non-dirty open tab as `externalChange: true`; the user reloads with the toolbar banner or clicks "Keep my version" to dismiss.

`PathTree` (the existing component shared with PR / MR diffs) gains an opt-in `showIcons` + `fileIconResolver` prop so the file-editor tree gets the rich Nerd-Font glyph treatment while the diff lists keep their compact look. Files / folders sort directories-first then alpha-by-name, and nested `<ul>`s reset their list-style so the browser's default disc bullets don't leak through into the rendered tree.

Sixty new i18n keys in en-US + es-ES (twenty-two settings, thirty-eight editor); paraglide bindings regenerated. New TS dependencies: `@codemirror/search@^6` (the find / replace panel + selection-matches highlight + search keymap), `@codemirror/legacy-modes@^6`, `@codemirror/lint@^6`, `@replit/codemirror-css-color-picker@^6`, `@replit/codemirror-indentation-markers@^6`. Plus a fresh wave of tests: 16 fileEditor store tests, 5 editorPrefs store tests, 9 wrapper / spec tests for the new IPC surface, 3 storage round-trip tests for `AppConfig.editor_preferences`, and 24 cases across `keywords.test.ts`, `snippets.test.ts`, `json-lint.test.ts`, and `file-icons.test.ts`.

### Discard unstaged changes per file

`feat(changes): discard unstaged changes per file`. The Changes panel's per-row context menu now has an explicit **Discard changes** entry next to the existing stage / unstage actions, alongside a confirmation dialog so a misclick can't blow away a long edit. Maps to `git checkout -- <path>` under the hood and routes through `with_mutation_guard` so the staging area refreshes the moment the workdir reverts. Previously the only way to drop unstaged work for a single file was the Clean panel or a terminal — neither of which was the obvious move on a dirty file row.

### Patches generated by BeardGit are now valid for users with a custom `diff.external`

`fix(cli,tests): pass --no-ext-diff to programmatic git diff + plug stale tests`. Users with a global `diff.external` configured (e.g. [`difftastic`](https://difftastic.wilfred.me.uk/)) were silently producing non-applicable patch text from the "Create patch" command, malformed commit-stat numbers in the graph and on PR/MR pages, and AI prompts fed pretty-printed diff output instead of the canonical unified diff that the model actually expects. Every programmatic `git diff` shell-out now passes `--no-ext-diff` so the canonical unified diff is what comes back regardless of the user's config. Touches `git-engine::patch::create_working_tree_patch`, `git-engine::diff::commit_file_diff`, `git-engine::cli::commit_stats`, and `app_core::ai_commands::get_staged_diff_text`.

While there: three pre-existing tests on `beta` that were red on a freshly-cloned machine are now green. `requests_list_project` no longer materialises the `.beardgit/` directory chain on a project that hasn't opted into the Requests feature (the explicit seeding command remains the canonical path); `TabBar.svelte` defensively guards `$aiProviders.length` so a vitest e2e teardown race no longer surfaces an unhandled `TypeError`; the `BackgroundRunTranscript` copy test queries the rendered `<button>` instead of a `.btn-copy` class that never existed in the IconButton primitive.

## [0.1.11] — Requests panel + brand mark refresh — 2026-05-05

### Requests panel — `.http` API testing inside the repo

A new sidebar entry hosts a native HTTP request workspace that follows the same shareable-by-git philosophy as the rest of BeardGit. Project requests live under `<project>/.beardgit/requests/` as plain `.http` files (REST Client / IntelliJ HTTP Client format), so a `git pull` is enough for the whole team to share them; folders nest arbitrarily and the tree renders recursively. Environments live as sidecar JSON files under `_env/<name>.json` for non-sensitive variables, with secrets kept out of the repo and stored encrypted in BeardGit's local credential store via a new `requests-env://<env>/<name>` namespace on top of the existing `auth::CredentialStore`. The `default` env is always present — both `requests_list_project` and `requests_get_envs` lazily recreate `_env/default.json` when missing, and the env switcher dropdown has no "no env" option, so you can never end up working without one.

The editor pane uses CodeMirror 6 with the canonical `createCodemirrorTheme` so the body editor matches the rest of the app's code panes, JSON syntax highlighting, and `{{var}}` autocomplete that fires on the trailing `{{` and pulls suggestions from the active env's vars + secret names. The URL bar is a single-line CodeMirror micro-editor (`MiniCodeInput`) that gets the same autocomplete treatment, and autocomplete tooltips render with `position: fixed` against `document.body` so the popover never gets clipped behind the response tabs. The response viewer's Pretty mode is a read-only CodeMirror surface so you actually see syntax-highlighted JSON responses, not just a `<pre>` clone of Raw mode.

Send / Cancel is a real toggle backed by a `tokio_util::sync::CancellationToken` registered in `AppState.requests_cancellations` and addressed from a frontend-generated `crypto.randomUUID()` ticket id, so clicking Cancel actually aborts the in-flight `reqwest` future — not just the UI label. Run results persist into a new `requests-store` crate (its own SQLite file, `requests.db`, separate from the main app DB) with a 50-row history cap per request and a 5 MB body cap (truncated with a banner pointing at "Save raw to file…"). The response viewer's History tab reuses the existing CodeMirror merge view to diff any two responses by checkbox-selecting them. Includes Copy-as-cURL / fetch / HTTPie / wget code generators (rendered as a canonical dropdown menu matching `AddProjectMenu`'s look-and-feel), a Paste-from-cURL importer, right-click context menu on tree leaves (Copy as cURL, Duplicate, Rename inline, Open in editor via a backend `requests_open_in_editor` command that bypasses the `tauri-plugin-opener` allowlist, Delete with `ConfirmDialog`), and a "+ New request" affordance per section that surfaces in two places: the in-tree button and a primary-CTA in the empty-state seed prompt, both wired to a shared `newRequestOpen` writable so they open the exact same dialog.

A "Load test set" secondary action seeds nine `.http` examples against the public **JSONPlaceholder** (REST CRUD) and **httpbin.org** (request inspection, status codes, slow responses for testing Cancel) APIs, plus a default env wiring `base_url` / `httpbin_base_url` / `post_id`. Picking it lands you on `quickstart/jsonplaceholder/list-posts.http` ready to hit Send. The first save bumps `treeReloadSignal`, the watcher polls `.beardgit/requests/` for external mutations, and the env switcher refreshes off the same signal so seeded envs appear in the dropdown without a panel remount.

Visually the panel uses the shared design-system primitives end to end (`Button`, `IconButton`, `Field`, `Card`, `Dialog`, `List`, `TwoLineRow`, `ContextMenu`, `ConfirmDialog`); zero hardcoded colors, every accent goes through `var(--accent-*)` and `color-mix(...)` so the panel theme-tracks light/dark and any custom theme like the rest of the app. Method dropdown + URL field are normalised to the same height and the same `var(--font-mono)` so the row aligns cleanly. Verb badges (GET / POST / PUT / PATCH / DELETE) appear next to each leaf in the collections tree using the canonical tonal-on-accent recipe (`color-mix(--accent-blue 18%, transparent)` etc.).

EnvManager (opened via the **Manage** button next to the env switcher) lets you create / edit / delete envs, set encrypted secret values via a `SecretPrompt` modal, and shows a `(N vars, M secrets)` summary per env in the dropdown so you see at a glance whether an env has content. Save closes the dialog after a successful write, and Delete prompts a `ConfirmDialog` *before* removing the file (was: deleted-then-asked).

### Brand mark refresh

`chore(icons): refine logo design` + `fix(welcome): sync welcome-screen logo with redesigned brand mark`. The app icon got a refresh and the welcome screen's hero glyph was re-pointed at the redesigned source asset so it stops drifting from the title-bar / tray icon set.

`fix(welcome): show BeardGit logo instead of generic icon glyph`. The welcome screen used to fall back to a generic icon when no project was open; now it surfaces the actual BeardGit brand mark, matching the rest of the empty-state messaging.

### Tab + welcome polish

`fix(tabs): clear repo state when closing the last tab`. Closing the only open project tab used to leave stale repo state in memory — the next "Open folder" could pick up the previous repo's HEAD or change-count for a brief flash. The tab close handler now wipes the active-project state when `projects.length` hits zero, so the welcome screen takes over cleanly.

### Landing site

`chore(docs/landing): add Metricool tracker`. The marketing/landing site under `docs/` now ships the standard Metricool analytics snippet, the same one the rest of metricool.com uses, so we can tell whether the landing actually drives sign-ups. No telemetry was added to the desktop app — `beardgit` itself remains telemetry-free.

## [0.1.10] — AI tasks in the drawer + persisted reviews, landing & community polish — 2026-04-28

### AI code review surfaces in the tasks drawer + persists to disk

`feat(ai): TaskKind::AiHeadless + save_ai_review`. Every headless AI command — Code Review, Generate commit message, Analyze, PR description, PR review — is now spawned with a new `TaskKind::AiHeadless` runtime variant and surfaced in the unified tasks drawer (statusbar Tasks slot + popover) alongside git ops and AI background runs. The tasks were previously spawned as `TaskKind::Generic`, which `TaskManager::should_emit` filters out, so they ran invisibly with output reachable only via the legacy detail panel. Adding `AiHeadless` to both `kind_from_runtime` and the `should_emit` allowlist makes the rows appear with the lightbulb glyph and a Cancel action while running.

`feat(ai): persist code review to <project>/.beardgit/reviews/`. When a Code Review task completes, the cleaned ANSI-stripped output is written to a new `<project>/.beardgit/reviews/review-YYYY-MM-DD-HHMMSS-<short-head>.md` file (a fresh `save_ai_review` Tauri command in `app-core::ai_commands`, backed by `git2::Repository::head` for the short oid). A 10-second success toast surfaces the relative path with an **Open** action that calls `openPath` (now allowed via `opener:allow-open-path` in capabilities) and falls back to `revealItemInDir` if the OS has no default markdown handler. The saved-file path is mirrored onto the task entry's subtitle via a new `setTaskSubtitle` helper backed by a `subtitleOverrides` map in `tasks.ts`, so the drawer's detail panel surfaces it under "Context" even after a late `task://update` upsert would otherwise have clobbered the field.

### Code review + Commit-message buttons gated on staged changes

`feat(staging): disable Code Review when nothing is staged`. The Code Review icon button in the Changes toolbar is disabled when `staged.length === 0` and its tooltip swaps to *"You need to stage changes to get a review"*. Same gate applies to the AI commit-message button. Both backends already analysed only the staged diff (`git diff --cached`), so the disabled state matches what the AI would actually see. Drops the dead `create_review_patch` Tauri command + `git_engine::create_review_patch` helper that briefly tried a HEAD-vs-worktree shape — staged-only is the right semantic.

### Tasks drawer polish

`fix(tasks): per-row Dismiss removes only that entry`. The Dismiss action on a single task row used to call `clearFinished()`, which wipes every finished task in the drawer — including ones the user wanted to keep. A new `removeTask(id)` helper trims a single entry, and the popover's `handleAction` routes per-row Dismiss through it. The header's "Clear" button still does the bulk wipe.

`fix(tasks): action-button clicks no longer also open the detail view`. Clicking Cancel / Dismiss / Retry on a row used to fire the action AND open the detail panel because the click bubbled up to the row's `onclick={openDetail}`. A `<div class="task-row__actions" onclick={(e) => e.stopPropagation()}>` wrapper stops the bubble at the actions container; the action's own onclick still fires.

`fix(tasks): each_key_duplicate when output has blank lines`. The `{#each outputLines}` block in `TaskDetailPanel` keyed entries by `${stream}:${text}`, which collides on blank lines (`stdout:` × N) and made Svelte refuse to render the entire `<pre>` — empty detail panels for any AI review whose Markdown body had paragraph breaks. Switched the key to `${idx}:${stream}`.

`fix(tasks): Dismiss / Cancel buttons now have visible hover state`. Scoped CSS in `TaskEntryRow` lifts the neutral-button hover to `color-mix(--text-primary 12%, --bg-secondary)` inside `.task-row__actions` so the affordance reads cleanly in both themes.

`feat(tasks): bulb glyph for AI rows`. AI task rows in the drawer now use `` (fa-lightbulb-o), matching the AI commit-message button in the Changes toolbar.

### Diff: show whitespace toggle

`feat(settings): "Show whitespace in diffs" toggle`. New entry under **Settings → General → Diff display** that renders spaces as `·` and tabs as `→` in the side-by-side diff viewer (CodeMirror `highlightWhitespace` extension). Default off so unchanged diffs stay clean. Persists to `AppConfig::diff_show_whitespace` and re-renders any open `DiffEditor` instance immediately on toggle.

### Tauri Build workflow → manual-only

`ci(build): drop tag trigger, keep workflow_dispatch`. The Build workflow used to fire on every `v*` tag push, which duplicated the cross-platform Tauri build that the Release workflow already runs. `release.yml` covers the tag case fully (creates the draft release, uploads bundles, publishes); leaving `build.yml` triggered by tags burned six extra runners per release with no downstream consumer. The trigger is now `workflow_dispatch:` only — useful for "manually build the current branch and download the artifacts" without cutting a tag.

### README — AI and Observability sections rewritten

`docs: clearer AI background session description + local-only Observability`. The AI providers Highlights paragraph drops the irrelevant "show their version" claim and expands the background-session section with bullets that actually explain the user-facing surface — worktree under `.beardgit/ai-worktrees/<slug>`, dedicated `ai/<provider>/<slug>` branch, real-time streaming output that survives tab switches, FIFO + concurrency cap, final markdown report alongside the worktree, Resume / Focus actions. Observability is renamed *Observability — local-only* and gains an explicit "nothing leaves your machine" paragraph (no telemetry / analytics / phone-home; the only outbound traffic the app initiates is the Tauri auto-updater poll) plus a per-platform log-path table. The Why bullet on credential storage is renamed *Secure and private by default* and reinforces the no-telemetry posture at scan-level.

### Landing page — SEO/social meta, AVIF screenshots, Keyboard + FAQ sections

`feat(docs/landing): SEO/social meta, AVIF/WebP shots, keyboard + FAQ sections`. The marketing landing under `docs/` gains a real `<head>` — light/dark `theme-color`, canonical URL, full Open Graph + Twitter card with a 1200×630 `og:image`, a proper `svg` + 32px favicon + 180px apple-touch-icon set, and a JSON-LD `SoftwareApplication` block. Fraunces is pinned to `opsz=144,wght=500` and Fira Code to `wght@400;500` to cut first-paint cost. Every showcase `<img>` becomes a `<picture>` with AVIF → WebP → PNG fallback (hero keeps `fetchpriority=high`; the strip below it is `loading=lazy`). Total screenshot footprint drops from **8.6 MB PNG to ~497 KB AVIF**; the cold hero shot is now 86 KB instead of 1.3 MB. The placeholder-tag markup and CSS are removed now that real screenshots exist.

Two new sections: **04 Keyboard** lists 14 real shortcuts (Git / Graph / Tabs + UI) sourced from `src/lib/stores/shortcuts.ts` so the page can't drift from the app, and **06 FAQ** ships 7 collapsible items (unsigned builds, AI key storage, no-Electron, offline, other forges, license, bug reporting). The license FAQ wording makes explicit that BeardGit is free to use anywhere — the NC clause only blocks reselling BeardGit itself. Eyebrows renumber (Install 04→05, FAQ 06); both new sections are added to the top nav and the footer Product list. `app.js` now updates `<source srcset>` before `<img src>` on theme swap so the browser re-evaluates the `<picture>`, and `wireDownloads` fills a hidden hero badge with the version and relative release date when GitHub responds. Adds `docs/robots.txt` and `docs/sitemap.xml`.

### Community health — Code of Conduct, security policy, issue/PR templates

`chore(community): add CoC, security policy, issue/PR templates`. Sets up the GitHub community-health files. `CODE_OF_CONDUCT.md` and `SECURITY.md` (latest stable release supported, `beta` best-effort, vulnerabilities reported via GitHub Private Vulnerability Reporting). `.github/ISSUE_TEMPLATE/` ships structured forms for bug reports and feature requests plus a `config.yml` that disables blank issues and points support questions at Discussions. `.github/PULL_REQUEST_TEMPLATE.md` nudges contributors to target `beta` (not `main`) and prompts for type-of-change checkboxes that match the conventional-commit prefixes the repo already uses.

### Internal

- `crates/cli-provider/src/auth.rs`: `mock_cli` switched from `fs::write` + `set_permissions` to `OpenOptions::new().mode(0o755)` + `sync_all` + `drop`, plus a `wait_for_exec_ready` probe loop that retries the script's exec on `ETXTBSY` for up to ~1.5 s before returning. Targets the intermittent *"Text file busy"* failure on the GitHub Actions ubuntu runners. (Still flaky on `main` — tracked separately.)
- `crates/terminal/src/manager.rs`: `Session::last_fg_process` and its constructor assignment are gated behind `#[cfg(unix)]` to match the `master_fd` field, silencing the dead-code warning Windows builds were emitting.
- `crates/git-engine/src/patch.rs`: removed an unused `create_review_patch` helper that briefly backed a HEAD-vs-worktree review patch (replaced by direct `createWorkingTreePatch(true)` from the FE; staged-only is the correct semantic for review).

## [0.1.9] — Init repo on folder open + consolidated post-0.1.8 work — 2026-04-27

Bundles every change since `v0.1.8-beta` into a single cut. Drafts that were briefly headered as `0.1.10-beta` and `0.1.11-beta` while staged on `beta` are folded back in here — they never tagged or shipped under those numbers, so the version state package.json/tauri.conf bumps cleanly to `0.1.9`.

### Init repo on folder open

`feat(init-repo): InitRepoDialog + init_repo pipeline (gh/glab repo create)`. Picking a folder via **+ → Open folder…** that isn't already a git repository now opens an actionable dialog instead of failing with a generic toast. The dialog walks the folder via a new `count_folder_contents` Tauri command (respects any pre-existing `.gitignore` plus a built-in skiplist; capped at 50k files / 1 GiB) to preview how many files would be staged, then submits a single `init_repo` pipeline that runs `git init` with `main` as the initial HEAD, optionally drops a multipurpose `.gitignore` (covers macOS/Linux/Windows OS metadata, every common IDE/editor, and the Node/Rust/Python/Java/.NET/Go/Ruby/PHP/C++/Swift ecosystems), optionally stages and commits the existing files as **Initial commit**, optionally creates a matching repo on the active forge provider via `gh repo create` / `glab repo create`, wires it as `origin`, and pushes — all in one submit.

The primary action button auto-labels itself based on the ticked options (`Initialize` / `Initialize & create remote` / `Initialize & commit` / `Initialize, commit & push`). The provider dropdown only renders when more than one provider is connected; with zero providers the "Create remote" checkbox is disabled with a `Sign in to a provider in Settings` hint. The pipeline preserves partial progress on failure — a missed push, for example, leaves the local repo and origin remote intact so the user can retry from the toolbar without losing their work. Errors are step-tagged so the dialog can banner *which* step failed (`Failed to create remote on GitHub: name already taken`) and which provider rejected the request.

Backend additions: `OpenProjectError::NotARepo` (so the FE can branch on the typed payload instead of substring-matching error strings), the `init_repo` and `count_folder_contents` Tauri commands, a new `ForgeProvider::create_repo` trait method with default `NotSupported`, GitHub and GitLab CLI adapters that map the modern flags (`--private`/`--public`, `--defaultBranch main`) and translate name-collision wording from both the REST and GraphQL surfaces (`already exists` / `already been taken`), and a `build_forge_provider_for_index` helper that lets `init_repo` resolve a provider before any project is open. Adds `i18n` keys in en-US + es-ES for every dialog label, primary-button variant, in-flight step strip, error banner, and the success toast.

### Init repo — use existing remote URL + tooltips on every element

`feat(init-repo): RemoteSpec::UseExisting wires a typed URL as origin`. The InitRepoDialog gains a second remote mode. Inside the renamed **Add remote repository** fieldset a radio chooses between *Create new on {provider}* (the existing `gh repo create` / `glab repo create` flow) and *Use existing remote URL* — a free-text field that wires whatever the user types as `origin` and pushes the initial commit, without going through any forge provider API. Lifts the original spec's "manually-typed remote URL" non-goal.

Side benefit: the dialog is now usable with **zero providers connected**. With no `gh`/`glab` configured, the *Create new* radio is disabled with the existing `Sign in to a provider in Settings` hint and *Use existing* auto-selects, so self-hosted git, Gitea, BitBucket, Codeberg, and internal corporate forges all work end-to-end from the same dialog. URL validation is intentionally loose — submit accepts any non-empty trimmed string, with a soft inline hint when the value doesn't look like an `https://` / `http://` / `ssh://` / `git@` / local-path URL. The push step is the authoritative validator; a typo / unreachable host / non-empty remote / missing credentials surfaces in the existing `Push` failure banner. Partial success is still preserved — a missed push leaves origin wired so the user can retry from the toolbar.

Two new pipeline labels (`Initialize & wire origin`, `Initialize, commit & push to existing remote`) cover the new combinations, bringing the primary-button label to a 6-state machine. A new success-toast variant (`Initialized {name} and pushed to the existing remote`) fires when the user-typed URL was pushed to.

`feat(init-repo): tooltips on every element`. Every interactive element in the dialog — the *Add remote* checkbox, both mode radios, the provider dropdown, the name input, the visibility radios, the URL input, the *Commit existing files* checkbox, and both action buttons — now carries a paraglide-driven `title=` tooltip. The primary button's tooltip is dynamic: it lists the exact pipeline steps that will run on click (e.g. `• git init on main` / `• Drop the multipurpose .gitignore` / `• Wire origin = https://…` / `• Push origin main`), updating live as the user toggles options. The dynamic step labels are localised in both en-US and es-ES.

Backend additions: a second `RemoteSpec::UseExisting { url, push_after }` variant; `run_init_pipeline` becomes a 3-arm match (`None` / `Create` / `UseExisting`); the Tauri wrapper's provider-resolution match is made exhaustive (no wildcard) so a future variant fails to compile here. The `UseExisting` arm trims whitespace, calls `git2::Repository::remote("origin", url)` and (optionally) `push_initial`; no provider lookup happens.

Frontend additions: TS `RemoteOption` discriminated union (`{kind:"create"} | {kind:"use_existing"}`); the `initRepo` payload mapper switches on `kind` and emits the matching snake_case wire shape. Dialog state grows `remoteMode` + `remoteUrl`; `$effect` defaults the mode based on provider count; `submit()` snapshots `path`/`name`/`mode`/`pushAfter` before `closeInitRepoDialog()` clears component state.

### Sidebar edit-menu fixes

### Fixed — drag-reorder of Navigation items did nothing

`fix(sidebar): enable HTML5 drag-drop by disabling Tauri native intercept`. Tauri 2 windows ship with `dragDropEnabled: true` by default — that intercepts pointer drag events at the window level for the native file-drop API and silently swallows the HTML5 `dragstart` / `dragover` / `drop` events the sidebar uses to reorder Navigation items. Set `dragDropEnabled: false` on the main window so the customize-layout drag-handle actually works. The keyboard fallback (focus the handle, ArrowUp / ArrowDown) was always functional but invisible to most users; both now work.

### Added — "Show more…" expander when items are hidden

`feat(sidebar): inline reveal for hidden navigation items`. The customize-layout panel lets the user hide Navigation items, but in normal mode there was no escape hatch — once hidden, an item could only be re-enabled by entering edit mode. A new `Show more…` row now appears below the visible list whenever any item is hidden, with a count badge of how many. Clicking it expands the hidden items inline (dimmed but clickable, so they're still navigable); clicking again collapses them. Local-only UI state — re-collapses on next mount.

### Fixed — customize-sidebar pencil button rendered a hover rectangle

`fix(sidebar): edit-toggle uses IconButton`. The "customize navigation" pencil in the sidebar's Navigation header had its own `.edit-toggle` style that drew a faint rectangular fill on hover, breaking the app-wide rule established in the IconButton refactor (icon-only buttons brighten the glyph, never draw a background). Migrated to the shared `IconButton` and dropped the dead CSS. Tooltip is the new `tooltip_customize_sidebar` paraglide key (en + es).

### Post-IconButton polish

### Fixed — `gh pr view` regression on `headRepositoryUrl`

`fix(mr-pr): use headRepository.url instead of headRepositoryUrl`. The recent PR diff view shipped with `headRepositoryUrl` in the `gh pr view --json …` field list, which `gh` does not expose — only `headRepository` (an object) is valid, and the URL lives on its `.url` sub-field. The Rust side now requests `headRepository` and walks the nested `url` via the existing path-walker, so opening any GitHub PR detail no longer fails with `CLI error: Unknown JSON field: "headRepositoryUrl"`. GitLab's `head_repo_url_path` was already correct (`["source_project", "http_url_to_repo"]`).

### Fixed — AI toolbar dropdown click-outside swallowed by xterm/CodeMirror

`fix(layout): close AI dropdown via capture-phase mousedown`. The dropdown's "click anywhere outside to close" handler ran in the bubble phase on `document`, so embedded surfaces that call `stopPropagation()` on mousedown (xterm.js terminal, CodeMirror editor) prevented it from ever firing. Switched to `{ capture: true }` so the handler always sees the click first.

### Fixed — AI dropdown tooltip described only one of its two actions

`fix(layout): clarify AI dropdown tooltip`. The trigger button labelled itself "Start AI background session on a worktree", which is only one of the two things in the menu — interactive provider terminals are the other. Updated the localized tooltip in en-US and es-ES to mention both.

### Fixed — `+` glyph and "↗ Graph" nav in the Branches view

`fix(branches): use canonical + glyph + wire show-in-graph nav`. The Branches header's "new branch" button rendered Nerd Font `U+E632` (a non-`+` glyph); replaced with `U+F067` (`nf-fa-plus`) to match every other "+" button in the app. Separately, clicking the `↗ Graph` button on a commit selected from the Branches view did nothing visible — `navigateToCommit` repositioned the graph viewport but the active view stayed on Branches. The handler now also calls `handleNavigate("graph")`, mirroring the working callsites on the graph and reflog views.

### Fixed — primary / danger / ghost button system: tonal-rest, solid-hover

`fix(ui): tonal-rest, solid-hover for shared Button variants`. The shared `Button.svelte` `primary` and `danger` variants used a fully-saturated accent at rest with `opacity: 0.9` on hover, which read as "highlighted at rest" and "dimmed on hover" — the inverse of the desired feedback. Worse, the local `.btn.primary` in `AiSessionDetail.svelte` was being silently overridden on hover by a cascading `.btn:hover` rule that turned the label `var(--accent-blue)` (matching the fill, hiding the text). All three variants now follow a consistent rule:

- **`primary`**: translucent accent-blue tint at rest (`color-mix(accent-blue 18%, transparent)`) with accent-blue text → solid `var(--accent-blue)` with `text-primary` on hover.
- **`danger`**: same pattern in red. Solid red at rest read as alarming for buttons (Disconnect, Clear cache, Delete asset) that don't fire instantly.
- **`ghost`**: dropped the `background: var(--overlay-hover)` rectangle on hover; only the text colour brightens, matching the `IconButton` rule.

The `AiSessionDetail` Resume / Focus buttons get the same pattern via local CSS overrides.

### Fixed — "Check for updates" raw error + missing diagnostics

`fix(settings): friendly error + diagnostics for update check`. Two changes that landed together:

- The Tauri updater plugin returns implementation-detail strings (`"could not fetch json"`, `"the network has temporary issue"`, etc.) verbatim. The Settings → Advanced → Check for updates row now maps recognisable "endpoint unreachable" shapes to a localized hint (`update_server_unreachable`) and only shows the raw text for unexpected failures.
- A new diagnostics block under the row exposes `Last checked: <relative time>`, `Endpoint: <url>` (the `latest.json` URL the plugin tries), and on error a monospace `Detail: <raw>` line. `UpdateState.lastCheckedAt` is set in the store on every terminal resolution. Useful for distinguishing "endpoint 404'd" from "DNS hiccup" without leaving the app.

Note: the underlying 404 (`releases/latest/download/latest.json` is missing because every release is currently flagged `prerelease=true` and GitHub's `/releases/latest/` redirect skips prereleases) is a release-pipeline concern, not addressed here.

### Icon-only buttons + brand logos

### Added — `IconButton` component + `Button.description` prop

`feat(ui): IconButton with native title tooltip; Button gains description`. New `src/lib/components/ui/IconButton.svelte` is the canonical primitive for buttons that show only a Nerd Font glyph (close ✕, refresh, new branch, etc.). It always renders a transparent background — only the glyph color brightens on hover, never a rectangular fill. `description` is required and drives both the native browser `title` (hover tooltip) and the `aria-label`, so an icon-only button is never silent to screen readers. New en-US/es-ES tooltip keys (`tooltip_close`, `tooltip_close_log`, `tooltip_remove`, `tooltip_new_branch`, `tooltip_refresh`) cover the migrated call sites.

`Button.svelte` gains a matching `description?: string` prop with the same semantics (sets `title`, falls back to `aria-label` when `ariaLabel` isn't provided).

### Changed — every icon-only button migrated, dead per-component CSS dropped

`refactor(ui): migrate 18 icon-only buttons to IconButton`. The grab-bag of `.btn-icon` (dialog.css), `.icon-btn` (list.css), `.refresh-btn` (list.css) and per-component `.header-btn` rules — each with its own slightly-different "fill on hover" — are gone. Migrated callsites: `BlameView`, `CommitDetail`, `ShortcutOverlay`, `BranchList` (new branch + refresh), `BisectWorkflow`, `PipelineView` (close log), `PipelineList`, `IssueList`, `MrPrList`, `TagList`, `AiSessionList`, `TriggerWorkflowDialog` (close + remove pair), `ReleaseDetail` (delete asset), `StagingDiffEditor` (close), and the shared `List.svelte` refresh button. Visible difference: hovering an icon-only button now brightens the glyph instead of drawing a rectangle around it.

### Changed — official brand logos for Codex / OpenCode

`feat(ai-sessions): theme-aware brand logos for codex + open_code`. The placeholder `codex.svg` (an outdated OpenAI mark with a hardcoded green fill) and `opencode.svg` (two arrow brackets) are replaced with the official assets:

- **Codex** — OpenAI monoblossom mark, shipped in black + white variants. `ProviderIcon` picks the right one off `$activeTheme.meta.mode` so the logo stays legible on both dark and light themes.
- **OpenCode** — official two-tone wordmark, shipped in light + dark variants and switched the same way.

`<img>` cannot resolve `currentColor` from the parent document, so a single asset per brand would either flatten the two-tone OpenCode mark or paint OpenAI's monoblossom in only one mode — hence the two-asset approach.

### Repo settings multi-instance fixes

### Fixed — auth probe scoped to the repo's host

`fix(repo-config): scope gh/glab auth status to the repo's host`. Opening repo settings on a `gitlab.com` (or `github.com`) repo no longer reports "authentication required" just because an *unrelated* configured host (e.g. a self-hosted GitLab on a corporate VPN that happens to be unreachable) is failing. The CLI probe now passes `--hostname <host>`, where `<host>` is extracted from the repo's `origin` remote, so multi-instance `glab` / `gh` configs no longer poison each other. The frontend auth-required classifier was also tightened to match the structured `RepoConfigError::NotAuthenticated` Display prefix instead of any `auth` substring, so unrelated load failures no longer trigger the auth empty state.

### Fixed — `glab repo view` payload with both `topics` and `tag_list`

`fix(repo-config): drop tag_list serde alias on GlabRepoView.topics`. Modern GitLab emits both the canonical `topics` array and the deprecated `tag_list` array in the same `repo view -F json` payload. The previous `#[serde(alias = "tag_list")]` mapped both to the same struct field and serde rejected them as `duplicate field "topics"`, surfacing in the UI as "Failed to load — JSON parse error: duplicate field `topics`". The alias is removed; we read `topics` only.

### Theme color audit groundwork

### Theme — six new `--overlay-accent-*` tokens

`feat(theme): derive six --overlay-accent-* tokens in applyTheme`. The runtime theme now exposes `--overlay-accent-blue`, `--overlay-accent-red`, `--overlay-accent-green`, `--overlay-accent-orange`, `--overlay-accent-purple`, and `--overlay-accent-muted`, each derived at `applyTheme` time from the matching `ThemeData.derived` accent (or `text_secondary` for "muted") at 10 % alpha. Theme JSON files are unchanged — this is a pure runtime extension, ready to be consumed by the upcoming component color sweep.

### Brand allowlist

`feat(theme): add brand-colors.ts allowlist with snapshot test`. Log/provider brand colors (Anthropic, GitHub, GitLab, OpenAI, Codex, Gemini) now live in a single `src/lib/ui/brand-colors.ts` module. The component sweep will migrate every hardcoded brand hex onto these constants.

### Playwright visual baseline

`chore(test): install Playwright + visual baseline spec for top-level routes`. Added `tests/visual/routes.spec.ts` covering every top-level sidebar route in dark and light mode, so the upcoming component color sweep can diff against a known-good paint. The spec waits for `--overlay-accent-blue` to confirm `applyTheme` has run before snapping. Baseline screenshots will be captured on the first CI run where the full Tauri runtime is available (the Vite-only dev server lacks Tauri IPC, so `applyTheme` cannot fire locally).

### Lint — color literals blocked

`chore(lint): stylelint + custom eslint rule for color literals`. Stylelint's `color-no-hex` plus a small `eslint-plugin-beardgit/no-hex-in-svelte` rule now run in CI (`.github/workflows/ci.yml`). Hardcoded colors are rejected everywhere except the four documented sources of truth: `src/lib/stores/theme.ts`, `src/lib/utils/status.ts` (pre-theme fallback map), `src/lib/ui/brand-colors.ts`, and `src/app.css` (root token defaults). Escape hatch for rare one-offs: a `// beardgit:allow-hex: <reason>` comment (or `<!-- beardgit:allow-hex: ... -->` in Svelte template markup) on or immediately above the offending line.

### PR diff view

### PR / MR diff view

`feat(mr-pr): per-file diff + inline review + prev/next nav`. Clicking any file in a PR or MR now opens a bottom resizable `DiffEditor` with the same CodeMirror merge view used by branches, stashes, tags, and the graph. Works for both GitHub and GitLab; fork PRs are supported via a new `ensure_commit_local` Tauri command that fetches the head commit on demand and streams progress to the tasks drawer. Inline review comments render as gutter bubbles with an on-click thread panel + composer, with GitLab `resolve`/`unresolve` toggles surfaced inline; posting a comment refreshes the PR detail so both the inline widget and the bottom comments section stay synced. Above 20 changed files the file list auto-switches to a collapsible path tree with per-folder aggregate add/del stats; under 20 it stays flat. `[` / `]` cycle through files with a visible "3 / 24" position indicator in the diff header. Binary files render a "Binary file — no preview" placeholder instead of the merge view.

### Data model

`feat(mr-pr): base_sha / head_sha / head_repo_url on MrPr`. Both the Rust and TS `MrPr` types gained these fields, populated from `gh pr view --json headRefOid,baseRefOid,headRepositoryUrl` and `glab mr view`'s `diff_refs` + `source_project.http_url_to_repo`. The new `ensureCommitLocal` IPC command uses them to fetch fork-PR heads before reading file content.

### Fix

`fix(mr-pr): include diff_refs in GitLab inline-comment position`. `add_mr_pr_inline_comment` on GitLab now sends the full `base_sha` / `head_sha` / `start_sha` trio in the `position` object, which the previous single-path implementation omitted — that shape is required for anything but trivial diffs.

### Branches UI feature-complete

### Branches — new-branch entry points + rename + force-push + shortcut

`feat(branches): unified create-branch dialog, rename, force-push, Cmd+Shift+B`. The Branches panel gains a visible "+" in its header that opens a new `CreateBranchDialog`, the single entry point used by every create-branch call site (header, context menu, graph, reflog, and the new global `⌘⇧B` / `Ctrl+Shift+B` shortcut). The dialog pre-fills the local name by stripping the matching remote prefix when branching from a remote ref, offers a "From" picker covering local and remote branches, and chains a `checkoutBranch` when "Check out new branch" is ticked (default on). Two previously WIP context-menu items are live: "New branch from here" opens the dialog with the clicked ref as the source; "Push" fires directly to the single configured remote or expands to a submenu when multiple remotes exist. Branch rename ships as a new dialog + `rename_branch` Tauri command; renaming the checked-out branch updates HEAD automatically and the panel's selection follows the new name. Force-push gets its own submenu that always requires a destructive confirm — even for single-remote repos — and passes `--force-with-lease` to `git push` along with `-u` so first-time pushes establish the upstream tracking ref. The `graph_branch_name_prompt` `window.prompt()` calls in the graph and reflog are retired.

### Sidebar customization

### Sidebar — reorder and hide Navigation items

`feat(sidebar): user-customisable Navigation order + hide toggles`. The Navigation section of the sidebar now has an explicit edit mode — click the pencil in the `NAVIGATION` label to enter. In edit mode each row gets a drag handle, an eye toggle, and the section header gains `Reset` + `Done` buttons. Drag-and-drop reorders items (keyboard: `ArrowUp`/`ArrowDown` on the drag handle); the eye toggles individual items between visible and hidden with a guardrail preventing the user from hiding every last section. Layout is persisted app-wide (not per-repo) via two new `AppConfig` fields (`sidebar_nav_order`, `sidebar_nav_hidden`) and debounced by 250 ms. When a future release ships a new nav item, it appears automatically at the end of the saved order.

The Provider section (GitHub / GitLab) is no longer user-managed — it auto-hides when no provider is connected, and if the user was viewing a provider-scoped route (`pipelines`, `issues`, `merge-requests`, `releases`, `repo-config`) at disconnect time, the app reroutes them back to the Graph.

### AI sessions list trim

### AI sessions — one-line rows, detail-pane actions

`feat(ai-sessions): trim list rows to icon + title + date`. The Active terminals and Conversations sections in the AI Sessions view now render one line per row: provider icon, title, relative date. Everything else — provider name, cwd, forked-from badge, bg-run status badge, Resume / Focus buttons — moves to the detail pane. Tab and segment rows, which previously had no detail branch, gain one: selecting them surfaces the provider, title, cwd, and a Focus button so keyboard users can reach the action without chasing a hover affordance. Three selection stores (`selectedConversationId`, `selectedBackgroundSessionId`, `selectedActiveTerminal`) now coordinate through a shared `selectAiSessionRow` helper so at most one row is selected at any time.

### Toolbar — AI dropdown, plain terminal button

- `refactor(toolbar): AI becomes a dropdown, terminal becomes a plain button`. The toolbar's terminal split-button is now a single button (its old dropdown only surfaced the project-root fallback that the button itself already does). The "AI" / "IA" button is now an always-dropdown listing every installed AI CLI provider plus a "Launch session in background…" entry, which is where the per-provider launchers used to live under the terminal chevron. Escape / outside-click close the menu; aria-haspopup / aria-expanded / role=menu land for screen-reader parity.

### Pipelines + Issues lists v2 (was drafted as 0.1.11-beta)

### Wider list pane, shared row primitive, richer meta

`feat(lists): widen pipeline + issues pane to 420px and share TwoLineRow`. The Pipelines and Issues side panes now open at 420 px so seeded data stops ellipsing past the first glance. Rows in both lists render through a new shared `TwoLineRow` primitive (`src/lib/components/common/TwoLineRow.svelte`), so future layout tweaks land in one file instead of drifting between the two views. `PipelineList` migrated off its bespoke container onto the shared `List` component — header, search, empty state, load-more, and the polling bar are now the List's responsibility.

Issues rows drop the 3-label cap (every label shows, wrapping onto line 2 naturally), surface the milestone as a chip, and render up to three assignees as an `AssigneeStack` with `+N` overflow. Pipeline rows gain the triggering actor (`triggering_actor.login` on GitHub, `user.username` on GitLab) and move the duration onto the meta line so the trailing-date slot only holds the relative timestamp.

New i18n keys: `issues_milestone_icon_aria`, `issues_assignees_aria`, `pipeline_actor_aria` (en-US + es-ES).

### Repo settings in the sidebar (was drafted as 0.1.10-beta)

### Repo settings — sidebar view replaces the modal

`feat(repo-config): move repo settings to a provider-scoped sidebar view`. The per-repo "Repo settings" UI is no longer a modal dialog launched from a cog on the project tab. It's a first-class sidebar entry inside the GitHub / GitLab section, with a master/detail layout (section list on the left, option rows on the right), hash deep-links (`#repo-config/<section>`), per-section Save/Discard, and a navigation guard that catches every way of leaving a dirty section — sidebar click, section switch, project switch. Sections render instantly; each one loads its data in the background on first open via a shared loader with an in-flight dedupe + ~30 s TTL cache. The cog button and the right-click "Repo settings" context menu on the project tab are gone — the sidebar is the only entry point.

### Auto-update, viewport-windowed graph, lean statusbar, landing page, quick wins, reactivity foundation, AI sessions UX, forge data fixes, settings IA polish, log rename, E2E retirement, AI sessions transcript-first rewrite (originally drafted as the first 0.1.9 cut)

Two distinct waves of work since `v0.1.8-beta`. Wave one (2026-04-20) landed the in-app auto-updater, viewport-windowed commit walking, the lean statusbar + unified tasks drawer, a persistent graph layout cache, the GitHub Pages landing page, and the "Quick Wins" refactor bundle. Wave two (2026-04-21 / 2026-04-22) shipped seven sequential specs — each on its own feature branch with a dedicated design + plan doc — culminating in the AI sessions transcript-first rewrite.

### In-app auto-update

`feat(update): in-app auto-update with re-auth notice`. The app now self-updates from the tauri-updater feed without sending the user back to the download page. When the update lands on a build that needs a re-auth (token schema bump, new scope), the updater surfaces a one-time notice so users don't hit a silent failure on first post-update connect.

### Graph — viewport-windowed commit walking (MT-1)

`feat(MT-1): viewport-windowed commit walking (#4)`. The graph builder no longer materializes the entire history on cold start. The walker pages the commit list against the viewport window and extends as the user scrolls, so repos with 100k+ commits paint in milliseconds instead of seconds. Pair this with the Spec-1 `GraphViewportCache` slice and cold-start paint is now synchronous from persisted state.

### Lean statusbar + unified tasks drawer

`feat(ui): lean statusbar + unified tasks drawer`. The statusbar is compacted to the essentials — branch, provider, tasks indicator, AI slot — with everything else collapsing into dropdowns. The old tasks footer and the floating task toasts are unified into a single tasks drawer that slides up from the statusbar; it doubles as the "See details" target for sticky failure toasts.

Follow-up fixes in the same slice: `fix: statusbar tasks + ai-slot navigation + post-commit graph refresh` restored navigation when clicking a task row; `fix(tasks): restore popover UX + spinning state-coloured icon` brought back the running-state animation that the migration had dropped; `fix(tabbar): AI background button renders play + branch glyph` and `fix(tabbar): AI background button shows bold AI/IA text label` finished the tab-bar entry point for background runs.

### Persistent graph layout cache

`feat: persistent graph layout cache`. Graph lane assignments and row positions are now persisted per-repo so the second-and-subsequent open of a project hits warm layout state. This is the storage half of the Spec-1 cache-first paint path.

### Repo config — configure remote repo via gh/glab CLI

`feat(repo-config): configure remote repo via gh/glab CLI`. The "Configure remote" flow now delegates to `gh repo edit` / `glab repo update` for supported fields (description, homepage, topics, default branch) instead of inventing a bespoke REST surface. Keeps the provider abstraction thin and picks up any field support upstream adds for free.

### AI — Codex + OpenCode provider parity

`feat(ai): Codex + OpenCode provider parity`. Fills the remaining gaps between the three providers so the sessions / background / settings surfaces behave identically across Claude Code, Codex, and OpenCode. Session detection, argv builders, config-file paths, and brand wiring all line up — no more per-provider "coming soon" states in the UI.

### Landing page (docs/)

`site(landing): add GitHub Pages landing page under docs/`. BeardGit now has a public landing page served from `docs/` on GitHub Pages.

- `site(landing): wire real screenshots with theme-aware swap and lightbox` — replaces the placeholder art with actual app screenshots; each shot has dark + light variants that swap based on the visitor's `prefers-color-scheme`, and clicking opens a lightbox.
- `fix(landing): responsive breakpoints for mobile + tablet` — layout no longer breaks under 768 px.

### Forge — pre-releases + closed PRs on GitHub

`fix(forge): surface pre-releases + closed PRs on GitHub`. The GitHub list queries were filtering out prereleases and closed PRs by default; both now appear in the respective views alongside stable releases and open PRs.

### AI sessions polish (pre-Wave A)

`fix(ai): session list polish, resume-in-terminal, and dialog alignment`. Pre-spec round of fixes on the AI Sessions view — row alignment, dialog chrome consistency, and the resume-in-terminal action wiring — preceding the Spec-2 UX pass further down.

### Quick Wins bundle

Seven disjoint-file branches merged sequentially into `feat/quick-wins`, then into `beta`. No single slice warrants a full section but together they tighten the component story meaningfully.

- **Shared empty-state partial** — `feat(frontend): shared empty-state partial + descriptions for issues/pipelines/releases`. Empty states across Issues, Pipelines, and Releases now render from one primitive with per-view descriptive copy, instead of three near-duplicate empty blocks.
- **Picker consolidation** — `refactor(mr-pr): migrate MrPrDetail to common/LabelPicker`, `chore(mr-pr): drop duplicate LabelPicker component`, `refactor(issues): adopt shared dialog chrome in AssigneePicker`, `refactor(mr-pr): adopt shared dialog chrome in ReviewerPicker`. The MR-PR view stops shipping its own `LabelPicker` copy; AssigneePicker and ReviewerPicker drop their hand-rolled dialog styles in favour of the shared chrome.
- **`.btn-icon` refactor** — nerd-font glyph-only buttons across `ShortcutOverlay`, `CommitDetail`, `BlameView`, `StagingDiffEditor`, `TriggerWorkflowDialog`, `PipelineView`, `TaskPopover`, `TaskPanel`, and the dialog close buttons in `RebaseEditor` / `CreateReleaseDialog` / `CreateIssueDialog` / `TriggerWorkflowDialog` now share one `.btn-icon` class on `dialog.css`. `refactor(frontend): resolve .btn-icon naming collisions` cleans up the one place two different components had collided on the name.
- **Graph refresh on branch-from-context-menu** — `fix(graph): reload graph after creating a branch from context menu`. The graph didn't pick up branches created from the context menu until the next manual refresh; it now reacts immediately.
- **Console-noise cleanup** — `fix(frontend): route theme/branch errors through toast, drop console noise`. Theme-load and branch-switch failures used to log to the console and vanish; they now surface in the toast system where the user actually sees them.
- **E2E on feature branches** — `ci: run E2E suite on all branches, not just main/beta`. Catches regressions before the `beta` merge, not after. (Later disabled temporarily while the 2026-04-21 spec set lands, then removed entirely with the Spec-6 E2E retirement below.)

### Tests — app-core command coverage

`test(app-core): unit tests for all command modules`. The 24 command modules split out of the old monolithic `commands.rs` in v0.1.8 now have unit test coverage across the board, not just the three or four hot paths.

---

### Wave two — the 2026-04-21 / 2026-04-22 spec cycle

Seven sequential specs brainstormed and shipped across two days. Each spec merged into `beta` on its own feature branch with a dedicated design + plan document.

### AI sessions — transcript-first rewrite (Spec 7, 2026-04-22)

The AI Sessions view no longer treats `~/.claude/sessions/{pid}.json` as the source of truth. Each provider now reads its own on-disk transcript store and surfaces conversations as first-class rows; terminal tabs BeardGit actually owns are a separate "Active" list that supports real focus. Root cause ticket: [claude-code#12235](https://github.com/anthropics/claude-code/issues/12235) — every Claude `--resume` spawns a fresh process with a new UUID; attaching to a running CLI was never possible, so the old "Focus external" button was always a lie.

- **New `AiConversation` type** (Rust `ai_provider::AiConversation`, TS mirror) — id, provider, cwd, created/last_activity unix-ms timestamps, title, optional `parent_id` 8-char prefix when the transcript was forked.
- **New `AiProvider::list_conversations` trait method**. `list_sessions` and `is_session_active` deleted from the trait; `AiSession` the type stays (background runs still use it).
- **Claude Code** reads `~/.claude/projects/{cwd-slug}/*.jsonl`. Title walks for the first non-meta `type: "user"` record with extractable text (supports both string content and `{type, text}` arrays; skips `<command-name>` / `<local-command-caveat>` envelopes). Fork detection from the first record's `parentUuid`.
- **Codex** walks `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`, parses the first-line `session_meta` payload for id/cwd/timestamp. Title is MVP-empty (follow-up extracts the first user prompt from later lines).
- **OpenCode** shells out to `opencode session list --format json` and maps `directory → cwd`, `updated → last_activity_at`, `title` verbatim.
- **Two new Tauri commands** — `ai_list_conversations` and `ai_resume_conversation`. The old `ai_list_sessions` / `ai_resume_session` / `aiListSessions` / `aiResumeSession` are deleted.
- **Two-section UI** — `AiSessionList.svelte` renders Active (BeardGit-owned terminal tabs + segments + running/queued bg-runs) above Conversations (on-disk transcripts for the current repo). Mutually-exclusive selection between conversation and bg-run in the detail pane. Row action for Conversations is labelled "Resume in new terminal" with a tooltip naming the forking semantics explicitly.
- **Dropped** the `~/.claude/sessions/{pid}.json` scanner, `is_claude_process`, `process_alive`, `is_file_active`, `AiSessionWatcher` pointing at the dead PID dir (repointed to `~/.claude/projects/` + `~/.codex/sessions/`).
- **i18n** — 10 new `ai_sessions_*` keys in en-US + es-ES (peninsular tuteo). `ConversationRow` is keyboard-reachable (`role="button"` + Enter/Space).
- **Testing**: Rust workspace 1 045 tests (−31 for deleted legacy tests), Vitest 653 tests across 91 files (−25 for deleted stores), clippy clean, svelte-check 0/0.

### Reactivity & feedback foundation (Spec 1)

Every repository mutation — UI-initiated, AI-initiated, or external CLI — now broadcasts a precise `project-mutated` event so the UI converges on fresh state without per-call-site refresh code.

- **New `mutation-events` Rust crate** — `Snapshot::capture` + `diff`, `MutationGuard` RAII wrapper, `MutationKind` enum (commit / push / stash / worktree / staging_change / ai / external / …), `MutationFlags` struct, `emit_mutation` helper. Status fingerprint tracks per-file index/worktree bitflags so staging transitions flip `status_changed` even when the overall dirty boolean doesn't move.
- **app-core commands wrapped** — every mutating Tauri command (commit / amend / branch / tag / stash / staging / worktree / remote / cherry-pick / revert / rebase / reset / merge / conflict / clean / patch / submodule / mr_pr / releases) fires the guard on success.
- **watcher crate** — debounced `.git/**` change → `MutationKind::External` so CLI edits outside BeardGit still refresh the UI.
- **AI background runs** — coordinator captures pre/post worktree snapshots and emits `MutationKind::Ai { source }` on completion / failure / cancellation.
- **TS `mutations.ts` store** — single `project-mutated` listener coalesces events per rAF tick, buffers flags per project path, flushes on tab switch, dispatches the minimal refresh set to `graph` / `changes` / `stashes` / `worktrees` / `repoConfig`.
- **`runMutation` wrapper** — caller-side toast + task-record seam. Silent-set (stage / unstage / discard) suppresses success toast. Failures are sticky with a **See details** action that opens the Tasks popover at the failing task's detail panel.
- **Graph cache-first paint** — per-project `GraphViewportCache` slice persisted in `project-cache.ts`; synchronous hydration on cold start; faint skeleton stripes while first paint resolves; HEAD-OID reconciliation preserves scroll anchor when new commits land above the cached top.
- **Statusbar provider filter** — new `projectProvider` derived store. `repoConfig` wins; otherwise inferred from `origin` URL (`github.com`, `gitlab.*`). Renders 0 or 1 pill, never both.
- **Tasks popover regression fixed** — click-bubble race where the opening click hit the outside-click handler on the same frame. Rising-edge `ready` latch tied to the `open` transition.

### AI sessions UX pass (Spec 2)

The AI Sessions tab is now async-first, populates detail on click, supports open-in-terminal via the shared runMutation seam, and renders brand logos at native transparency.

- **`ProviderIcon` shared component** + brand SVG assets for Claude Code, Codex, OpenCode, and a generic fallback. No enclosing background square — brand logos render at native transparency.
- **`AiSessionList`** — shell paints immediately, refresh fires fire-and-forget in `onMount`. Row layout: 8 px padding, vertically-centered 20 px icon slot, External badge when `worktree_path` is missing or unreachable.
- **`AiSessionDetail`** — populates on `selectedBackgroundSessionId` change. Header uses `ProviderIcon`. Open-in-terminal / Cancel / Discard routed through `runMutation` with sticky-failure toasts.
- **AI Settings, TabBar, AiSlot, TaskEntryRow** — migrated to the shared `ProviderIcon`; generic nerd-font glyphs retired.
- **i18n** — en-US + es-ES keys for the new toast labels.

### Forge data fixes (Spec 3)

PR and Release detail panes no longer hang; error and empty states are distinct and localized.

- **`ForgeDetailShell`** — shared loading / error / empty / content state primitive used by both `MrPrDetail` and `ReleaseDetail`.
- **`withTimeout`** helper + `TimeoutError` — 15 s bound on detail fetches; errors surface via a new per-detail error store (`mrPrDetailError` / `releaseDetailError`) + sticky toast with a **Retry** action.
- **PR #18 infinite loading** — root cause traced to unbounded `gh api --paginate` for ~3 400-file diffs. Fix: 50 MB payload cap + 20 s subprocess timeout in `cli-provider` via `wait-timeout`. Frontend's 15 s `withTimeout` is the outer guard.
- **Release-blank** — `#[serde(default)]` didn't accept explicit JSON `null`. New `null_as_default` deserializer handles null `body` / `assets` from `gh`/`glab` payloads.
- **Empty-state copy** — "No changes in this pull request." / "No release notes or assets published for {tag}."

### Settings IA polish (Spec 4)

One canonical shape for the settings navigation, driven by the shared primitives from Spec 2.

- **`LookAndFeelSection.svelte`** extracted from `GeneralSettings`. General now owns a single Look & Feel card (no duplicate blocks).
- **Appearance tab removed** — collapsed into General; legacy `appearance` deep-links redirect to `general`.
- **Editor/Diff tab removed** — the placeholder page wasn't implemented; legacy `editor` deep-links redirect to `general`.
- **`CATEGORY_IDS`** reduced to `general / git / ai / integrations / advanced`.
- **AI Settings** — stray broken-glyph refresh button deleted; provider icons verified wired to `ProviderIcon`.
- **`ConnectionHowTo`** — reworked as a compact top-level dropdown (OAuth / PAT / CLI modes) rendered above the card, not inside.
- **Integrations Connections** — unified single Card with a new `ConnectionRow` primitive dispatching on `kind` (github / gitlab / gh / glab). `CliAuthSection.svelte` and `ProviderSetup.svelte` deleted.

### Log filename convention (Spec 5)

Log files now write as `beardgit.{date}.log` (was `beardgit.log.{date}`) so `*.log` globs match them and log-rotation tooling sees the date as the disambiguator, not the extension. Rotation cleanup tolerates both shapes so legacy files age out under the existing retention policy.

### E2E infrastructure retired

The WebdriverIO + tauri-driver suite is removed while the app is in heavy flux. Specs would need continuous rewriting against a moving target; the Vitest integration layer under `src/test/e2e/` remains as the sustainable cross-store regression suite. Re-introduction happens once the UI stabilises and a focused "write E2E from scratch" spec is brainstormed.

- Deleted `e2e/` directory (specs, fixtures, page objects, Dockerfile, run scripts).
- Dropped the `e2e-tests` job from `.github/workflows/ci.yml`.
- Removed `@wdio/*` devDependencies + the `npm run e2e*` scripts from `package.json`.
- Dropped the `window.__E2E__` surface + `VITE_BEARDGIT_E2E` gate from `src/routes/+layout.svelte`.

### Auth & Integrations

- Connecting a forge PAT now also logs the matching `gh`/`glab` CLI in automatically via a fire-and-forget background task; disconnecting logs it out. Users get both API and CLI auth in one action, under the same identity.
- `ConnectionHowTo` PAT guidance rewritten: defaults to the PAT mode, explicit warning against fine-grained tokens (they break `gh`/`glab` via missing GraphQL support — see cli/cli#6680), SSO callout for GitHub orgs, and a collapsed "manual login command" reference block with `<YOUR_PAT>` placeholders and a no-history `read -rs` variant.
- CLI row in the Connections card now labels itself **"Connected · via {Provider} PAT"** when its authenticated username matches a connected forge provider — surfacing that both halves share one identity.
- Removed orphan programmatic OAuth device-code flow (`cli_login` Tauri command, `start_cli_login`, `OAuthLoginProcess`, `OAuthLoginInfo`, `oauth-device-code` event emit). Superseded by the terminal-hosted `gh auth login` flow shipped with xterm.js in v0.1.5 and the new PAT-pipe flow above.

### Wave A polish — buttons, markdown, AI sessions

Three disjoint-file slices shipped in parallel worktrees, merged sequentially to `beta`.

- **Button primitive.** `ui/Button` gains a `subtle` variant (theme-token tonal fill + accent-blue hover border) that sits between `primary` and `ghost` for actions that need to read as actionable without going loud. `secondary`'s baseline moved from `--overlay-hover` (a hover state, never a fill) to `--bg-secondary` — fixes the task-row action buttons rendering as near-white on the default dark theme. `ConnectionRow.svelte` fully migrated: every button routes through the primitive, 39 lines of shadowed local `.btn-*` CSS gone. Manage button no longer reads as disabled.
- **Markdown renderer.** Release / Issue / MR-PR detail bodies now render full GitHub-flavored markdown (fenced code blocks, tables, task lists, autolinks, strikethrough) via `marked`. Replaces the minimal `snarkdown` renderer that dropped or garbled those elements. Sanitiser extended to allow `<input type="checkbox">` for task lists and to rewrite `href="javascript:…"` to `href="#"`. Scoped `.body` styles added per detail component using theme tokens.
- **AI Sessions interactions.** Fixed four bugs that made the view feel broken: clicking an external (provider-reported) session now populates the detail pane (was always empty — the derived selection only looked in the background-run map); the detail pane gains real Focus / Open-Terminal / Dismiss actions instead of a placeholder "external terminal" label; the Focus button on a composite-tab-hosted terminal now sets both `activeTabIndex` and the composite's `activeSegmentIndex` so the user actually sees the terminal; the merged-sessions list dedupes by `(provider, cwd)` for active-interactive rows so a single Claude process doesn't show as two entries with different IDs. Tier/focus/resume logic extracted to a shared `aiSessionActions.ts` so list and detail share one implementation.

### Testing

- **Rust:** 1 034 tests across the workspace, clippy clean on `--workspace --all-targets -- -D warnings`.
- **Frontend:** 639 Vitest tests across 90 files.
- **svelte-check:** 0 errors / 0 warnings across 2 989 files.

## [0.1.8] — Phases 6–10: Bisect, CLI Auth, AI Stack, Forge Integration, Bundled CLIs, Refactor, E2E, Performance

The biggest release since the MVP — everything since `v0.1.7-beta` ships in one cut. Five phases of feature work plus a deep architecture and performance pass: visual bisect, CLI auth, the full AI stack (three providers, headless background runs in worktrees), GitLab + GitHub forge integration with bundled CLIs, the provider architecture cleanup, and the E2E + tracing infrastructure.

### AI Background Worktree Runs (Phase 10)

Launch a headless AI coding run inside a fresh git worktree without opening a terminal. Three entry points: tab bar button, AI Sessions header, and `Cmd+Shift+A`. Prompt source: free text, saved prompt from `.claude/prompts/`, or skill from `.claude/skills/` (user or project scope). Provider: Claude Code, Codex, or OpenCode. Worktree root configurable (default `.beardgit/ai-worktrees`); concurrency cap configurable (default 3) with FIFO queueing past the cap.

- **`ai-provider`** — `AiBackgroundRunInput` + `AiBackgroundRunStatus` + `AiTokenUsage` types; `launch_background` trait method with `NotSupported` default; `MockProvider` override for tests.
- **`claude-code` / `codex` / `opencode`** — headless command builders with provider-specific flags (Claude: `--print --output-format stream-json --verbose`; Codex/OpenCode: prompt concatenation fallback where skill/prompt flags aren't native).
- **`task-runner`** — `TaskKind::AiBackground` variant + `spawn_with_options` with stdin piping (backwards compatible — `spawn()` unchanged).
- **`app-core`** — `AiBackgroundCoordinator` with full lifecycle (Queued → Running → Completed / Failed / Cancelled), concurrency cap enforcement, worktree creation via `git-engine` and cleanup on discard. 6 Tauri commands + 2 settings commands.
- **`git-engine`** — `create_worktree_at` helper used by the coordinator.
- **`storage`** — `AppConfig` gains `ai_worktree_root`, `ai_background_concurrency_cap`, `ai_prompt_auto_accept` fields with serde defaults.
- **Frontend** — `CreateBackgroundRunDialog` with Free / Saved / Skill tabs, `BackgroundRunStatusBadge`, `BackgroundRunTranscript` with ANSI stripping, session detail + list integration, settings card, ~50 i18n keys per locale. `aiBackground.ts` store wires `ai-background-output` / `ai-background-status` events via `requestAnimationFrame` batching (matches the `tasks.ts` pattern); merges live runs into `aiSessions` for unified sidebar display.
- **Testing** — 13 tests in `app-core::ai_background` (coordinator lifecycle + cap + cancel + discard), 5 in `ai-provider`, 3 per provider for argv builders, 7 vitest tests for the store.

Known follow-ups for a later release: "View changes" button (deferred — merge-editor expects a conflict state, current release ships "Switch to worktree tab" as the review path), toast notification event wiring (i18n keys present), and an end-to-end spec exercising the full dialog (placeholder at `e2e/specs/regression/ai-background.spec.ts`).

### Beta Audit — Performance & Code Quality

A bundled audit pass landing 15 fixes from the beta-audit spec — the highest-leverage cleanup before tagging the release.

**Performance (high impact)**
- Cache `which::which()` results per provider kind on `AppState` — repeated provider detection no longer hits the filesystem.
- Replace task polling loops with `TaskManager::wait_for_terminal` backed by `tokio::sync::Notify` — no more spin-wait on long-running tasks.
- Memoise `Arc<dyn ForgeProvider>` keyed on `(provider_index, project_path)` — repeated forge lookups skip the construction cost.

**Correctness**
- Populate the GitLab label cache so issue labels render with their real colour.
- Unify `MrPr.labels` with `Issue.labels` on `Vec<Label>`; `PillRow` now renders the real label colour everywhere.
- Drop redundant `refreshIssueList` calls on label / assignee / milestone mutations — the optimistic update already covers it.
- Route `resolve_startup_theme` through `src/lib/api/tauri.ts` for consistency with every other IPC call.
- Key `#each` blocks over MR/PR diff files + comment lists for stable Svelte reconciliation.

**Code quality**
- Extend the trait-crate purity CI guard to include `ai-provider` (alongside `provider` and `forge-provider`).
- Share `build_gh_upload_args` / `build_glab_upload_args` across crates instead of duplicating the argv shape.
- Add `TaskManager::get_status` and a frontend `taskById` derived map for O(1) status lookup.
- Move `shell_escape` into `helpers.rs` with unit tests.
- Rename `MrPrComment` to `ForgeComment` in TypeScript (deprecated alias kept for one release).
- Extend `MrPrFilter` with author / label / text fields, matching `IssueFilter`.
- Rename the `render:text` perf measure to `render:badges-and-text` to match what it actually measures.

### GitLab Provider Polish

- **Per-file +/- counts** — `projects/:id/merge_requests/{n}/diffs` returns the raw patch but no additions/deletions counts. We were hardcoding 0/0, which showed as "+0 -0" beside every file in the MR detail panel. New `count_patch_changes` parser counts `+` / `-` content lines while skipping `+++` / `---` file headers and `@@` hunk headers. 4 unit tests.
- **`glab mr list` boolean state flags** — glab (both 1.46.1 and 1.92.1) does not accept `--state <value>`; passing `--state opened` made glab reply "Unknown flag" and our list returned empty. Switched to the boolean form glab actually supports: default → opened, `--closed`, `--merged`, `--all`. Dropped the unused `state_to_glab_str` helper.
- **Provider-aware sidebar label** — sidebar "Merge Requests" now reads "Pull Requests" when the active provider is GitHub. View id stays `merge-requests` so routing is unchanged; only the label swaps. New `sidebar_pull_requests` i18n key.
- **MR/PR list errors are surfaced** — `refreshMrPrList()` no longer swallows failures. Errors go to a new `mrPrListError` store and `MrPrList` renders them inline with a Retry button instead of an empty list with no explanation.

### Settings — Connection Guide

- **"How to connect" guide** — collapsible help block in Settings → Connection covering standard gitlab.com / github.com setup (PAT + CLI flows), self-hosted GitLab with OAuth and token fallback, plus troubleshooting for the multi-config warning and the 404-when-self-hosted-points-at-gitlab.com trap.

### Distribution

- **macOS x64 dropped from the release matrix** — Apple Silicon runners are now the only macOS target. Reduces CI matrix time and avoids the dual-bundle confusion at install time.
- **Bundle formats trimmed** — `.msi`, `.deb`, and `.rpm` removed from the bundle list. The `.dmg`, `.AppImage`, and `.exe` remain as the supported install paths per platform.
- **First-launch documentation** — README now explains the unsigned-build workaround for macOS until code signing lands (Gatekeeper right-click → Open). E2E fixture path no longer pins to a hardcoded location; derived from the working directory at runtime so contributors can run the suite from anywhere.
- **Repository hygiene** — AI assistant artifacts (`.claude/`, `.codex/`, etc.) untracked from the repo and added to `.gitignore`.

### Forge Integration (Phase 8)

Full daily-dev-workflow parity with GitHub and GitLab web UIs, behind a clean provider abstraction.

**MR/PR Enhancements (8.2)** — 11 new forge methods. Add/remove labels and reviewers post-creation, mark draft ↔ ready, reopen closed MR/PRs, resolve / unresolve GitLab discussion threads, and check out an MR/PR branch locally via `TaskManager`-streamed CLI output. The detail panel gains `LabelPicker`, `ReviewerPicker`, draft toggle, reopen button, per-comment resolve controls, and a "Checkout locally" action.

**Issues (8.3)** — Complete issue management as a new sidebar vertical. List / get / create / edit / close / reopen / comment plus assignees, labels, and milestones via 13 new trait methods. New `IssueView`, `IssueList`, `IssueDetail`, `CreateIssueDialog`, `AssigneePicker`, `MilestonePicker` components. Generic shared `LabelPicker` and `Xrefs` components plus a new `xrefs.ts` utility that auto-links `#NNN`, `!MMM`, `@user`, and short SHAs inside any text body.

**CI/CD Control (8.4)** — Actions on top of the existing Pipelines view. Trigger, retry, retry-failed-only, per-job retry, cancel, and list-workflows via six new `CiProvider` methods implemented over reqwest. `PipelineList` gets a "Run Workflow" button and row context menu; `PipelineDetail` gains action buttons and per-job retry. New `TriggerWorkflowDialog` with dynamic input form. GitHub PAT `workflow` scope hint surfaced in provider setup.

**Releases (8.5)** — Release management as its own vertical. 9 new trait methods including asset upload that streams via `TaskManager` for non-blocking progress. New `ReleaseView`, `ReleaseList`, `ReleaseDetail`, `CreateReleaseDialog`, `AssetUploadProgress` components plus an atomic `create_tag_and_release` flow that pushes the tag and creates the release in one streamed task. The cross-reference parser is extended to recognise release tags against a live tag cache.

**ForgeProvider Trait (8.1)** — New `forge-provider` crate extracts the `ForgeProvider` trait and shared types (`MrPr`, `Issue`, `Release`, `Label`, `User`, `Milestone`, `Comment`, …) with a `ForgeError` enum. `cli-provider` split into `GitHubCli` and `GitLabCli` structs each implementing the trait. `build_forge_provider(AppState) → Arc<dyn ForgeProvider>`. Zero user-visible change; foundation for 8.2–8.5.

### Bundled CLI Binaries (Phase 7.2)

BeardGit now ships `gh` (v2.62.0) and `glab` (v1.46.1) as Tauri sidecars on macOS arm64, Linux x64, and Windows x64 — no manual install required. `scripts/download-cli-binaries.js` pulls pinned binaries from the official release URLs; the Build and Release pipelines fetch the matrix-specific target before `tauri-action`. `resolve_cli_binary()` checks the sidecar location first and falls back to the system PATH, so existing installations keep working. Validated end-to-end across all four platforms.

### Terminal Enhancements (Phase 7.1)

- **OSC 7 cwd auto-detection** — terminals emit their current working directory on every prompt; when the cwd matches an open project path, the terminal tab auto-links to that project's composite tab.
- **AI provider auto-detection** — a lightweight polling loop detects when a terminal launches `claude`, `codex`, or `opencode` and updates the tab label plus brand icon dynamically.

### UI Polish & Bug Fixes (Phase 7.6)

- **Bisect graph integration** — good/bad/current/skipped commits get colored overlays in the canvas graph; right-click a commit for "Mark as good / bad / skip".
- **Worktree lock / unlock** — full `git worktree lock` / `unlock` wired through `git-engine` with context menu controls.
- **Worktree "Open in graph"** — navigates the graph view to the worktree's branch.
- **AI Config Editor live reload** — new `watcher::ai_config` module picks up external edits to `settings.json`, `agents/*.md`, `skills/*/SKILL.md`, and `CLAUDE.md` files; the editor refreshes without losing in-progress changes.
- **AI Sessions "Focus"** — focuses the linked terminal tab if the session has one; otherwise launches `claude --resume <sessionId>` in a new PTY terminal.

### Infrastructure (Phase 7.5)

- **Log rotation** — `storage::logging::purge_old_logs()` auto-removes `beardgit.*.log` files older than 7 days on startup (async, non-blocking). Legacy `beardgit.log.*` files from pre-rename installs are also purged by age.
- **Tracing on git writes** — 41 `#[instrument]` spans on `git-engine` write operations (bisect / operations / conflict / reset / clean / remote / worktree / submodule / interactive_rebase). Sensitive fields (commit bodies, PR descriptions, PAT tokens) excluded via `skip(...)`.
- **Tracing on Tauri commands** — 80 `#[instrument(name = "cmd::…")]` spans across 19 command modules. Hierarchical names make log grepping trivial.

### Performance (Phase 7.7)

- **Graph render profiler** — six `performance.mark` pairs around the render loop plus a dev-only FPS overlay toggled with `Ctrl+Shift+P`. Measurement infrastructure for future optimisations without runtime overhead in production bundles.
- **Interactive terminal pool** — 3-deep `xterm.js` instance pool recycles terminals across tab open/close. Faster tab spawn, lower GC pressure.
- **CodeMirror language cache** — module-level `Map<string, Extension>` short-circuits repeated dynamic imports per file extension. Second-and-subsequent opens of a language are instant.

### Code Quality (Phase 7.3)

The remaining items from Phase 6.3 plus anything picked up along the way. Generic `<List>` component now backs 10 consumers (Branch / Tag / Stash / Reflog / MrPr / Worktree / Submodule / Release / Issue / AiSession). `fetchIntoStore` / `fetchListIntoStore` / `fetchPageIntoStore` helpers consumed by 10 stores. Two residual `serde_json::from_str` call sites in `cli-provider/src/{github,gitlab}/mr_pr.rs` swapped for the shared `run_json` helper.

### E2E Testing (Phase 7.4 + follow-up)

Full WebdriverIO + `tauri-driver` suite covering every major vertical. 9 spec files, ~53 tests: app-launch, navigation, golden-path, and regression suites for graph / branches / staging / terminal / bisect / settings. 6 new page objects, data-testid attributes across the UI, and a Linux `e2e-tests` job in `ci.yml`. Follow-up pass in the same release cycle fixed every layer end-to-end: ESM `__dirname` shim for wdio v9, specs glob resolution, tauri-driver hostname/port, workspace-root binary path, `VITE_BEARDGIT_E2E` frontend hook, and switching to `tauri build --debug --no-bundle` so the frontend actually embeds. A new Docker harness (`e2e/Dockerfile` + `npm run e2e:docker`) lets macOS contributors run the full suite locally in ~1–2 min per iteration.

### Provider Architecture Cleanup (Phase 9)

Pure refactor. `provider/lib.rs` (883 LOC of trait + types + kind + error) split into `traits.rs` / `types.rs` / `kind.rs` / `error.rs` / `http_helpers.rs` / `mock.rs`; `lib.rs` is now 43 LOC of re-exports. `cli-provider/src/{github,gitlab}.rs` (~800 LOC each) converted to directory modules with per-vertical submodules (`mr_pr`, `labels`, `reviewers`, `lifecycle`, `discussions`, `checkout`, `issues`, `releases`). The `impl ForgeProvider` block stays in `mod.rs` as pure delegation to feature-scoped methods — no file exceeds 400 LOC. A CI grep guard in `ci.yml` enforces that `provider` and `forge-provider` never import `reqwest`, `tokio`, `tauri`, or `hyper`. Shared HTTP primitives (`api_error`, `retry_after_secs`, `trim_base_url`) extracted into `provider::http_helpers` and consumed by both `gitlab-api` and `github-api`. `crates/CLAUDE.md` refreshed with the new layout and an "Adding a new forge capability" walkthrough.

### Security

- `npm audit --audit-level=moderate` clean. Override added for `serialize-javascript` (wdio transitive dep) that upstream hadn't patched yet.

### Tooling

- `npm run e2e:docker` (plus `:rebuild` and `:shell`) — one-command local E2E.
- `e2e/README.md` documents the happy-path authoring pattern so test authors have a template.

---

### Phase 6 — Bisect, CLI Auth, AI Views, Multi-Provider, Code Quality

(Previously drafted as a standalone `[0.1.8]` release; folded into the unified `[0.1.8]` cut since it never tagged separately.)

**Git Bisect**

- Visual bisect workflow with good/bad/skip controls and progress indicator
- Auto-bisect mode: provide a test command, BeardGit runs `git bisect run` and reports the culprit
- New `git-engine` bisect module with full lifecycle (start, good, bad, skip, reset, log)
- 8 new Tauri commands, dedicated store, 2 Svelte components (BisectWorkflow, AutoBisectDialog)

**CLI Auth (gh/glab)**

- `gh auth status` and `glab auth status` detection — shows CLI login state in Settings
- Terminal-based login flow: "Login with CLI" opens interactive `gh auth login` / `glab auth login` in a PTY tab
- Unified Authentication settings page combining Token Auth and CLI Auth sections
- New `cli-provider` auth module with status parsing and terminal login commands

**AI Config Editor**

- Dual file tree (project-scoped + user-scoped) showing all AI config files
- Editable CodeMirror pane for settings.json, agents, skills, and CLAUDE.md files
- Create Config dialog for adding new agent/skill/settings files
- 3 new Tauri commands: `ai_get_config_content`, `ai_save_config_content`, `ai_create_config_file`

**AI Sessions**

- Project-scoped session list showing active and recent Claude Code sessions
- File watcher on `~/.claude/sessions/` with auto-refresh on changes
- Session metadata: model, start time, duration, token usage, status (active/completed)

**AI Worktree Enrichment**

- `EnrichedWorktree` type combining git worktree data with AI provider status
- AI badges on worktrees created by Claude Code / Codex / OpenCode
- Context menu with cleanup action for orphaned AI worktrees

**Codex & OpenCode Providers**

- New `codex` crate: full `AiProvider` implementation with binary detection, command building, and config discovery
- New `opencode` crate: full `AiProvider` implementation with binary detection, command building, and config discovery
- Both wired into `app-core` provider factory with automatic detection
- Dynamic terminal dropdown: only shows providers detected on the system
- Codex brand color corrected (#10a37f → #ffffff)

**Structured Error Logging**

- Structured file logging via `tracing` with `tracing-appender` daily rotation
- Logs written to `~/.local/share/com.beardgit.app/logs/` (platform-appropriate data dir)
- New `ErrorDialog` component with copy-error-to-clipboard and open-log-file actions
- All dialogs (Confirm, Clean, CreateMrPr, PatchPreview, TagCreate, CreateWorktree) upgraded with error display

**Composite Tab Upgrade**

- Multi-segment tabs: N terminals + worktrees per project in a single composite tab
- Fixed segment ordering: Project → Worktrees → AI Terminals → Terminals
- Terminal button always adds to the active project's composite tab instead of creating standalone tabs

**Code Quality — commands.rs Split**

- Split monolithic `commands.rs` (3,267 LOC) into 24 feature-based modules under `commands/`
- Modules: advanced, bisect, branch, ci, clean, cli_auth, commit, config, conflict, diff, gitignore, graph, helpers, logging, mod, mr_pr, patch, project, provider_auth, reflog, remote, repository, settings, staging, stash, submodule, tag, theme, worktree
- Extracted shared `dialog.css` (93 lines) replacing duplicated dialog styles across 7 components
- New `fetchIntoStore` utility for consistent store-loading patterns

**E2E Test Infrastructure**

- WebdriverIO + `tauri-driver` configuration for end-to-end testing
- Fixture repo setup script (`e2e/fixtures/setup.sh`) for reproducible test environments
- Page objects: `sidebar.page.ts`, `graph.page.ts`
- Initial specs: `app-launch.spec.ts`, `navigation.spec.ts`

**Bug Fixes & Polish**

- AI Config file tree correctly distinguishes project vs user scope
- AI Sessions auto-cleanup on component destroy (watcher unsubscribe)
- CreateConfigDialog validates file paths and prevents duplicates
- Store helpers centralized with `fetchIntoStore` reducing boilerplate across stores

## [0.1.7] — AI Provider Integration, Changes Redesign, UI Polish

**AI Provider Architecture**

- New `ai-provider` crate: `AiProvider` trait with 17 methods across 7 capability groups (identity, detection, headless execution, specialized actions, interactive launch, session/worktree introspection, config/attribution)
- Shared types: `AiProviderKind`, `AiSession`, `AiWorktree`, `AiConfigFile`, `ExecuteOptions`, `AttributionPattern`
- Trait builds `std::process::Command` objects without executing — execution delegated to `TaskManager` (headless) or `TerminalManager` (interactive)
- Default implementations return empty/None/NotSupported — providers override what they support

**Claude Code (First Provider)**

- New `claude-code` crate implementing `AiProvider` for Claude Code CLI
- Binary detection via `which` + version parsing from `claude --version`
- Repo artifact detection (`.claude/` directory, `CLAUDE.md` file)
- Headless command builder: `--print`, `--output-format`, `--model`, `--max-budget-usd`
- Interactive launch: spawns `claude` binary directly in PTY terminal
- Worktree support: `--worktree [name]` flag
- Session introspection: parses `~/.claude/sessions/*.json`, PID liveness checks (`kill(pid, 0)` on Unix)
- Worktree introspection: `git worktree list --porcelain` parser, filters `worktree-*` branches, status detection (Active/Clean/Orphaned)
- Config discovery: user/project/local settings.json, `.claude/agents/*.md`, `.claude/skills/*/SKILL.md`, CLAUDE.md hierarchy
- Commit attribution: detects `Authored-by:` footer, `Co-authored-by:` trailer with Claude/Anthropic mention, author name matching

**16 Tauri Commands**

- Detection: `ai_get_providers`, `ai_get_repo_status`, `ai_refresh_detection`
- Headless actions (via TaskManager): `ai_generate_commit_message`, `ai_analyze_code`, `ai_generate_pr_description`, `ai_review_code`, `ai_review_pr`
- Interactive launch (via TerminalManager): `ai_launch_interactive`, `ai_launch_worktree`
- Introspection: `ai_list_sessions`, `ai_list_worktrees`, `ai_cleanup_worktree`, `ai_get_config_files`
- Preference: `ai_get_preferred_provider`, `ai_set_preferred_provider`

**AI Provider Settings**

- New "AI Provider" section in Settings replacing the WIP "Editor" section
- Shows all known providers (Claude Code, Codex, OpenCode) with detection status
- Detected providers show version and "Detected" badge; unavailable ones are greyed out
- Click to set default provider, click again to reset to auto-detect
- Preference persisted in `AppConfig.preferred_ai_provider` across restarts
- Refresh button to re-scan PATH for provider binaries

**AI Button Validation**

- AI Commit Message button now shows a warning toast when no staged changes exist
- AI Code Review button now shows a warning toast when no changes exist at all
- Previously both buttons silently triggered tasks with no input

**Terminal AI Launch**

- Terminal dropdown "Claude Code" now calls `ai_launch_interactive` — spawns the `claude` binary directly in PTY (Claude Code starts automatically)
- Terminal tabs show Claude Code SVG brand icon (coral `#d97757`) instead of generic terminal icon
- Brand-colored status dots: Claude (#d97757), Codex (#10a37f), OpenCode (#8b8b8b)
- Same icon treatment in both standalone `TerminalTab` and composite tab terminal segments
- `TerminalTabInfo` extended with optional `provider` field for brand identification

**Changes Section Redesign**

- Pinned commit box at bottom with toolbar row: amend toggle, AI buttons, overflow menu
- AI Commit Message button (purple accent) with loading spinner; Code Review button (blue accent)
- Overflow menu: Create Patch, Clean, History (reflog), Push — replacing scattered buttons
- Commit message textarea with Cmd+Enter shortcut
- Single commit button replacing separate stage+commit actions

**Reflog Section Overhaul**

- Fixed broken "Create Branch" context menu action — was creating branch at HEAD instead of at the reflog entry's commit. New `create_branch_at(name, oid)` backend operation
- Fixed misleading "Checkout" action — was performing `reset --mixed` (destructive). New `checkout_detached(oid)` backend operation for proper detached HEAD checkout
- Fixed selection model — `selectedReflogOid` used just the OID which is not unique across reflog entries. Switched to index-based selection
- Removed duplicate `repo-changed` listeners — SplitView now handles lifecycle exclusively
- Added action buttons to detail pane: Checkout, Create Branch, Reset (dropdown with Soft/Mixed/Hard), Copy SHA
- Added refresh button to list header
- Context menu actions now refresh the reflog list after operations
- Selection cleared when navigating away to prevent stale state on return
- File diff panel: clicking a file in the reflog commit detail now shows a resizable diff editor below

**Submodule Management — Add & Remove**

- New "Add Submodule" button in header — opens inline form with URL and path inputs
- New `add_submodule(url, path)` backend operation (`git submodule add`)
- New "Remove Submodule" in right-click context menu with confirmation dialog
- New `remove_submodule(path)` backend operation (`git submodule deinit -f` + `git rm -f`)
- Empty state no longer blocks the "Add Submodule" button

**UI Polish**

- Folder icons changed from orange to blue for better visual cohesion
- Tab badge style changed from solid orange pill to subtle green tint with green text
- Tab hover tooltips with project snapshot (branch, changes, last commit)
- Project snapshot cache for instant tooltip display
- Task panel command bar truncated to single line with ellipsis (fixes output being pushed off-screen by long AI commands)

**Bug Fixes**

- Fixed task panel output not visible when AI commands have long prompts (command bar had no max-height)
- Fixed `width: 100%` missing on SplitView — right pane not reaching container edge in flex layouts
- Fixed graph tooltip positioning and content
- Fixed terminal resize on tab switch
- Fixed project switch clearing stale data (reflog, conflict state, diffs)
- Fixed unstaged file diff preview not loading after project tab switch
- Removed gitignore editor component (functionality preserved via context menu)

**E2E Test Infrastructure**

- Global vitest setup mocking `@tauri-apps/api/core`, `@tauri-apps/api/event`, `@tauri-apps/api/window`, `@tauri-apps/plugin-dialog`
- Configurable `mockInvokeResponse()` helper for per-test IPC mocking
- 6 E2E workflow test suites: repo-open, staging-commit, branch-ops, tag-ops, stash-ops, ai-provider
- 103 new tests (149 total frontend tests, all passing)

## [0.1.6] — Interactive Terminal Tabs, Composite Tabs, Sidebar Collapse

**Composite Segmented Tabs**

- Project + linked terminal merge into a single segmented pill tab: `[● Repo | ⌨ Terminal]`
- Each segment independently clickable, closeable (hover-only ✕), and middle-click closeable
- Closing a segment reverts the composite to a simple tab (project-only or terminal-only)
- Terminal opens in-place — project tab is promoted to composite, not a new tab at the end
- Shell exit auto-removes the terminal segment, reverting to a simple project tab
- Cmd+W closes the active segment of a composite tab (not the whole tab)
- Standalone terminal tabs remain for "New terminal in ~" (not linked to any project)

**Interactive Terminal Tabs**

- Full interactive xterm.js terminal wired to Rust PTY backend (keyboard input, resize, base64 byte streaming)
- Terminal split button in the actions area: left (terminal icon) opens terminal, right (chevron) opens dropdown
- Dropdown options: "New terminal in ~", Claude Code, Codex, OpenCode — with official SVG brand logos and hardcoded brand colors (#d97757, #10a37f, #8b8b8b)
- Claude logo uses official Anthropic symbol (CC0 public domain from Wikimedia Commons)
- NerdFont icons render correctly in terminal (NerdFontSymbols added to xterm.js fontFamily)
- Cmd+T shortcut to open a new terminal tab
- Terminal tabs auto-close when the shell process exits
- Fetch/Pull/Push buttons hidden when a terminal tab is active

**Sidebar Collapse**

- New collapse toggle button at bottom of sidebar with chevron icon
- Collapsed mode: icon-only (44px width) with smooth 150ms CSS transition
- Tooltips on hover when collapsed
- Cmd+B keyboard shortcut to toggle
- Collapse state persisted in AppConfig across restarts

**Performance**

- Graph viewport cached per project — instant tab switching with no loading spinner for the graph view
- Auto-navigate to graph on project tab switch — prevents stale pipeline/changes data from previous project

**Bug Fixes**

- Fixed: recent projects list empty on first use — now populated when opening a project, not just when closing one
- Fixed: unstaged file diff preview not loading after project tab switch (diffs now auto-refresh on file click)
- Fixed: close button icons inconsistent — standardized to `\uF00D` (nf-fa-times) across all tabs and panels
- Fixed: + button icon inconsistent — standardized to `\uF067` (nf-fa-plus)
- Fixed: icons not vertically centered in Fetch/Pull/Push/Terminal action buttons
- Fixed: tab close buttons oversized with circle hover — now smaller, highlight-only on hover
- Fixed: + button popup not closing when clicking outside
- Sidebar navigation from a terminal tab automatically switches to the most recent project tab

## [0.1.5] — Terminal Core + Theme Redesign

**Terminal Core (xterm.js) + Theme Redesign**

- New `terminal` Rust crate with PTY lifecycle management via `portable-pty`
- Cross-platform shell detection (zsh/bash on Unix, powershell/cmd on Windows)
- `TerminalManager` with spawn, write, resize, kill, kill_all operations
- Tauri commands and event bridge for terminal sessions (base64-encoded byte streaming)
- Reusable `<Terminal>` Svelte component (xterm.js with WebGL, fit, web-links, search addons)
- Read-only xterm.js instance pool (max 3: 2 visible + 1 warm) for zero-lag view switching
- TaskPanel output migrated from manual ANSI-to-HTML to xterm.js read-only terminal
- JobLog (CI pipeline logs) migrated from manual ANSI-to-HTML to xterm.js read-only terminal
- Theme system redesigned: 18 base colors (background + foreground + 16 ANSI) replace 12 semantic colors
- All 14 TOML themes updated with explicit ANSI color palettes
- Semantic UI colors now auto-derived from base palette (DerivedColors struct)
- Direct xterm.js ITheme mapping from base colors (no derivation needed for terminal)
- Retired `ansi.ts` (250+ lines) — replaced by native xterm.js rendering + lightweight `stripAnsi()` utility

**Auto-Update System**

- Tauri updater plugin checks GitHub Releases for updates on app launch
- Two-step update flow: toast notification → Download → Restart (non-disruptive)
- Download progress shown in toast with percentage
- Updater signing keys configured in CI release workflow

**Toast Notifications**

- Reusable toast notification system (bottom-right, max 3, stackable)
- Types: success, error, warning, info with auto-dismiss
- Used by auto-updater, extensible for future notifications

**Multi-File Selection in Changes**

- Per-file checkboxes in both staged and unstaged file lists
- Select All header checkbox with indeterminate state
- Header action swaps contextually: Stage All / Stage Selected (N) and Unstage All / Unstage Selected (N)
- Selection clears on refresh

**Bug Fixes**

- Commits now use git config identity (user.name/user.email) instead of hardcoded author
- Untracked directories show individual files instead of collapsed folder entry (recurse_untracked_dirs)
- README prerequisites and architecture table accuracy fixes

## [0.1.4] - 2026-04-09 — UI Polish, Layout Consistency & Bug Fixes

**3-Way Merge Editor (IntelliJ-style)**

- Full 3-panel layout: Theirs (Incoming) | Result | Ours (Current)
- Custom 3-way diff engine with LCS-based line alignment and chunk classification
- Non-conflicting changes auto-applied to the result on open
- Conflict placeholder lines in center with accept/ignore buttons on each side
- SVG bezier connector curves between panels linking conflict regions visually
- Hybrid curves: filled bezier when sparse, thin connector lines when dense (> 4 conflicts)
- Dynamic connector gap width (24px normal, 40px for many conflicts)
- Color scheme: green (added), purple (conflict), blue (center placeholder), active highlight (brighter)
- Chunk-aware scroll sync: center drives side panels based on line mapping, not proportional
- Side panel wheel events redirected to center for consistent behavior
- Smooth scroll animations on side panels during sync
- SVG connectors update on scroll, accept/ignore, undo, and window resize
- Undo support: Cmd/Ctrl+Z undoes accept/ignore operations, toolbar undo button
- Toggle line numbers button (# icon) for all three panels
- Prev/Next conflict navigation scrolls all panels aligned with active highlight
- Mark Resolved button: grey when disabled, green when all conflicts resolved
- Warning popup when resolving with conflict markers still present
- Cancel button with red destructive styling
- Syntax highlighting in all panels (language-aware via filename)

**Merge Request / Pull Request Improvements**

- List layout aligned with pipeline section pattern (3-column horizontal rows with state icon, title, time)
- Removed filter tabs (Open/Closed/Merged/All), replaced with SearchBar state filter (default: state:open)
- Added search/filter bar with state, author, branch, and label filters
- Markdown rendering in descriptions and comments (minimal parser + allowlist-based XSS sanitizer, links open externally)
- Redesigned merge action buttons: split-button with dropdown menu for merge strategy (merge/squash/rebase)
- Added refresh button and "no provider" empty state
- Provider readiness guard prevents empty list on startup

**Layout Consistency**

- Migrated Reflog view to SplitView (resizable sidebar, consistent with Tags/Stash/Branches/MR)
- Migrated Pipelines view to SplitView (replaces custom resize logic in +page.svelte)
- Pipeline job log pane is now resizable with a drag handle and has a close button
- Standardized icon-only buttons across all views: 14px, no border, color-only hover (refresh, close, nav buttons)
- Worktree, CommitDetail, DiffEditor, StagingDiffEditor, BlameView buttons all aligned
- Tag push button uses green hover (from theme --accent-green) consistently in both list and detail
- Worktree delete button: same color as others by default, red highlight on hover only
- Graph header separator line now reaches full width (border on container, not SearchBar)

**Git Config Editor**

- Empty values show italic "empty" label in light grey instead of em dash
- Clicking an empty field and typing nothing no longer saves an empty value
- Tooltips use i18n keys instead of hardcoded English

**Task System**

- Task popover now appears correctly (fixed position + click-outside race condition)
- Task output loaded from backend on selection (fixes empty output for completed tasks)
- Panel output shows executed command at top ($ git fetch origin)
- Three distinct empty states: "Select a task", "No output", output content
- Correct NerdFont icons for expand/collapse/close buttons
- Removed output preview from popover (kept in full panel only)

**Authentication**

- GitHub/GitLab CLI OAuth login disabled until terminal integration (PAT-only for now)
- OAuth errors now shown to user instead of silent fallback to PAT

**Keyboard Shortcuts**

- `?` shortcut now works globally (even when editor is focused)
- Fixed shift-key matching for shortcuts like `?` that inherently need Shift
- Help overlay: Escape key closes the popup, larger fonts, bigger close button
- Sidebar highlight syncs with keyboard navigation (Cmd+1-6)

**Other Fixes**

- Reflog empty-state message properly centered
- Hardcoded hex colors replaced with theme variables (--accent-green, --accent-red) in tag buttons
- Markdown sanitizer uses allowlist approach; links get target="_blank"
- MR merge dropdown closes on click-outside
- Conflict marker regex handles Windows \r\n line endings
- MR filtered-empty state shows "No results match your filter" instead of generic message
- MR and reflog state cleared on project switch (prevents stale data)
- CreateMrPrDialog backdrop changed to button for a11y compliance
- All svelte-check warnings resolved (0 errors, 0 warnings)

## [0.1.3] - 2026-04-08 — Phase 3: Power Features + CLI Integration

**Task History Popup**

- Enriched TaskInfo with command string, start timestamp, and exit code
- Always-clickable status bar task area — visible even when no tasks are running
- Two-line card popup: colored status bar (green/red/orange/gray), label, command, duration, relative time
- Click any task to open full output panel

**Keyboard Shortcuts**

- Central shortcut registry with platform-aware modifiers (⌘ on macOS, Ctrl on Windows/Linux)
- Cmd+1-6 for view navigation, Cmd+Tab/Shift+Tab for tabs, Cmd+W to close tab
- Cmd+Shift+F/L/P for Fetch/Pull/Push, Cmd+Shift+S/U for Stage/Unstage all
- J/K for graph commit navigation, Home/End for first/last commit, / for search
- `?` opens cheat sheet overlay with all shortcuts grouped by category

**Reflog Viewer**

- New sidebar view showing HEAD reflog entries with action-specific icons (commit, checkout, rebase, reset, merge, pull)
- Detail panel reuses CommitDetail with "Show in Graph" navigation button
- Context menu: checkout commit, create branch, reset (soft/mixed/hard), copy SHA

**Clean (Untracked File Removal)**

- "Clean" button in staging area when untracked files exist
- Dialog with filter toggles: include directories, include ignored, only ignored
- Per-file checkboxes with select/deselect all
- Per-file "Delete untracked file" from right-click context menu
- Destructive action warnings on all delete operations

**Git Config Editor**

- New Settings section: two-column table showing Local (project) and Global (user) config side by side
- Dropdown selectors for known enum-type keys (core.autocrlf, pull.rebase, push.default, etc.)
- Free text input for all other keys
- Inline editing with Enter to save, Escape to cancel
- Add new entries, unset existing keys, filter by key name
- Collapsible read-only System config section

**Gitignore Management**

- Quick "Add to .gitignore" from untracked file context menu with smart pattern suggestions (filename, *.ext, exact path, directory/)
- Full CodeMirror editor in Settings with save/revert and dirty state tracking
- Basic syntax highlighting for comments and negation patterns

**Patch Management**

- Create patches from commits (graph context menu → native save dialog)
- Create patches from working tree changes (staged or unstaged)
- Apply patches with dry-run preview showing per-file stats
- Three-way merge fallback for conflicting patches — integrates with existing merge editor

**Submodules**

- New sidebar view listing all submodules with status badges (Uninitialized, Clean, Outdated, Dirty)
- Init, update (background task), deinit operations
- "Open in Tab" — opens submodule as a full project tab with all BeardGit features
- Context menu with all operations + copy path/URL
- "Update All" header button for batch update

**MR/PR Management (GitHub + GitLab)**

- New `cli-provider` crate wrapping bundled `gh` and `glab` CLIs (both MIT licensed)
- CLI OAuth as primary auth flow — opens browser, extracts token, stores in encrypted credential store
- PAT entry remains as fallback for restricted environments
- Full CRUD: list, view, create, edit, merge (merge/squash/rebase), close
- Code review: approve, request changes, general + inline comments
- MR/PR badges on graph commits for branches with open MR/PRs (purple pills)
- Create dialog with source/target branch, title, description, draft toggle, labels, reviewers
- Filter tabs: Open / Closed / Merged / All

**Windows DPI & Zoom Fixes**

- Replaced CSS `zoom` with Tauri native `webview.setZoom()` — fixes blurry fonts and layout overflow at >100% UI scale on Windows
- Added `-webkit-font-smoothing: antialiased` and `text-rendering: optimizeLegibility` for crisper text rendering
- Canvas graph detects DPI changes when moving between screens and re-renders at correct resolution
- Fixed canvas subpixel blurriness at fractional DPR values

**Graph UX Improvements**

- Row hover highlight (subtle transparent overlay)
- Standard cursor instead of pointer hand on graph rows
- Increased canvas font sizes by 1px across all text elements
- Fixed column resize hit zone misaligned with separator lines

**Tauri Native Migration**

- Replaced `-webkit-app-region: drag` CSS with `data-tauri-drag-region` HTML attribute
- Added `core:webview:allow-set-webview-zoom` capability permission

**Performance & Code Quality**

- All MR/PR CLI commands run on `spawn_blocking` — never block the Tauri async runtime
- Canvas draw batched with `requestAnimationFrame` via `scheduleDraw()` helper
- Keyboard shortcut handler uses `get()` instead of subscribe/unsubscribe per keydown
- Reflog auto-refresh debounced (300ms) on repo-changed events
- Extracted shared utilities: `shortOid()`, `configure_no_window()`, `run_blocking()` helper
- Added `GitError::CliError` variant — CLI failures no longer misuse `RepoNotFound`
- Stringly-typed MR/PR params replaced with proper enum types (`MrPrState`, `MergeStrategy`)
- Error handling added to CleanDialog and GitConfigSettings (visible error messages)
- `formatRelativeTimeMs` delegates to `formatRelativeTimeUnix` instead of duplicating

## [0.1.2] - 2026-04-07

**Hunk + Line-Level Staging**

- Stage, unstage, or discard individual hunks or specific lines within a hunk
- StagingDiffEditor with per-hunk and per-line checkboxes, select all/deselect all
- Backend builds unified diff patches from selections and applies via `git apply --cached`
- Discard with confirmation dialog (destructive action)

**Blame + File History**

- Blame view with per-line gutter annotations (author, OID, relative date)
- Commit grouping in gutter — consecutive lines from same commit share annotation block
- Click OID in gutter to reload blame at that commit
- File history panel with `git log --follow` — shows all commits that touched the file
- Rename detection — shows "renamed from" badge when file was moved
- Click any commit in history to view blame at that point in time
- Right-click any file in staging area or commit detail → "Blame" / "File History"

**Rebase**

- Non-interactive rebase from branch context menu ("Rebase onto this branch") and graph context menu ("Rebase current onto here")
- Confirmation dialog before rebase; conflicts route to merge editor automatically

**Interactive Rebase**

- Visual commit list editor from graph context menu ("Interactive rebase from here")
- Per-commit action dropdown: pick, squash, fixup, edit, drop
- Drag-to-reorder commits with color-coded left border per action
- Drop action shows strikethrough with reduced opacity
- Footer legend explaining each action
- Backend uses `GIT_SEQUENCE_EDITOR` to inject pre-built todo list

**3-Way Merge Editor**

- CodeMirror `unifiedMergeView` with ours as editable content, base as reference
- Inline accept/reject controls per changed chunk
- Prev/Next conflict navigation buttons
- "Mark Resolved" writes content to disk and stages the file
- Conflict toolbar now shows expandable clickable file list → opens merge editor
- Activated during any conflict operation (merge, rebase, cherry-pick, revert)
- Backend: `get_conflict_file_contents` reads ours/theirs/base from libgit2 index stages

**Graph Columns**

- Resizable columns — drag column separators to adjust width (min 50px), persisted across sessions
- New Email column (hidden by default) showing commit author email
- SHA column now hidden by default (toggleable from Columns dropdown)
- Column visibility and widths persisted to settings.json via new Tauri commands

**10 New Built-in Themes + Complementary Pairing**

- Dracula, One Dark Pro, Catppuccin Mocha, Catppuccin Latte, Nord, Tokyo Night, Solarized Dark, Solarized Light, Gruvbox Dark, Monokai Pro
- Total: 14 built-in themes (10 dark, 4 light)
- Complementary theme pairing for OS auto-switch — each theme maps to a light/dark counterpart so toggling OS appearance picks the right pair (e.g., Catppuccin Mocha ↔ Catppuccin Latte)

**Performance**

- `Arc<str>` for commit OIDs in graph-builder — eliminates ~10 String clones per commit in the 100K+ commit hot path
- GitLab stage grouping optimized from O(n²) to O(n) via HashMap index

**Code Quality & Deduplication**

- Replaced 30+ hardcoded CSS color values with theme variables across 5 components
- Added `--overlay-accent-*` CSS variables for consistent overlay theming
- Consolidated 3 inline date formatters into shared `formatDate()`/`formatDateTime()` utilities
- Replaced manual debounce in TagList with shared `debounce()` utility
- Deduplicated `normalize_github_url` (auth crate now imports from github-api)

**Bug Fixes**

- Fixed stale detail panels when switching repository tabs — graph, branch, tag, stash, blame, and worktree state now fully cleared on tab switch
- Conflict status now refreshed on repo tab switch
- Branch commit list not taking full available width (missing `min-width: 0`)
- Diff close button hidden when file path is too long (added overflow handling + `flex-shrink: 0`)
- npm security audit: resolved high-severity vite vulnerability, overrode cookie to ^0.7.0 (0 vulnerabilities)

**Settings**

- Removed Repository section (remote management) — will return with gh/glab CLI integration

**CI/CD**

- Release pipeline auto-syncs version from git tag — no manual version bumps needed
- Strips non-numeric pre-release suffixes for Windows MSI compatibility (e.g., `v0.1.2-beta` → `0.1.2`)

---

## [0.1.1] - 2026-04-07

**CodeMirror 6 Editor Engine**

- Replaced custom diff viewer with CodeMirror 6 — syntax highlighting for 16 languages (JS, TS, Rust, Python, CSS, HTML, JSON, YAML, Markdown, Java, Go, C/C++, SQL, XML, and more)
- Side-by-side diff view with collapsed unchanged regions via @codemirror/merge
- Line numbers in all editor and diff views
- Language auto-detection from file extension with lazy-loaded grammars

**Core Git Operations**

- Revert commits from graph context menu with confirmation dialog
- Amend last commit via toggle in staging area (pre-fills HEAD message)
- Reset to any commit: soft (keep staged), mixed (unstage), hard (discard all) from graph context menu
- Hard reset shows destructive warning with explicit confirmation

**Worktree Management**

- Sidebar section listing all worktrees with branch name, path, and status badges
- Create new worktrees with auto-suggested path and new/existing branch options
- Open worktree as a tab (reuses multi-project tab system)
- Remove worktrees with confirmation dialog

**Remote Management**

- Settings > Repository section showing configured remotes
- Rename and remove remotes with inline editing and confirmation

**Theme System Improvements**

- Simplified TOML themes: only `[meta]` + `[colors]` required (14 lines instead of 50+)
- Graph, editor, and syntax highlighting colors auto-derived from 12 base colors
- Optional `[graph]` and `[editor]` overrides for fine-tuning
- Syntax token colors derived from theme accent palette (keywords, strings, comments, functions, types, numbers, operators, properties)
- Updated themes README with full documentation for custom theme creators

**UI Improvements**

- UI Scale setting (80%–150%) in Settings > Appearance for font size control
- Ref badges in commit detail and graph rotate through accent colors (hash-based, deterministic)
- Fira Code font explicitly set in all CodeMirror instances

**Performance & Windows Fixes**

- All 22 git CLI-backed commands now run on background threads (async + spawn_blocking) — UI never freezes during git operations
- Added CREATE_NO_WINDOW flag on Windows to prevent CMD console flash when spawning git processes
- Covers: tags, stashes, diffs, conflict operations, remotes, worktrees

**Testing**

- Added vitest coverage configuration (@vitest/coverage-v8)
- 32 new Rust tests (theme derivation, file content, remote operations)
- 23 new frontend tests (diff utils, ref colors, editor theme, language support)
- Shared ref-colors utility extracted from duplicate implementations

---

## [0.1.0] - 2026-04-06

**Git Operations**

- Visual commit graph with canvas rendering (100K+ commits via virtual scroll)
- Staging area with file-level stage/unstage and commit
- Branch management: create, delete, checkout, merge, cherry-pick
- Branch view with folder tree, commit history, context menu, and inline commit detail panel
- Stash management: push, pop, apply, drop, per-file apply, diff preview
- Tag management: paginated list, create (annotated + lightweight), delete, push, inline file diff preview, clickable parent refs
- Side-by-side file diff panel with word-level diff highlighting and vertical resize handle
- Clickable ref badges on merge commits showing changed files
- Fetch, Pull, Push as background tasks with live output streaming
- Auto-refresh graph and branches after remote operations complete

**Conflict Detection**

- Detect MERGING / REBASING / CHERRY-PICKING / REVERTING state
- Amber status bar badge and full-width ConflictToolbar with Abort/Continue
- Conflict marker highlighting in diff viewer (ours/separator/theirs)
- Auto-refresh conflict status on repo-changed events

**Graph Features**

- Lane-segment + merge-curve architecture
- Sync-state-aware line styles: thick (pushed), thin (local-only), dashed (fetched)
- Lane recycling at MAX_LANES=8 cap with arrow indicators
- Configurable columns (author, date) with resize
- Author bold — your commits shown in bold (matches git config name/email + provider identities)
- HEAD branch highlighting — thicker line + subtle background tint
- Lane click selection — click a lane to focus it, everything else dims
- Lane hover feedback — cursor and subtle highlight when hovering over lanes
- Clickable parent OIDs — click parent SHA in commit detail to navigate
- Context menu on commits: copy SHA, copy message, create branch, cherry-pick, checkout

**CI Pipeline Integration**

- Multi-provider support: GitLab REST v4 + GitHub REST API
- Unified pipeline list with real-time polling (15s list, 10s detail, 3s logs)
- Job log viewer with full ANSI color rendering (256-color, true-color, bold/dim/italic/underline)
- CI log preprocessing — strips timestamps, stream codes, section markers; adds line numbers (GitLab + GitHub)
- Server-side filtering by branch, source, and status
- Auto-detect provider from git remote URL

**Multi-Project Tabs**

- Open multiple repos as tabs with lazy loading
- Tab persistence across app restarts
- Starship-style title bar with git status summary (ahead/behind/staged/unstaged/stash)
- "+" dropdown with recent repos and open folder

**Background Task System**

- Task manager with async spawn/cancel and output streaming
- Status bar indicator with running/failed states
- Task popover for quick glance, expandable panel for full log viewer
- rAF-batched output events to reduce GC pressure

**Theme System**

- 4 built-in themes: GitHub Dark, GitHub Light, GitLab Dark, GitLab Light
- Default follows OS light/dark preference with live reactive switching
- User-installable custom themes via TOML files in `~/.config/beardgit/themes/`
- Auto-generated README in themes directory documenting the TOML schema
- Theme selector in Settings with "Follow system theme" toggle
- All UI colors driven by CSS custom properties
- Graph canvas renderer fully themed (lane colors, badges, text, selection)
- CI status colors adapt to active theme

**Internationalization**

- English (en-US) and Spanish (es-ES) via Paraglide.js v2
- Compile-time typed message functions
- Language selector in Settings > Appearance

**Authentication**

- Encrypted credential storage (AES-256-GCM, machine-derived key via HKDF-SHA256)
- PAT validation for GitLab and GitHub
- Multi-provider auto-reconnect on app startup

**UI/UX**

- Custom app icon (BeardGit glasses + beard + git diamond)
- Pill-shaped tabs merged into toolbar bar
- Fira Code monospace font with ligatures
- Symbols Nerd Font Mono for icons throughout the UI
- Responsive viewport-relative layouts with clamp()/min()
- Minimum window size 900x600
- Right-click context menus on files and commits
- Reusable components: SplitView, FileChangeList, CommitDetail, ConfirmDialog, SearchBar, ContextMenu
- Shared CSS for consistent styling across all views
- Filesystem watcher with debounced auto-refresh

**Storage**

- SQLite database with versioned schema and commit cache
- JSON config with provider migration support
- TOML theme system (built-in + user-installed)

**CI/CD**

- GitHub Actions CI: frontend checks + Rust fmt/clippy/tests
- Multi-platform build pipeline: macOS (arm64 + x64), Linux x64, Windows x64
- Release pipeline with draft GitHub releases on version tags
- Weekly security audit (cargo audit + npm audit)

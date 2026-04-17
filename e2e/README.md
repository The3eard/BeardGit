# BeardGit E2E Tests

End-to-end tests driving the real Tauri app via `tauri-driver` + `WebdriverIO`. Because `tauri-driver` only supports Linux + Windows, macOS developers run the suite inside a docker container that mirrors the CI environment.

## Running locally

**Requirements:** Docker Desktop (macOS) or dockerd (Linux). Nothing else.

```bash
npm run e2e:docker            # full suite (one command)
npm run e2e:docker:rebuild    # force rebuild the image after Dockerfile edits
npm run e2e:docker:shell      # open an interactive bash inside the container
```

First run is slow (~10 min) because it builds Rust + frontend from scratch. Subsequent runs reuse named docker volumes for `cargo/registry`, `target/`, and `node_modules`, bringing iteration down to ~1–2 min per cycle.

On linux, you can also run natively without docker:

```bash
cargo install tauri-driver --locked
sudo apt-get install -y webkit2gtk-driver xvfb  # once
VITE_BEARDGIT_E2E=true npx tauri build --debug --no-bundle
tauri-driver & sleep 2
CI=true BEARDGIT_BUILD_TYPE=debug xvfb-run npx wdio run e2e/wdio.conf.ts
```

## Writing new happy-path tests

Every spec that touches repo state must open a fixture first:

```ts
import { openFixtureProject } from "../helpers/project";
import sidebar from "../pages/sidebar.page";

describe("My feature", () => {
    before(async () => {
        await $("aside.sidebar").waitForExist({ timeout: 10000 });
        await openFixtureProject("simple-repo"); // or bisect-repo / conflict-repo
    });

    it("does something", async () => {
        /* ... */
    });
});
```

**Available fixtures** (built fresh by `e2e/fixtures/setup.sh`, called by the wdio `onPrepare` hook):

| Fixture         | Shape                                       | When to use                                               |
| --------------- | ------------------------------------------- | --------------------------------------------------------- |
| `simple-repo`   | 10 commits, 3 branches (main, feature/\*), 2 tags | branches / graph / staging / most golden-path steps       |
| `conflict-repo` | 3 commits, 3 branches, merge conflict ready | merge-editor / conflict resolution flows                  |
| `bisect-repo`   | 20 commits, bug at commit 12                | bisect workflow                                           |

**Page object pattern**: see `pages/` for the existing objects (sidebar, graph, branches, changes, terminal, settings, bisect, dialogs). Prefer adding a method to the relevant page object over putting `$('selector')` calls directly in specs.

**Data-testid convention**: kebab-case, stable. Add `data-testid="foo-bar"` to Svelte components and target via `$('[data-testid="foo-bar"]')`. Dynamic values (`data-testid="nav-{item.id}"`) are fine — the generator emits literal strings at render time.

## How the harness works

Key files, in dependency order:

- `e2e/Dockerfile` — ubuntu 22.04 + webkit2gtk-driver + xvfb + tauri-driver + node 22 + rust stable
- `e2e/run-in-docker.sh` — container entrypoint: `npm ci` → download gh/glab → `paraglide compile` → `VITE_BEARDGIT_E2E=true tauri build --debug --no-bundle` → start Xvfb → start tauri-driver → `xvfb-run npx wdio run`
- `e2e/run.sh` — host wrapper; lazy-builds the image, owns the named volumes
- `e2e/wdio.conf.ts` — WebdriverIO config. Points wdio at `127.0.0.1:4444` (tauri-driver) with `tauri:options.application = target/<profile>/beardgit`. Specs glob resolved absolutely so cwd doesn't matter
- `e2e/helpers/project.ts` — `openFixtureProject()` proxies through `window.__E2E__.openProject()` (a frontend hook exposed in `+layout.svelte` only when `VITE_BEARDGIT_E2E=true`). This is what drives the UI into the "repo loaded" state; direct `open_project` IPC alone does not sync the Svelte stores.
- `e2e/specs/*.spec.ts` — the tests themselves, grouped into `app-launch`, `navigation`, `golden-path`, and `regression/*`

The same flow runs unmodified on GitHub Actions via `.github/workflows/ci.yml::e2e-tests` (main / beta pushes + PRs targeting those). Artefacts (junit.xml + failure screenshots) upload to the run summary.

## Troubleshooting

- **"No such file" for the beardgit binary** → the tauri build didn't complete. Check the build logs above the wdio output
- **"Could not connect to localhost"** in the webview → the app was built with `cargo build` instead of `tauri build`; plain cargo doesn't embed the frontend assets
- **"window.__E2E__.openProject is not available"** → the app was built without `VITE_BEARDGIT_E2E=true`. Rebuild.
- **"pattern … did not match any file"** → wdio v9 resolves specs relative to the config, not cwd. Keep the `path.join(__dirname, …)` pattern in `wdio.conf.ts`
- **Stale fixture repos** → delete `e2e/fixtures/` to force `onPrepare` to recreate them

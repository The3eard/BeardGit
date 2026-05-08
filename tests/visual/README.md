# Visual regression suite

Playwright + Vite dev server. Runs the SvelteKit UI in Chromium with a
mocked Tauri IPC layer, takes screenshots per view/state, and diffs
them against committed baselines.

The tests are intentionally **decoupled from the Rust backend** —
nothing is shelled out, no real repos are opened, no provider tokens
are needed. The mock IPC layer (`helpers/mock-ipc.ts`) intercepts every
`invoke()` call before the bundle loads and resolves it against an
in-page response map.

## Running

```sh
npm run test:visual              # diff against committed baselines
npm run test:visual:update       # regenerate baselines (after intentional UI changes)
npx playwright test tests/visual/components/changes.spec.ts   # one file
npx playwright test --ui         # debug interactively
```

The dev server is started/reused automatically by `playwright.config.ts`
on `localhost:1420`. CI re-spawns it; locally it's reused if already up.

A single retry is configured (`playwright.config.ts:retries`) to
absorb the occasional Vite dynamic-import flake when several workers
ramp simultaneously. If a test fails on retry it's a real regression.

## Layout

```
tests/visual/
├── helpers/
│   ├── mock-ipc.ts          # window.__TAURI_INTERNALS__ stub
│   ├── bootstrap.ts         # installBootstrapMocks + waitForAppReady
│   ├── themes.ts            # applyTheme + THEME_MODES
│   ├── nav.ts               # clickNav(label)
│   └── index.ts             # public re-exports
├── components/
│   ├── changes.spec.ts      # StagingArea per state
│   ├── mr-pr.spec.ts        # PR list + detail
│   ├── graph.spec.ts        # GitGraph with N-commit fixtures
│   ├── issues.spec.ts       # Issues list + detail
│   ├── branches.spec.ts     # Branches tree + commit list
│   ├── commit-detail.spec.ts # commit selection
│   └── atomic.spec.ts       # toasts (others covered in-app)
├── routes.spec.ts           # 16 sidebar views × dark/light
├── smoke.spec.ts            # mock-IPC harness self-tests
├── routes.spec.ts-snapshots/        # 32 baselines
└── components/<spec>.spec.ts-snapshots/   # per-spec baselines
```

The fixture factories live in `src/test/fixtures/` and are shared with
the Vitest store/integration tests, so a `makeMrPr()` shape used here
also drives `src/test/e2e/mr-pr-diff-flow.test.ts`.

## Writing a new test

1. Pick a view and decide on the distinct visual states (e.g. for
   Issues: empty / populated / detail).
2. Compose fixtures from `src/test/fixtures/`. Override fields that
   matter for the scenario; everything else defaults to sensible
   non-empty values.
3. Use `installBootstrapMocks(page, { mode, activeProject, extra })`
   to seed the Tauri IPC. `extra` is where per-view fixtures go,
   keyed by Tauri command name (snake_case).
4. After `page.goto("/")`, call `applyTheme(page, mode)` then
   `waitForAppReady(page)` (the latter waits for `isLoading` to drop).
5. Click into the view with `clickNav(page, "<label>")` — the label
   must match the visible sidebar text exactly.
6. Take a screenshot with `expect(page).toHaveScreenshot(...)`.

Minimum spec template:

```ts
import { expect, test } from "@playwright/test";
import {
  applyTheme, clickNav, installBootstrapMocks,
  THEME_MODES, waitForAppReady,
} from "../helpers";
import { makeProjectInfo, makeIssueList } from "../../../src/test/fixtures";

const PROJECT = makeProjectInfo();

for (const mode of THEME_MODES) {
  test.describe(`my-feature — ${mode}`, () => {
    test("populated", async ({ page }) => {
      await installBootstrapMocks(page, {
        mode,
        activeProject: PROJECT,
        forge: "github",                      // omit / "none" to hide provider items
        extra: { list_issues: makeIssueList() },
      });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);
      await clickNav(page, "Issues");
      await expect(page).toHaveScreenshot(`${mode}-populated.png`, {
        animations: "disabled",
      });
    });
  });
}
```

## Adding fixtures

If your test needs a shape that's not yet exported from
`src/test/fixtures/`, add a `make<Type>(overrides = {})` factory in
the matching file and re-export from `index.ts`. Keep the defaults
realistic so other tests can re-use the factory.

Don't accept the factory's plain return value as the only shape —
construct a small `make<Type>List()` whenever a populated screenshot
benefits from variety (mixed states / labels / authors / counts).

## Updating baselines

Always inspect the diff before regenerating:

```sh
npx playwright test --reporter=html             # opens an HTML report with diff PNGs
npm run test:visual:update                      # then approve and overwrite
```

`tests/visual/**/*-snapshots/` is committed; `test-results/` is
git-ignored and is the per-run scratch directory where Playwright
writes `expected/actual/diff` PNGs on failure.

## Debugging visual regressions with Claude Code

When a test fails on a UI change you're working on:

1. Run the failing test in isolation:
   ```sh
   npx playwright test tests/visual/components/changes.spec.ts -g "mixed-populated"
   ```
2. Playwright drops three PNGs in
   `test-results/<test-id>/`: `<name>-expected.png`, `<name>-actual.png`,
   `<name>-diff.png`.
3. Ask Claude to read the three with the `Read` tool — it's multimodal
   and can describe what changed, distinguishing intentional changes
   from regressions.
4. If the change was intentional, regenerate baselines with
   `npm run test:visual:update`. If it's a regression, fix the
   underlying issue and re-run without updating snapshots.

This human + AI baseline review is the design intent of the suite —
the screenshot diff catches *any* pixel change, but it can't tell
"oops, the spinner colour is wrong" from "yep, we picked a new
brand colour"; that's where the model in the loop pays off.

## Known gotchas

- **Dev server retries.** Vite occasionally fails the first
  dynamic-import for `.svelte-kit/generated/client/nodes/0.js` when
  several workers warm up at once. The config retries once, which
  covers it without hiding real flakiness.
- **`<html>` attributes don't survive hydration.** Don't rely on
  data-attributes set in `addInitScript` — SvelteKit rewrites the
  `<html>` element. Use `window.__beardgitMockIPC` for harness
  introspection instead.
- **Empty screenshots ⇒ bootstrap blocked.** If a baseline is the
  "Opening repository..." spinner, an `activateProjectTab` IPC call is
  rejecting and the `try/finally` in `projects.ts` is timing out the
  wait. Add the missing command to `bootstrap.ts`'s default response
  set; don't sprinkle it into individual specs.
- **Canvas/SVG views can't be clicked node-by-node.** The Graph view
  is canvas-rendered; for selection states drive the underlying store
  directly with `page.evaluate(... window.__TAURI_INTERNALS__.invoke(...))`
  or trigger via store imports from the dev server
  (`import("/src/lib/stores/graph.ts")`).

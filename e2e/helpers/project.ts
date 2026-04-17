/**
 * Project / repo helpers shared by every E2E spec.
 *
 * BeardGit launches to an empty "welcome" screen with no repo loaded;
 * every golden-path or regression spec that touches repo state has to
 * open a fixture first. These helpers keep that boilerplate out of the
 * specs themselves.
 */

/** Path roots — match `e2e/fixtures/setup.sh`. Resolved inside the container. */
export const FIXTURE_ROOT = "/workspace/e2e/fixtures";

export type FixtureName = "simple-repo" | "conflict-repo" | "bisect-repo";

/**
 * Access Tauri's invoke() from inside the webview.
 *
 * The bundled app exposes it at `window.__TAURI__.core.invoke`. Dynamic
 * `import("@tauri-apps/api/core")` fails inside WebKit because the module
 * specifier isn't a URL — node_modules aren't served by the asset
 * protocol.
 */
const getInvokeSnippet = `
  const w = window;
  const core = w.__TAURI__ && w.__TAURI__.core;
  if (!core || typeof core.invoke !== "function") {
    throw new Error("window.__TAURI__.core.invoke is not available");
  }
  return core.invoke;
`;

/**
 * Resolve Tauri's invoke() inside the webview.
 *
 * Tauri v2 apps without `app.withGlobalTauri: true` don't expose a
 * high-level `window.__TAURI__`. The NPM package `@tauri-apps/api/core`
 * calls `window.__TAURI_INTERNALS__.invoke(cmd, args)` under the hood —
 * we do the same. `invoke` on that object is not enumerable, which is
 * why a `Object.keys()` probe doesn't surface it.
 */
async function browserInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const result = await browser.executeAsync<
    { ok: boolean; value?: unknown; error?: string },
    [string, Record<string, unknown>]
  >(
    (c, a, done) => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const internals = (window as any).__TAURI_INTERNALS__;
      if (!internals || typeof internals.invoke !== "function") {
        done({ ok: false, error: "window.__TAURI_INTERNALS__.invoke is not available" });
        return;
      }
      internals
        .invoke(c, a)
        .then((v: unknown) => done({ ok: true, value: v }))
        .catch((err: unknown) =>
          done({ ok: false, error: err instanceof Error ? err.message : String(err) }),
        );
    },
    cmd,
    args ?? {},
  );
  if (!result.ok) {
    throw new Error(result.error ?? "unknown invoke error");
  }
  return result.value as T;
}

/**
 * Open a fixture project through the frontend store.
 *
 * We call `window.__E2E__.openProject(path)` which proxies to
 * `openProjectTab` — that's the same flow the "Add project" UI button
 * runs. It creates the project tab, activates it, loads the graph, etc.
 * Going straight to the Rust `open_project` IPC skips all the store
 * bookkeeping and leaves the UI stuck on the welcome screen.
 *
 * The `__E2E__` surface is only mounted when the app is built with
 * VITE_BEARDGIT_E2E=true (see +layout.svelte) — production bundles
 * strip it.
 */
export async function openFixtureProject(name: FixtureName): Promise<void> {
  const absolutePath = `${FIXTURE_ROOT}/${name}`;
  const result = await browser.executeAsync<{ ok: boolean; error?: string }, [string]>(
    (path, done) => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const e2e = (window as any).__E2E__;
      if (!e2e || typeof e2e.openProject !== "function") {
        done({
          ok: false,
          error:
            "window.__E2E__.openProject is not available — rebuild the app with VITE_BEARDGIT_E2E=true",
        });
        return;
      }
      e2e
        .openProject(path)
        .then(() => done({ ok: true }))
        .catch((err: unknown) =>
          done({
            ok: false,
            error: err instanceof Error ? err.message : String(err),
          }),
        );
    },
    absolutePath,
  );
  if (!result.ok) {
    throw new Error(`Failed to open fixture ${name}: ${result.error ?? "unknown"}`);
  }
  await browser.pause(800);
}

/** Close all open projects / tabs so the next spec starts from a clean welcome screen. */
export async function closeAllProjects(): Promise<void> {
  await browser.executeAsync<void, []>((done) => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const e2e = (window as any).__E2E__;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const internals = (window as any).__TAURI_INTERNALS__;
    if (!e2e || !internals || typeof internals.invoke !== "function") {
      done();
      return;
    }
    internals
      .invoke("get_open_projects")
      .then(async (projects: Array<unknown>) => {
        for (let i = projects.length - 1; i >= 0; i--) {
          await e2e.closeTab(i);
        }
        done();
      })
      .catch(() => done());
  });
  await browser.pause(300);
}

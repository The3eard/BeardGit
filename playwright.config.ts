import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/visual",
  fullyParallel: true,
  // No retries. This started at 1, for a Vite dev-server failure on the
  // first dynamic-import of `.svelte-kit/generated/client/nodes/0.js`
  // under worker ramp-up — but "papers over that without hiding real
  // flakiness" was not true: it also papered over a hover state landing
  // on the wrong sidebar row and the graph canvas repainting mid-shot,
  // both of which are now waited for properly in `helpers/`. A retry on a
  // screenshot suite hides precisely the instability the suite exists to
  // surface. If the Vite flake returns it fails as a module error, which
  // reads nothing like a diff — run with `--retries=1` while diagnosing.
  retries: 0,
  reporter: [["list"]],
  use: {
    baseURL: "http://localhost:1420",
    trace: "off",
    viewport: { width: 1440, height: 900 },
  },
  webServer: {
    command: "npm run dev -- --port 1420 --strictPort",
    url: "http://localhost:1420",
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
  expect: {
    toHaveScreenshot: {
      // An absolute cap, not `maxDiffPixelRatio`. The ratio this replaces
      // was 0.01 — 12,960px of a 1440×900 shot — which is larger than most
      // things in this app. A status badge is 18×18 = 324px, so changing
      // every badge in a populated list moved 2,680px and passed clean;
      // that is how a real regression shipped inside this very branch.
      //
      // 300 is measured, not picked: repeated `--retries=0` runs put
      // run-to-run jitter at 1–7px for DOM views, and it buys back the
      // single-badge case. The two surfaces that jitter more than this —
      // CodeMirror's diff text and the graph's canvas — carry their own
      // documented override at the callsite rather than loosening it for
      // everything.
      maxDiffPixels: 300,
      // Playwright's default per-pixel `threshold` is 0.2 in YIQ space,
      // which is an order of magnitude more than a theme-token change
      // produces — a grey shifting by 18/255 scores ~164 against a 1408
      // cutoff, so those pixels are never counted and the pixel budget is
      // never reached. That made these baselines blind to exactly the
      // regressions they look like they guard: an entire panel losing its
      // elevated background passed clean.
      //
      // Tight because a token change is a colour change, which is what
      // this knob governs. Raise it, don't remove it, if a real
      // antialiasing difference appears on another platform.
      threshold: 0.02,
    },
  },
});

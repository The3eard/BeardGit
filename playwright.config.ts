import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/visual",
  fullyParallel: true,
  // Vite's dev-server occasionally fails the first dynamic-import for
  // `.svelte-kit/generated/client/nodes/0.js` when several Playwright
  // workers ramp up simultaneously; one retry papers over that without
  // hiding real flakiness.
  retries: 1,
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
      maxDiffPixelRatio: 0.01,
      // Playwright's default per-pixel `threshold` is 0.2 in YIQ space,
      // which is an order of magnitude more than a theme-token change
      // produces — a grey shifting by 18/255 scores ~164 against a 1408
      // cutoff, so those pixels are never counted and `maxDiffPixelRatio`
      // is never reached. That made these baselines blind to exactly the
      // regressions they look like they guard: an entire panel losing its
      // elevated background passed clean.
      //
      // The renders are deterministic (identical across repeated runs), so
      // this can be tight. Raise it, don't remove it, if a real
      // antialiasing difference appears on another platform.
      threshold: 0.02,
    },
  },
});

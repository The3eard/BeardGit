import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/visual",
  fullyParallel: true,
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
    },
  },
});

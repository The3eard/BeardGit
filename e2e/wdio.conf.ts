import "@wdio/types";
import path from "path";
import { execSync } from "child_process";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const isCI = !!process.env.CI;
const projectRoot = path.resolve(__dirname, "..");

/**
 * Resolve the Tauri binary path based on platform and build type.
 * CI uses release builds from the build artifact; local uses debug.
 */
function resolveBinaryPath(): string {
  const buildType = process.env.BEARDGIT_BUILD_TYPE ?? "debug";
  const base = path.join(projectRoot, "src-tauri", "target", buildType);

  switch (process.platform) {
    case "darwin":
      return path.join(base, "beardgit");
    case "linux":
      return path.join(base, "beardgit");
    case "win32":
      return path.join(base, "beardgit.exe");
    default:
      throw new Error(`Unsupported platform: ${process.platform}`);
  }
}

export const config: WebdriverIO.Config = {
  runner: "local",
  specs: [path.join(__dirname, "specs", "**", "*.spec.ts")],
  exclude: [],

  maxInstances: 1, // Tauri app is single-instance

  capabilities: [
    {
      // @ts-expect-error -- tauri-driver specific capability
      "tauri:options": {
        application: resolveBinaryPath(),
      },
    },
  ],

  logLevel: isCI ? "warn" : "info",
  bail: isCI ? 1 : 0, // Fail fast in CI, run all locally
  waitforTimeout: isCI ? 15000 : 10000,
  connectionRetryTimeout: isCI ? 180000 : 120000,
  connectionRetryCount: 3,

  framework: "mocha",

  reporters: [
    "spec",
    ...(isCI
      ? [
          [
            "junit",
            {
              outputDir: path.join(projectRoot, "e2e", "results"),
              outputFileFormat: () => "junit.xml",
            },
          ] as [string, Record<string, unknown>],
        ]
      : []),
  ],

  mochaOpts: {
    ui: "bdd",
    timeout: isCI ? 120000 : 60000,
  },

  /**
   * Run fixture setup before all test suites.
   * Creates fresh repos from scratch every run for determinism.
   */
  onPrepare: async function () {
    const fixtureScript = path.join(projectRoot, "e2e", "fixtures", "setup.sh");
    console.log("[wdio] Setting up fixture repos...");
    execSync(`bash "${fixtureScript}"`, {
      cwd: projectRoot,
      stdio: "inherit",
    });
  },

  /**
   * Clean up fixture repos after all suites complete.
   * Only in CI to save disk; locally keep for inspection.
   */
  onComplete: async function () {
    if (isCI) {
      const reposDir = path.join(projectRoot, "e2e", "fixtures");
      console.log("[wdio] Cleaning up fixture repos...");
      execSync(
        `rm -rf "${reposDir}/simple-repo" "${reposDir}/conflict-repo" "${reposDir}/bisect-repo"`,
        {
          stdio: "inherit",
        },
      );
    }
  },

  /**
   * Capture screenshot on test failure.
   */
  afterTest: async function (test, _context, result) {
    if (!result.passed) {
      const screenshotDir = path.join(projectRoot, "e2e", "results", "screenshots");
      execSync(`mkdir -p "${screenshotDir}"`);
      const sanitizedName = test.title.replace(/[^a-zA-Z0-9]/g, "_").substring(0, 80);
      const timestamp = Date.now();
      const filePath = path.join(screenshotDir, `${sanitizedName}_${timestamp}.png`);
      try {
        await browser.saveScreenshot(filePath);
        console.log(`[wdio] Screenshot saved: ${filePath}`);
      } catch (err) {
        console.warn(`[wdio] Failed to save screenshot: ${err}`);
      }
    }
  },
};

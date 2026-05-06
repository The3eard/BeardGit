#!/usr/bin/env node
/**
 * Render the social-preview composition (scripts/build-og.html) to PNG via Playwright.
 *
 * Outputs:
 *   docs/assets/og-github.png   1280×640  (GitHub repo "social preview" slot)
 *   docs/assets/og-image.png    1200×630  (landing-page <meta og:image>)
 */

import { chromium } from "@playwright/test";
import { fileURLToPath } from "node:url";
import path from "node:path";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "..");
const htmlPath = path.join(here, "build-og.html");
const fileUrl = "file://" + htmlPath;

const targets = [
  { name: "og-github.png", width: 1280, height: 640, query: "" },
  { name: "og-image.png",  width: 1200, height: 630, query: "?size=og" },
];

const browser = await chromium.launch();
try {
  for (const t of targets) {
    const ctx = await browser.newContext({
      viewport: { width: t.width, height: t.height },
      deviceScaleFactor: 2,
    });
    const page = await ctx.newPage();
    await page.goto(fileUrl + t.query, { waitUntil: "networkidle" });
    // Belt-and-braces: make sure web fonts are ready before snapping.
    await page.evaluate(() => document.fonts.ready);
    const out = path.join(repoRoot, "docs", "assets", t.name);
    const stage = await page.locator("#stage");
    await stage.screenshot({ path: out, omitBackground: false });
    console.log(`✓ ${t.name}  ${t.width}×${t.height}  →  docs/assets/${t.name}`);
    await ctx.close();
  }
} finally {
  await browser.close();
}

#!/usr/bin/env node
/**
 * Render the BeardGit logo SVG to PNG favicons at the sizes the landing
 * needs. Source: docs/assets/logo.svg (cropped viewBox so the artwork
 * fills the frame). Outputs replace the previous, small renders.
 */
import { chromium } from "@playwright/test";
import { fileURLToPath } from "node:url";
import path from "node:path";
import fs from "node:fs/promises";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "..");
const svgPath = path.join(repoRoot, "docs", "assets", "logo.svg");
const svg = await fs.readFile(svgPath, "utf8");

const targets = [
  { name: "favicon-32.png",      size: 32  },
  { name: "favicon-192.png",     size: 192 },
  { name: "apple-touch-icon.png", size: 180 },
];

const browser = await chromium.launch();
try {
  for (const t of targets) {
    const ctx = await browser.newContext({
      viewport: { width: t.size, height: t.size },
      deviceScaleFactor: 1,
    });
    const page = await ctx.newPage();
    // Inline SVG, no padding, transparent bg — let the icon fill the frame.
    await page.setContent(`<!doctype html>
<html><head><style>
  html, body { margin: 0; padding: 0; background: transparent; }
  body { width: ${t.size}px; height: ${t.size}px; }
  svg { display: block; width: 100%; height: 100%; }
</style></head><body>${svg}</body></html>`);
    const out = path.join(repoRoot, "docs", "assets", t.name);
    await page.locator("svg").screenshot({ path: out, omitBackground: true });
    console.log(`✓ ${t.name}  ${t.size}×${t.size}`);
    await ctx.close();
  }
} finally {
  await browser.close();
}

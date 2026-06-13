#!/usr/bin/env node
/**
 * Convert the marketing PNGs produced by `tests/visual/marketing.spec.ts`
 * into the `.webp` + `.avif` variants the landing page's `<picture>`
 * elements reference, sitting next to each `*.png`.
 *
 * Usage:
 *   npm run build:screenshots        # render PNGs (playwright) + convert
 *   node scripts/build-screenshots.mjs   # convert existing PNGs only
 *
 * Requires `cwebp` and `avifenc` on PATH (Homebrew: `webp`, `libavif`).
 */

import { execFile } from "node:child_process";
import { readdir } from "node:fs/promises";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";
import path from "node:path";

const run = promisify(execFile);
const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const DIR = path.join(ROOT, "docs/assets/screenshots/_new");

const WEBP_QUALITY = "82";
const AVIF_QUALITY = "62";
const AVIF_SPEED = "6";

async function convert(png) {
  const base = png.replace(/\.png$/, "");
  await Promise.all([
    run("cwebp", ["-quiet", "-q", WEBP_QUALITY, png, "-o", `${base}.webp`]),
    run("avifenc", ["-q", AVIF_QUALITY, "-s", AVIF_SPEED, png, `${base}.avif`]),
  ]);
  return path.basename(base);
}

async function main() {
  let entries;
  try {
    entries = await readdir(DIR);
  } catch {
    console.error(`No screenshots dir at ${DIR}. Run the marketing spec first:`);
    console.error("  npx playwright test marketing.spec.ts");
    process.exit(1);
  }

  const pngs = entries.filter((f) => f.endsWith(".png")).map((f) => path.join(DIR, f));
  if (pngs.length === 0) {
    console.error(`No PNGs in ${DIR}.`);
    process.exit(1);
  }

  console.log(`Converting ${pngs.length} PNG${pngs.length === 1 ? "" : "s"} → webp + avif …`);
  // Bound concurrency so we don't spawn 72 encoders at once.
  const POOL = 4;
  for (let i = 0; i < pngs.length; i += POOL) {
    const done = await Promise.all(pngs.slice(i, i + POOL).map(convert));
    for (const name of done) console.log(`  ✓ ${name}`);
  }
  console.log("Done.");
}

await main();

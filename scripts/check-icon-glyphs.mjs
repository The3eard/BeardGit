#!/usr/bin/env node
/**
 * Nerd Font glyph guard.
 *
 * The icons in this UI are private-use codepoints (U+E000–U+F8FF) from
 * Symbols Nerd Font, written literally into `.svelte` files. They are
 * invisible in most terminals, most diffs and most review tools, and an
 * edit that drops them leaves markup that is still valid Svelte, still
 * typechecks, still lints, and still passes every test — the icon column
 * simply renders blank.
 *
 * That is not hypothetical: a scripted edit to `WorkdirTree.svelte`
 * replaced all three of its folder glyphs with empty strings, and the
 * whole gate plus a re-rendered marketing screenshot went green over it.
 * The regression was caught by a human reading the picture.
 *
 * So this pins the count per file. It is deliberately dumb — it does not
 * care *which* glyphs a file uses, only that a file which had icons still
 * has at least as many as it did. Adding icons is free; losing them is a
 * failure with the file name attached.
 *
 * Update the baseline with: npm run check:glyphs -- --write
 */
import { lstatSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(fileURLToPath(import.meta.url), "..", "..");
const SCAN_DIRS = ["src"];
const BASELINE = "scripts/icon-glyphs.baseline.json";
const SKIP_DIRS = new Set(["node_modules", "paraglide", ".svelte-kit"]);
const EXTENSIONS = [".svelte", ".ts"];

/** Private Use Area — where Nerd Font puts its icons. */
function isPrivateUse(codePoint) {
  return codePoint >= 0xe000 && codePoint <= 0xf8ff;
}

function collectFiles(dir, out = []) {
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    return out;
  }
  for (const name of entries) {
    if (SKIP_DIRS.has(name)) continue;
    const full = join(dir, name);
    let stat;
    try {
      // `lstat`, not `stat`: a dangling symlink is a thing that exists in a
      // working tree, and following it here would abort the whole check
      // with an unhandled ENOENT instead of reporting anything.
      stat = lstatSync(full);
    } catch {
      continue;
    }
    if (stat.isSymbolicLink()) continue;
    if (stat.isDirectory()) collectFiles(full, out);
    else if (EXTENSIONS.some((e) => name.endsWith(e))) out.push(full);
  }
  return out;
}

/** `{ "relative/path": count }` for every file that contains any icon. */
function scan() {
  const counts = {};
  for (const rootRel of SCAN_DIRS) {
    for (const file of collectFiles(join(ROOT, rootRel))) {
      const source = readFileSync(file, "utf8");
      let n = 0;
      for (const ch of source) {
        if (isPrivateUse(ch.codePointAt(0))) n++;
      }
      if (n > 0) counts[relative(ROOT, file)] = n;
    }
  }
  return counts;
}

function main() {
  const found = scan();
  const write = process.argv.includes("--write");
  const path = join(ROOT, BASELINE);

  if (write) {
    // Report the losses before accepting them. `--write` rewrites the whole
    // baseline, so re-baselining one deliberate removal would otherwise
    // silently absorb any other glyph loss sitting in the same working
    // tree — which is precisely the accident this script exists to catch.
    let previous = {};
    try {
      previous = JSON.parse(readFileSync(path, "utf8"));
    } catch {
      /* first run */
    }
    const dropped = Object.entries(previous)
      .map(([file, expected]) => ({ file, expected, actual: found[file] ?? 0 }))
      .filter(({ expected, actual }) => actual < expected);
    if (dropped.length > 0) {
      console.log(`⚠ ${dropped.length} file(s) lose glyphs in this re-baseline:`);
      for (const { file, expected, actual } of dropped) {
        console.log(`    ${file}  ${expected} → ${actual}`);
      }
      console.log("  Check every line above is a removal you meant.\n");
    }

    writeFileSync(path, `${JSON.stringify(found, null, 2)}\n`);
    const total = Object.values(found).reduce((a, b) => a + b, 0);
    console.log(
      `✎ Wrote ${BASELINE}: ${Object.keys(found).length} files, ${total} glyphs.`,
    );
    return;
  }

  let baseline;
  try {
    baseline = JSON.parse(readFileSync(path, "utf8"));
  } catch {
    console.error(`✖ ${BASELINE} is missing.`);
    console.error("  → create it with: npm run check:glyphs -- --write");
    process.exit(1);
  }

  const lost = [];
  for (const [file, expected] of Object.entries(baseline)) {
    const actual = found[file] ?? 0;
    if (actual < expected) lost.push({ file, expected, actual });
  }

  const totalFiles = Object.keys(found).length;
  const totalGlyphs = Object.values(found).reduce((a, b) => a + b, 0);
  console.log(`Icon glyph check: ${totalGlyphs} glyphs across ${totalFiles} files.`);

  if (lost.length > 0) {
    console.error(`\n✖ ${lost.length} file(s) lost icon glyphs:`);
    for (const { file, expected, actual } of lost) {
      console.error(`    ${file}  ${expected} → ${actual}`);
    }
    console.error(
      "\n  Nerd Font icons are private-use characters and vanish silently from" +
        "\n  a bad edit. If the removal was deliberate, re-baseline with:" +
        "\n    npm run check:glyphs -- --write",
    );
    process.exit(1);
  }

  console.log("\n✓ No file lost an icon glyph.");
}

main();

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
 * So this pins the glyphs per file, as a multiset of codepoints: every
 * codepoint a file had, it must still have at least as many of. Adding
 * icons is free; losing one is a failure with the file name and the
 * codepoint attached.
 *
 * It used to pin a bare count per file, which left the obvious hole: swap
 * a folder glyph for a chevron and the total is unchanged, so the check
 * passed while the icon column was wrong. A count cannot tell "the icons
 * are intact" from "the icons were shuffled". Codepoints can, and the
 * error message can then name what went missing instead of just how many.
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

/** `U+E5FF`-style label, so the baseline and the errors are greppable. */
function label(codePoint) {
  return `U+${codePoint.toString(16).toUpperCase().padStart(4, "0")}`;
}

/**
 * `{ "relative/path": { "U+E5FF": 3 } }` for every file that contains any
 * icon — a multiset of codepoints per file, not a total.
 */
function scan() {
  const files = {};
  for (const rootRel of SCAN_DIRS) {
    for (const file of collectFiles(join(ROOT, rootRel))) {
      const source = readFileSync(file, "utf8");
      const glyphs = {};
      for (const ch of source) {
        const cp = ch.codePointAt(0);
        if (!isPrivateUse(cp)) continue;
        const key = label(cp);
        glyphs[key] = (glyphs[key] ?? 0) + 1;
      }
      // Sorted so a re-baseline produces a stable, reviewable diff instead
      // of reordering on every run.
      const keys = Object.keys(glyphs).sort();
      if (keys.length > 0) {
        files[relative(ROOT, file)] = Object.fromEntries(
          keys.map((k) => [k, glyphs[k]]),
        );
      }
    }
  }
  return files;
}

/**
 * A baseline entry from before this check pinned codepoints: a bare total.
 *
 * Kept so a stale baseline still enforces *something* rather than crashing
 * or silently passing. `main` fails on it anyway, with instructions.
 */
function isLegacyEntry(entry) {
  return typeof entry === "number";
}

/** Count-only comparison, for a legacy baseline entry. */
function legacyLosses(expectedTotal, actualGlyphs) {
  const have = Object.values(actualGlyphs ?? {}).reduce((a, b) => a + b, 0);
  return have < expectedTotal
    ? [{ cp: "(total, legacy baseline)", want: expectedTotal, have }]
    : [];
}

/** Total glyphs across a scan result, for the summary line. */
function totalGlyphs(files) {
  return Object.values(files)
    .flatMap((glyphs) => Object.values(glyphs))
    .reduce((a, b) => a + b, 0);
}

/**
 * Per-codepoint losses for one file: every codepoint whose count dropped.
 * A swap shows up as one entry at `n → 0` (the glyph that left) while the
 * replacement is simply a new key, which is allowed.
 */
function lossesFor(expected, actual) {
  return Object.entries(expected)
    .map(([cp, want]) => ({ cp, want, have: actual?.[cp] ?? 0 }))
    .filter(({ want, have }) => have < want);
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
      .map(([file, expected]) => ({
        file,
        losses: isLegacyEntry(expected)
          ? legacyLosses(expected, found[file])
          : lossesFor(expected, found[file]),
      }))
      .filter(({ losses }) => losses.length > 0);
    if (dropped.length > 0) {
      console.log(`⚠ ${dropped.length} file(s) lose glyphs in this re-baseline:`);
      for (const { file, losses } of dropped) {
        console.log(`    ${file}`);
        for (const { cp, want, have } of losses) {
          console.log(`      ${cp}  ${want} → ${have}`);
        }
      }
      console.log("  Check every line above is a removal you meant.\n");
    }

    writeFileSync(path, `${JSON.stringify(found, null, 2)}\n`);
    console.log(
      `✎ Wrote ${BASELINE}: ${Object.keys(found).length} files, ` +
        `${totalGlyphs(found)} glyphs.`,
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

  const legacy = Object.values(baseline).some(isLegacyEntry);
  const lost = [];
  for (const [file, expected] of Object.entries(baseline)) {
    const losses = isLegacyEntry(expected)
      ? legacyLosses(expected, found[file])
      : lossesFor(expected, found[file]);
    if (losses.length > 0) lost.push({ file, losses });
  }

  console.log(
    `Icon glyph check: ${totalGlyphs(found)} glyphs across ` +
      `${Object.keys(found).length} files.`,
  );

  if (lost.length > 0) {
    console.error(`\n✖ ${lost.length} file(s) lost icon glyphs:`);
    for (const { file, losses } of lost) {
      console.error(`    ${file}`);
      for (const { cp, want, have } of losses) {
        console.error(`      ${cp}  ${want} → ${have}`);
      }
    }
    console.error(
      "\n  Nerd Font icons are private-use characters and vanish silently from" +
        "\n  a bad edit. A codepoint at `n → 0` with a new one alongside it is a" +
        "\n  swap, not a loss — but it is still a different icon than shipped." +
        "\n  If the change was deliberate, re-baseline with:" +
        "\n    npm run check:glyphs -- --write",
    );
    process.exit(1);
  }

  if (legacy) {
    // A count-only baseline cannot detect a swap. Still enforce what it can
    // rather than skipping the file, but do not let the weaker form persist
    // silently — it looks identical to the strong one from the outside.
    console.error(
      "\n✖ Baseline is in the old count-only format, which cannot catch a" +
        "\n  glyph swap. Regenerate it with:" +
        "\n    npm run check:glyphs -- --write",
    );
    process.exit(1);
  }

  console.log("\n✓ No file lost an icon glyph.");
}

main();

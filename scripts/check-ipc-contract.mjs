#!/usr/bin/env node
/**
 * IPC contract drift check (spec 05, Phase 2).
 *
 * The "three-file contract" (Rust `#[tauri::command]` ↔ `tauri.ts` wrapper ↔
 * TS type) is manual discipline. This is the cheap 80%-of-codegen guardrail:
 * it verifies *names and existence* — every Rust command has a `tauri.ts`
 * wrapper, and every wrapper targets a real command — without checking shapes.
 *
 * It parses source text (no build, just node):
 *   - Rust: every `#[tauri::command]` attribute (ignoring the string inside
 *     comments/docs) followed by its `fn <name>`, across `crates/` + `src-tauri/`.
 *   - Frontend: every `invoke("<name>")` string in `src/lib/api/tauri.ts`,
 *     tolerant of `invoke<Generic<Nested>>("name")` and stripping comments so
 *     commented-out code doesn't count.
 *
 * Diffs both directions and exits non-zero with a readable report on any
 * mismatch. Dynamically-named invokes (`invoke(cmd)`, template literals) can't
 * be matched to a command name — they're reported as skipped, not failures.
 */
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(fileURLToPath(import.meta.url), "..", "..");
const RUST_ROOTS = ["crates", "src-tauri"];
const TAURI_TS = "src/lib/api/tauri.ts";
const SKIP_DIRS = new Set(["target", "node_modules", ".git"]);

/** Recursively collect `.rs` files under `dir`, skipping build/vendor dirs. */
function collectRustFiles(dir, out = []) {
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    return out;
  }
  for (const name of entries) {
    if (SKIP_DIRS.has(name)) continue;
    const full = join(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) collectRustFiles(full, out);
    else if (name.endsWith(".rs")) out.push(full);
  }
  return out;
}

/**
 * Extract command names from Rust source. A line only counts as a command
 * attribute when, trimmed, it *starts with* `#[tauri::command]` — this skips
 * doc/line comments that merely mention the attribute (e.g.
 * "/// `#[tauri::command]` wraps this"), which would otherwise mis-capture the
 * next `fn` (as happened for `run` / `run_clone_pipeline`).
 */
function extractRustCommands() {
  const cmds = new Map(); // name -> "relpath:line"
  const fnRe =
    /^\s*(?:pub(?:\s*\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)/;
  for (const rootRel of RUST_ROOTS) {
    for (const file of collectRustFiles(join(ROOT, rootRel))) {
      const lines = readFileSync(file, "utf8").split("\n");
      for (let i = 0; i < lines.length; i++) {
        if (!lines[i].trim().startsWith("#[tauri::command]")) continue;
        // Walk forward over any further attributes / doc lines to the fn.
        for (let j = i + 1; j < Math.min(i + 12, lines.length); j++) {
          const m = lines[j].match(fnRe);
          if (m) {
            if (!cmds.has(m[1])) {
              cmds.set(m[1], `${relative(ROOT, file)}:${j + 1}`);
            }
            break;
          }
        }
      }
    }
  }
  return cmds;
}

/**
 * Strip `//` line comments and `/* *\/` block comments from JS/TS source,
 * skipping over string and template literals so a `//` inside a string is
 * preserved. Keeps line count irrelevant — output is only fed to the invoke
 * scanner below.
 */
function stripComments(src) {
  let out = "";
  let i = 0;
  const n = src.length;
  while (i < n) {
    const c = src[i];
    const c2 = src[i + 1];
    // String / template literals: copy verbatim to the matching quote.
    if (c === '"' || c === "'" || c === "`") {
      const quote = c;
      out += c;
      i++;
      while (i < n) {
        out += src[i];
        if (src[i] === "\\") {
          out += src[i + 1] ?? "";
          i += 2;
          continue;
        }
        if (src[i] === quote) {
          i++;
          break;
        }
        i++;
      }
      continue;
    }
    if (c === "/" && c2 === "/") {
      while (i < n && src[i] !== "\n") i++;
      continue;
    }
    if (c === "/" && c2 === "*") {
      i += 2;
      while (i < n && !(src[i] === "*" && src[i + 1] === "/")) i++;
      i += 2;
      continue;
    }
    out += c;
    i++;
  }
  return out;
}

/**
 * Extract wrapped command names from `tauri.ts`. Handles `invoke("cmd")` and
 * `invoke<Type<Nested>>("cmd")` (generics never contain `(`), and multiple
 * invokes per wrapper. First args that aren't a plain string literal are
 * dynamic and reported separately.
 */
function extractWrappers(src) {
  const clean = stripComments(src);
  const wrapped = new Set();
  const dynamic = [];
  const re = /\binvoke\b\s*(?:<[^(]*>)?\s*\(/g;
  let m;
  while ((m = re.exec(clean))) {
    const after = clean.slice(m.index + m[0].length);
    const sm = after.match(/^\s*"([A-Za-z_][A-Za-z0-9_]*)"/);
    if (sm) wrapped.add(sm[1]);
    else dynamic.push(after.slice(0, 40).replace(/\s+/g, " ").trim());
  }
  return { wrapped, dynamic };
}

function main() {
  const rustCmds = extractRustCommands();
  const tauriSrc = readFileSync(join(ROOT, TAURI_TS), "utf8");
  const { wrapped, dynamic } = extractWrappers(tauriSrc);

  const missingWrappers = [...rustCmds.keys()]
    .filter((c) => !wrapped.has(c))
    .sort();
  const orphanWrappers = [...wrapped].filter((c) => !rustCmds.has(c)).sort();

  console.log(
    `IPC contract check: ${rustCmds.size} Rust commands, ${wrapped.size} tauri.ts wrappers.`,
  );
  if (dynamic.length > 0) {
    console.log(
      `\n⚠ ${dynamic.length} dynamic invoke(...) call(s) skipped (non-literal command name):`,
    );
    for (const d of dynamic) console.log(`    invoke(${d}…`);
  }

  let failed = false;
  if (missingWrappers.length > 0) {
    failed = true;
    console.error(
      `\n✖ ${missingWrappers.length} Rust command(s) have no tauri.ts wrapper:`,
    );
    for (const c of missingWrappers) {
      console.error(`    ${c}  (${rustCmds.get(c)})`);
    }
    console.error(
      "  → add a wrapper in src/lib/api/tauri.ts (see src/CLAUDE.md — the three-file IPC contract).",
    );
  }
  if (orphanWrappers.length > 0) {
    failed = true;
    console.error(
      `\n✖ ${orphanWrappers.length} tauri.ts wrapper(s) target a command that no longer exists:`,
    );
    for (const c of orphanWrappers) console.error(`    invoke("${c}")`);
    console.error(
      "  → the Rust #[tauri::command] was renamed or removed; update or delete the wrapper.",
    );
  }

  if (failed) {
    console.error("\nIPC contract drift detected.");
    process.exit(1);
  }
  console.log("\n✓ IPC contract is in sync.");
}

main();

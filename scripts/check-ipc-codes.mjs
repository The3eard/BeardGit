#!/usr/bin/env node
/**
 * IPC error-code coverage guard.
 *
 * Every `IpcError` carries a stable snake_case `code` so the frontend can
 * branch on error kind instead of matching free text. `errorCodeMessage`
 * in `src/lib/api/errors.ts` turns a handful of those into a written
 * sentence; the rest fall through to the raw message, which is the right
 * call for most of them.
 *
 * The gap this closes is not "some codes have no message" — that is
 * deliberate. It is that **nothing noticed a new code at all.** Add a
 * `IpcError::new("some_new_thing", …)` in Rust and no check, test or type
 * says whether the frontend should say something better than the raw
 * message. The decision simply never gets made.
 *
 * So this asks for a decision, not a translation: every code Rust can emit
 * must be either mapped in `errorCodeMessage` or listed under `@unmapped`
 * in that file's doc comment. Adding a code fails the gate until you have
 * said which.
 *
 * It also runs the other way, like the IPC contract check: a `case` or an
 * `@unmapped` entry naming a code Rust no longer emits is dead weight, and
 * dead entries are how a table stops describing the system it documents.
 * `@legacy-code` covers the deliberate exception — an alias kept for a
 * code that used to exist.
 */
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(fileURLToPath(import.meta.url), "..", "..");
const RUST_DIRS = ["crates", "src-tauri"];
const ERRORS_TS = "src/lib/api/errors.ts";

function rustFiles(dir, out = []) {
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    return out;
  }
  for (const name of entries) {
    if (name === "target" || name === "node_modules") continue;
    const full = join(dir, name);
    const stat = statSync(full);
    if (stat.isDirectory()) rustFiles(full, out);
    else if (name.endsWith(".rs")) out.push(full);
  }
  return out;
}

/**
 * Every code Rust can emit, mapped to where it is emitted.
 *
 * Two shapes, because the codes are written two ways: as the first
 * argument to `new`/`expected`, and as the right-hand side of a match arm
 * inside the `From` impls in `ipc_error.rs` (`G::Binary => "binary_file"`).
 * Missing the second shape would silently under-report by 14.
 */
function rustCodes() {
  const codes = new Map();
  const record = (code, file, line) => {
    if (!codes.has(code)) codes.set(code, []);
    codes.get(code).push(`${relative(ROOT, file)}:${line}`);
  };

  for (const rootRel of RUST_DIRS) {
    for (const file of rustFiles(join(ROOT, rootRel))) {
      const text = readFileSync(file, "utf8");
      const isEnvelope = file.endsWith("ipc_error.rs");
      text.split("\n").forEach((line, i) => {
        for (const m of line.matchAll(
          /(?:IpcError|Self)::(?:new|expected)\(\s*"([a-z0-9_]+)"/g,
        )) {
          record(m[1], file, i + 1);
        }
        // Match arms only inside the envelope module: elsewhere a
        // `=> "literal"` is any old string, not an error code.
        if (isEnvelope) {
          for (const m of line.matchAll(/=>\s*"([a-z0-9_]+)"/g)) {
            record(m[1], file, i + 1);
          }
        }
      });
    }
  }
  return codes;
}

/** Codes with a written message, and the codes each annotation lists. */
function frontendCodes() {
  const text = readFileSync(join(ROOT, ERRORS_TS), "utf8");
  const mapped = new Set(
    [...text.matchAll(/case\s+"([a-z0-9_]+)"\s*:/g)].map((m) => m[1]),
  );
  // One tag, one line. Continuing a list onto the next line requires
  // repeating the tag. That is a little repetitive to write and it is the
  // only rule that cannot mistake surrounding prose for a code list — an
  // earlier version scanned to the next tag and hoovered up 98 English
  // words as error codes.
  const tagged = (tag) => {
    const set = new Set();
    for (const m of text.matchAll(new RegExp(`@${tag}[ \\t]+([^\\n]*)`, "g"))) {
      for (const c of m[1].matchAll(/[a-z0-9_]+/g)) set.add(c[0]);
    }
    return set;
  };
  return { mapped, unmapped: tagged("unmapped"), legacy: tagged("legacy-code") };
}

function main() {
  const rust = rustCodes();
  const { mapped, unmapped, legacy } = frontendCodes();
  const problems = [];

  const classified = new Set([...mapped, ...unmapped]);
  for (const [code, sites] of [...rust].sort()) {
    if (classified.has(code)) continue;
    problems.push(
      `\`${code}\` is emitted by Rust but neither mapped in ` +
        `errorCodeMessage nor listed under @unmapped.\n` +
        `        emitted at ${sites.slice(0, 3).join(", ")}` +
        `${sites.length > 3 ? ` (+${sites.length - 3} more)` : ""}\n` +
        `        → give it a message, or add it to @unmapped to say the raw ` +
        `message is fine.`,
    );
  }

  // The other direction: entries describing codes that no longer exist.
  for (const code of [...mapped].sort()) {
    if (rust.has(code) || legacy.has(code)) continue;
    problems.push(
      `\`${code}\` has a message in errorCodeMessage but no Rust code emits ` +
        `it.\n        → drop the case, or list it under @legacy-code if it is ` +
        `a kept alias.`,
    );
  }
  for (const code of [...unmapped].sort()) {
    if (rust.has(code)) continue;
    problems.push(
      `\`${code}\` is listed under @unmapped but no Rust code emits it.\n` +
        `        → drop it from the list.`,
    );
  }

  console.log(
    `IPC code check: ${rust.size} Rust codes, ${mapped.size} with a message, ` +
      `${unmapped.size} unmapped on purpose` +
      `${legacy.size > 0 ? `, ${legacy.size} legacy alias(es)` : ""}.`,
  );

  if (problems.length > 0) {
    console.error(`\n✖ ${problems.length} unclassified or stale code(s):`);
    for (const p of problems) console.error(`    ${p}`);
    console.error(
      `\n  The point is a decision, not a translation: most codes are fine` +
        `\n  falling through to the raw message. Say so explicitly in ${ERRORS_TS}.`,
    );
    process.exit(1);
  }

  console.log("\n✓ Every IPC error code is accounted for.");
}

main();

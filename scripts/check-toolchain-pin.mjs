#!/usr/bin/env node
/**
 * Rust toolchain pin guard.
 *
 * The toolchain version lives in six places: `channel` in
 * `rust-toolchain.toml`, and one `dtolnay/rust-toolchain@<version>` ref in
 * each of the five workflows. They must all agree, and nothing but this
 * script makes them.
 *
 * The pin exists because they did not agree. CI installed a fresh `stable`
 * while the local gate ran whatever the machine had — three minor versions
 * behind — so a new clippy lint (`manual_filter`) failed CI while every
 * local run passed. `beta` was red for eight consecutive merges over it,
 * and no amount of care locally could have reproduced the failure.
 *
 * Pinning fixed that; this stops it coming back the next time someone
 * bumps one of the six and not the other five. A prose comment in the
 * workflow is not a mechanism.
 *
 * It also checks `components`, which is quietly load-bearing: the action
 * installs with `--profile minimal`, and a pinned version is not
 * preinstalled on the runner images the way `stable` is, so that list is
 * the only reason clippy and rustfmt exist in CI at all.
 */
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(fileURLToPath(import.meta.url), "..", "..");
const TOML = "rust-toolchain.toml";
const WORKFLOW_DIR = ".github/workflows";
const REQUIRED_COMPONENTS = ["clippy", "rustfmt"];

/**
 * Read `channel` and `components` out of `rust-toolchain.toml`.
 *
 * Hand-parsed rather than pulled in as a dependency: the file is five
 * lines and a TOML parser is not worth a devDependency the audit has to
 * carry. Deliberately strict — anything it cannot read is an error, not a
 * silent skip, because a silent skip here is exactly the failure mode the
 * script exists to prevent.
 */
function readToml() {
  let text;
  try {
    text = readFileSync(join(ROOT, TOML), "utf8");
  } catch {
    return { error: `${TOML} is missing.` };
  }

  const channel = text.match(/^\s*channel\s*=\s*"([^"]+)"/m)?.[1];
  if (!channel) return { error: `no \`channel = "…"\` in ${TOML}.` };

  const componentsRaw = text.match(/^\s*components\s*=\s*\[([^\]]*)\]/m)?.[1];
  const components =
    componentsRaw === undefined
      ? []
      : [...componentsRaw.matchAll(/"([^"]+)"/g)].map((m) => m[1]);

  return { channel, components };
}

/** Every `dtolnay/rust-toolchain@<ref>` in the workflows, with its file. */
function readWorkflowRefs() {
  const dir = join(ROOT, WORKFLOW_DIR);
  const refs = [];
  for (const name of readdirSync(dir).sort()) {
    if (!name.endsWith(".yml") && !name.endsWith(".yaml")) continue;
    const text = readFileSync(join(dir, name), "utf8");
    const lines = text.split("\n");
    lines.forEach((line, i) => {
      const ref = line.match(/dtolnay\/rust-toolchain@(\S+)/)?.[1];
      if (ref) refs.push({ file: `${WORKFLOW_DIR}/${name}`, line: i + 1, ref });
    });
  }
  return refs;
}

function main() {
  const toml = readToml();
  if (toml.error) {
    console.error(`✖ ${toml.error}`);
    process.exit(1);
  }

  const refs = readWorkflowRefs();
  const problems = [];

  if (refs.length === 0) {
    problems.push(
      `no \`dtolnay/rust-toolchain@…\` refs found under ${WORKFLOW_DIR}/ — ` +
        `either the action was swapped out (update this script) or the ` +
        `workflows lost their toolchain step.`,
    );
  }

  const mismatched = refs.filter((r) => r.ref !== toml.channel);
  for (const { file, line, ref } of mismatched) {
    problems.push(
      `${file}:${line} pins @${ref}, but ${TOML} says ${toml.channel}.`,
    );
  }

  // A floating ref is the specific thing the pin replaced, so name it
  // rather than letting it read as just another mismatch.
  const floating = refs.filter((r) => /^(stable|nightly|beta|master)$/.test(r.ref));
  for (const { file, line, ref } of floating) {
    problems.push(
      `${file}:${line} uses the floating @${ref} ref. That is what put beta ` +
        `red for eight merges: CI resolves it to a newer compiler than the ` +
        `local gate runs, so a new lint fails only here.`,
    );
  }

  const missing = REQUIRED_COMPONENTS.filter(
    (c) => !toml.components.includes(c),
  );
  if (missing.length > 0) {
    problems.push(
      `${TOML} does not list ${missing.map((c) => `\`${c}\``).join(" and ")} ` +
        `under \`components\`. The action installs \`--profile minimal\` and a ` +
        `pinned version is not preinstalled on the runners, so CI would have ` +
        `no ${missing.join("/")}.`,
    );
  }

  console.log(
    `Toolchain pin check: ${TOML} channel ${toml.channel}, ` +
      `${refs.length} workflow ref(s).`,
  );

  if (problems.length > 0) {
    console.error(`\n✖ ${problems.length} problem(s):`);
    for (const p of problems) console.error(`    ${p}`);
    console.error(
      `\n  To bump the toolchain, change all of them together, then run the` +
        `\n  full gate and fix what the newer clippy reports.`,
    );
    process.exit(1);
  }

  console.log("\n✓ Toolchain pin is consistent across the workflows.");
}

main();

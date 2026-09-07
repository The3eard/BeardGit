#!/usr/bin/env node
/**
 * Fail the gate when the bundled `gh` / `glab` sidecars have fallen behind.
 *
 * `resolve_cli_binary` (app-core) prefers the bundled sidecar over whatever
 * the user has on PATH, so every user runs exactly the versions pinned in
 * `cli-versions.json`. That only works if the pins move. Before this check
 * they sat at gh 2.62.0 / glab 1.46.1 for close to two years, and the
 * bundled glab could no longer read the config file a current glab writes
 * (`parsing time ... cannot parse`) — every GitLab call failed for anyone
 * who also had a modern glab installed. Nothing noticed because PATH-first
 * resolution hid the stale sidecar behind the system binary.
 *
 * Policy: the pin may trail upstream, but not for long. The check fails
 * when the pinned version is not the latest release AND that latest release
 * is older than GRACE_DAYS — i.e. we have been behind for a whole grace
 * period. A fresh upstream release therefore never breaks the gate by
 * itself.
 *
 * Network errors are reported and skipped (exit 0): the gate must stay
 * usable offline, and CI runs the same script with a network.
 */

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const GRACE_DAYS = 90;

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const pinned = JSON.parse(readFileSync(join(root, "cli-versions.json"), "utf8"));

const SOURCES = {
  gh: async () => {
    const r = await fetchJson("https://api.github.com/repos/cli/cli/releases/latest");
    return { version: r.tag_name.replace(/^v/, ""), publishedAt: r.published_at };
  },
  glab: async () => {
    const r = await fetchJson(
      "https://gitlab.com/api/v4/projects/gitlab-org%2Fcli/releases?per_page=1",
    );
    return { version: r[0].tag_name.replace(/^v/, ""), publishedAt: r[0].released_at };
  },
};

async function fetchJson(url) {
  const res = await fetch(url, {
    headers: { "user-agent": "beardgit-check-cli-versions" },
    signal: AbortSignal.timeout(15_000),
  });
  if (!res.ok) throw new Error(`${url} → HTTP ${res.status}`);
  return res.json();
}

let failed = false;
for (const [tool, latestOf] of Object.entries(SOURCES)) {
  const have = pinned[tool];
  if (!have) {
    console.error(`✗ ${tool}: no pin in cli-versions.json`);
    failed = true;
    continue;
  }
  let latest;
  try {
    latest = await latestOf();
  } catch (err) {
    console.warn(`⚠ ${tool}: could not query latest release (${err.message}); skipping`);
    continue;
  }
  if (latest.version === have) {
    console.log(`✓ ${tool} ${have} is the latest release`);
    continue;
  }
  const ageDays = Math.floor((Date.now() - Date.parse(latest.publishedAt)) / 86_400_000);
  if (ageDays > GRACE_DAYS) {
    console.error(
      `✗ ${tool}: pinned ${have}, latest ${latest.version} was released ${ageDays} days ago ` +
        `(> ${GRACE_DAYS}). Bump cli-versions.json and run scripts/download-cli-binaries.js.`,
    );
    failed = true;
  } else {
    console.log(
      `✓ ${tool} ${have} trails ${latest.version} (released ${ageDays} days ago, within ${GRACE_DAYS}-day grace)`,
    );
  }
}

process.exit(failed ? 1 : 0);

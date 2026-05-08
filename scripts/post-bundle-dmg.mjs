#!/usr/bin/env node
// Post-process Tauri-built macOS DMGs to embed extra files (e.g. a
// READ_BEFORE_RUN.md with xattr quarantine instructions). Tauri 2's DMG
// bundler doesn't support arbitrary extra files in the DMG layout, so we
// modify the artifact after the fact:
//
//   1. Convert the read-only DMG to read-write (UDRW)
//   2. Mount it
//   3. Copy the extras into the mount point
//   4. Detach the volume
//   5. Convert back to compressed read-only (UDZO) and replace the original
//
// The macOS auto-updater uses the .app.tar.gz artifact, not the DMG, so
// changing the DMG hash does not break updates. The script is a no-op on
// non-darwin platforms.

import { execFileSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  renameSync,
  rmSync,
  statSync,
} from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..");
const extrasDir = join(repoRoot, "src-tauri", "dmg");
const targetRoot = join(repoRoot, "src-tauri", "target");

if (process.platform !== "darwin") {
  console.log("[post-bundle-dmg] Skipping (not macOS)");
  process.exit(0);
}

if (!existsSync(extrasDir)) {
  console.log(`[post-bundle-dmg] No extras directory at ${extrasDir}, nothing to do`);
  process.exit(0);
}

const extras = readdirSync(extrasDir)
  .filter((name) => !name.startsWith("."))
  .map((name) => join(extrasDir, name))
  .filter((p) => statSync(p).isFile());

if (extras.length === 0) {
  console.log("[post-bundle-dmg] No extra files to embed");
  process.exit(0);
}

const dmgs = findDmgs(targetRoot);
if (dmgs.length === 0) {
  console.log(`[post-bundle-dmg] No DMG found under ${targetRoot}`);
  process.exit(0);
}

for (const dmg of dmgs) {
  embedExtras(dmg, extras);
}

console.log("[post-bundle-dmg] Done");

function findDmgs(root) {
  const out = [];
  if (!existsSync(root)) return out;
  const stack = [root];
  while (stack.length) {
    const dir = stack.pop();
    let entries;
    try {
      entries = readdirSync(dir, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const entry of entries) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) {
        stack.push(full);
      } else if (entry.isFile() && entry.name.endsWith(".dmg")) {
        out.push(full);
      }
    }
  }
  return out;
}

function embedExtras(originalDmg, files) {
  const rel = relative(repoRoot, originalDmg);
  console.log(`[post-bundle-dmg] Processing ${rel}`);

  const workDir = join(repoRoot, "src-tauri", "target", ".dmg-postbundle");
  mkdirSync(workDir, { recursive: true });

  const rwDmg = join(workDir, "rw.dmg");
  const newDmg = join(workDir, "new.dmg");
  const mountPoint = join(workDir, "mnt");

  cleanup(rwDmg, newDmg, mountPoint);
  mkdirSync(mountPoint, { recursive: true });

  try {
    run("hdiutil", ["convert", originalDmg, "-format", "UDRW", "-o", rwDmg]);
    run("hdiutil", [
      "attach",
      rwDmg,
      "-mountpoint",
      mountPoint,
      "-nobrowse",
      "-noverify",
      "-noautoopen",
    ]);

    try {
      for (const file of files) {
        const dest = join(mountPoint, basename(file));
        copyFileSync(file, dest);
        console.log(`[post-bundle-dmg]   + ${basename(file)}`);
      }
    } finally {
      run("hdiutil", ["detach", mountPoint, "-quiet"]);
    }

    run("hdiutil", [
      "convert",
      rwDmg,
      "-format",
      "UDZO",
      "-imagekey",
      "zlib-level=9",
      "-o",
      newDmg,
    ]);

    renameSync(newDmg, originalDmg);
    console.log(`[post-bundle-dmg]   replaced ${rel}`);
  } finally {
    cleanup(rwDmg, newDmg, mountPoint);
  }
}

function cleanup(...paths) {
  for (const p of paths) {
    try {
      rmSync(p, { recursive: true, force: true });
    } catch {
      // ignore
    }
  }
}

function basename(p) {
  return p.split("/").pop();
}

function run(cmd, args) {
  execFileSync(cmd, args, { stdio: ["ignore", "inherit", "inherit"] });
}

#!/usr/bin/env node

/**
 * Download CLI binaries (gh, glab) for Tauri sidecar bundling.
 *
 * Reads pinned versions from cli-versions.json, downloads platform-specific
 * binaries from GitHub/GitLab releases, extracts them, and places them in
 * src-tauri/binaries/ with Tauri sidecar naming convention.
 *
 * Usage:
 *   node scripts/download-cli-binaries.js              # Download for current platform
 *   node scripts/download-cli-binaries.js --dry-run    # Print URLs without downloading
 *   node scripts/download-cli-binaries.js --target aarch64-apple-darwin
 */

import {
  readFileSync,
  mkdirSync,
  chmodSync,
  renameSync,
  rmSync,
  createWriteStream,
  readdirSync,
} from 'fs';
import { join, resolve } from 'path';
import { execSync } from 'child_process';
import { pipeline } from 'stream/promises';
import { fileURLToPath } from 'url';

const __dirname = fileURLToPath(new URL('.', import.meta.url));
const ROOT = resolve(__dirname, '..');
const BINARIES_DIR = join(ROOT, 'src-tauri', 'binaries');
const VERSIONS_FILE = join(ROOT, 'cli-versions.json');

// ── Target triple mapping ──────────────────────────────────────────────

// Per-CLI extension + os/arch conventions vary. gh releases use a single
// pattern (zip for mac/windows, tar.gz for linux). glab v1.x uses macOS (not
// Darwin), tar.gz for macOS + linux, zip for windows.
const PLATFORM_MAP = {
  'aarch64-apple-darwin': {
    gh: { os: 'macOS', arch: 'arm64', ext: 'zip' },
    glab: { os: 'macOS', arch: 'arm64', ext: 'tar.gz' },
  },
  'x86_64-apple-darwin': {
    gh: { os: 'macOS', arch: 'amd64', ext: 'zip' },
    glab: { os: 'macOS', arch: 'x86_64', ext: 'tar.gz' },
  },
  'x86_64-unknown-linux-gnu': {
    gh: { os: 'linux', arch: 'amd64', ext: 'tar.gz' },
    glab: { os: 'Linux', arch: 'x86_64', ext: 'tar.gz' },
  },
  'x86_64-pc-windows-msvc': {
    gh: { os: 'windows', arch: 'amd64', ext: 'zip' },
    glab: { os: 'Windows', arch: 'x86_64', ext: 'zip' },
  },
};

// ── Detect target triple ───────────────────────────────────────────────

function detectTargetTriple() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === 'darwin' && arch === 'arm64') return 'aarch64-apple-darwin';
  if (platform === 'darwin' && arch === 'x64') return 'x86_64-apple-darwin';
  if (platform === 'linux' && arch === 'x64') return 'x86_64-unknown-linux-gnu';
  if (platform === 'win32' && arch === 'x64') return 'x86_64-pc-windows-msvc';

  throw new Error(`Unsupported platform: ${platform}/${arch}`);
}

// ── Download URL builders ──────────────────────────────────────────────

function ghDownloadUrl(version, platformInfo) {
  // https://github.com/cli/cli/releases/download/v{ver}/gh_{ver}_{os}_{arch}.{ext}
  const { os, arch, ext } = platformInfo.gh;
  return `https://github.com/cli/cli/releases/download/v${version}/gh_${version}_${os}_${arch}.${ext}`;
}

function glabDownloadUrl(version, platformInfo) {
  // https://gitlab.com/gitlab-org/cli/-/releases/v{ver}/downloads/glab_{ver}_{os}_{arch}.{ext}
  const { os, arch, ext } = platformInfo.glab;
  return `https://gitlab.com/gitlab-org/cli/-/releases/v${version}/downloads/glab_${version}_${os}_${arch}.${ext}`;
}

// ── Download + extract ─────────────────────────────────────────────────

async function downloadFile(url, destPath) {
  console.log(`  Downloading: ${url}`);
  const res = await fetch(url, { redirect: 'follow' });
  if (!res.ok) {
    throw new Error(`Failed to download ${url}: ${res.status} ${res.statusText}`);
  }
  const fileStream = createWriteStream(destPath);
  await pipeline(res.body, fileStream);
  console.log(`  Saved to: ${destPath}`);
}

function findFileRecursive(dir, name) {
  const entries = readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = join(dir, entry.name);
    if (entry.isDirectory()) {
      const found = findFileRecursive(fullPath, name);
      if (found) return found;
    } else if (entry.name === name) {
      return fullPath;
    }
  }
  return null;
}

function extractBinary(archivePath, binaryName, outputPath, ext, isWindowsTarget) {
  const tmpDir = join(BINARIES_DIR, '_extract_tmp');
  mkdirSync(tmpDir, { recursive: true });

  try {
    if (ext === 'zip') {
      if (process.platform === 'win32') {
        execSync(
          `powershell -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${tmpDir}' -Force"`,
          { stdio: 'pipe' },
        );
      } else {
        execSync(`unzip -o "${archivePath}" -d "${tmpDir}"`, { stdio: 'pipe' });
      }
    } else if (ext === 'tar.gz') {
      execSync(`tar -xzf "${archivePath}" -C "${tmpDir}"`, { stdio: 'pipe' });
    } else {
      throw new Error(`Unsupported archive extension: ${ext}`);
    }

    // Find the binary inside the extracted directory (may be nested)
    const exeName = isWindowsTarget ? `${binaryName}.exe` : binaryName;
    const found = findFileRecursive(tmpDir, exeName);
    if (!found) {
      throw new Error(`Binary '${exeName}' not found in extracted archive`);
    }

    renameSync(found, outputPath);

    // Make executable on Unix (only relevant when we're running on Unix)
    if (process.platform !== 'win32') {
      chmodSync(outputPath, 0o755);
    }

    console.log(`  Extracted: ${outputPath}`);
  } finally {
    rmSync(tmpDir, { recursive: true, force: true });
    rmSync(archivePath, { force: true });
  }
}

// ── Main ───────────────────────────────────────────────────────────────

async function main() {
  const args = process.argv.slice(2);
  const dryRun = args.includes('--dry-run');
  const targetIdx = args.indexOf('--target');
  const targetTriple = targetIdx !== -1 ? args[targetIdx + 1] : detectTargetTriple();

  const platformInfo = PLATFORM_MAP[targetTriple];
  if (!platformInfo) {
    console.error(`Unknown target triple: ${targetTriple}`);
    console.error(`Supported: ${Object.keys(PLATFORM_MAP).join(', ')}`);
    process.exit(1);
  }

  const versions = JSON.parse(readFileSync(VERSIONS_FILE, 'utf-8'));
  const isWindowsTarget = targetTriple.includes('windows');

  console.log(`Target: ${targetTriple}`);
  console.log(`gh version: ${versions.gh}`);
  console.log(`glab version: ${versions.glab}`);
  console.log('');

  const ghUrl = ghDownloadUrl(versions.gh, platformInfo);
  const glabUrl = glabDownloadUrl(versions.glab, platformInfo);

  const ghSidecarName = isWindowsTarget ? `gh-${targetTriple}.exe` : `gh-${targetTriple}`;
  const glabSidecarName = isWindowsTarget
    ? `glab-${targetTriple}.exe`
    : `glab-${targetTriple}`;

  if (dryRun) {
    console.log('DRY RUN — would download:');
    console.log(`  gh:   ${ghUrl}`);
    console.log(`  glab: ${glabUrl}`);
    console.log('');
    console.log('Output files:');
    console.log(`  ${join(BINARIES_DIR, ghSidecarName)}`);
    console.log(`  ${join(BINARIES_DIR, glabSidecarName)}`);
    return;
  }

  mkdirSync(BINARIES_DIR, { recursive: true });

  // Download and extract gh
  console.log('Downloading gh...');
  const ghArchive = join(BINARIES_DIR, `gh-archive.${platformInfo.gh.ext}`);
  await downloadFile(ghUrl, ghArchive);
  extractBinary(
    ghArchive,
    'gh',
    join(BINARIES_DIR, ghSidecarName),
    platformInfo.gh.ext,
    isWindowsTarget,
  );

  // Download and extract glab
  console.log('');
  console.log('Downloading glab...');
  const glabArchive = join(BINARIES_DIR, `glab-archive.${platformInfo.glab.ext}`);
  await downloadFile(glabUrl, glabArchive);
  extractBinary(
    glabArchive,
    'glab',
    join(BINARIES_DIR, glabSidecarName),
    platformInfo.glab.ext,
    isWindowsTarget,
  );

  console.log('');
  console.log('Done! Binaries ready for Tauri sidecar bundling.');
}

main().catch((err) => {
  console.error('Error:', err.message);
  process.exit(1);
});

/**
 * ESLint flat configuration for BeardGit.
 *
 * Purpose: enforce that inline Svelte style attributes and style: directives
 * do not contain hardcoded hex/rgb/hsl color literals. See the custom rule
 * at eslint-rules/no-hex-in-svelte.cjs and the plan at
 * docs/superpowers/plans/2026-04-23-theme-color-audit.md §C4.
 *
 * Uses svelte/flat/base (parser setup only, no opinionated svelte rules) and
 * typescript-eslint/base (registers @typescript-eslint plugin so existing
 * eslint-disable comments are recognised) to avoid surfacing pre-existing
 * quality issues out of scope for Phase C.
 *
 * Escape hatch: // beardgit:allow-hex: <reason>  (in <script> blocks)
 *              <!-- beardgit:allow-hex: <reason> -->  (in Svelte template markup)
 */

import svelteParser from "svelte-eslint-parser";
import tsParser from "@typescript-eslint/parser";
import svelte from "eslint-plugin-svelte";
import tseslint from "typescript-eslint";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const noHexInSvelte = require("./eslint-rules/no-hex-in-svelte.cjs");

/**
 * Guardrail for the "three-file IPC contract" (see src/CLAUDE.md): every
 * Rust `#[tauri::command]` call must go through a typed wrapper in
 * `src/lib/api/tauri.ts` (or `runMutation`), never a raw `invoke()`.
 * Forbid importing `@tauri-apps/api/core` — the module that exports
 * `invoke` — anywhere outside `src/lib/api/`. The rule is turned back off
 * for `src/lib/api/**` below, which is the one place `invoke` is allowed.
 */
const noRawInvoke = [
  "error",
  {
    paths: [
      {
        name: "@tauri-apps/api/core",
        message:
          "Do not import invoke() directly. Route Rust calls through the typed wrappers in src/lib/api/tauri.ts (or runMutation). See src/CLAUDE.md — the three-file IPC contract.",
      },
    ],
  },
];

export default [
  {
    ignores: [
      "node_modules/**",
      ".svelte-kit/**",
      "build/**",
      "dist/**",
      "src/lib/paraglide/**",
      "src-tauri/**",
      "tests/visual/**",
    ],
  },
  // Register @typescript-eslint plugin so eslint-disable comments in .ts
  // files are recognised (existing codebase uses them; this avoids "rule
  // definition not found" errors without turning on any type-check rules).
  tseslint.configs.base,
  // Set up svelte-eslint-parser (parser only, no opinionated rules).
  ...svelte.configs["flat/base"],
  {
    files: ["src/**/*.ts"],
    languageOptions: { parser: tsParser, ecmaVersion: "latest", sourceType: "module" },
    rules: {
      "no-restricted-imports": noRawInvoke,
    },
  },
  {
    files: ["src/**/*.svelte"],
    languageOptions: {
      parser: svelteParser,
      parserOptions: { parser: tsParser, extraFileExtensions: [".svelte"] },
    },
    plugins: {
      "beardgit": { rules: { "no-hex-in-svelte": noHexInSvelte } },
    },
    rules: {
      "beardgit/no-hex-in-svelte": "error",
      "no-restricted-imports": noRawInvoke,
    },
  },
  // `src/lib/api/` is the single home of the IPC layer — the only place
  // allowed to import `invoke` from `@tauri-apps/api/core`.
  {
    files: ["src/lib/api/**"],
    rules: {
      "no-restricted-imports": "off",
    },
  },
];

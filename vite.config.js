import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { paraglideVitePlugin as paraglide } from "@inlang/paraglide-js";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const host = process.env.TAURI_DEV_HOST;

// Inline the current package version so the Settings → Updates section
// can render it without a runtime IPC round-trip. Exposed on
// `import.meta.env.VITE_APP_VERSION` at build time.
const pkg = JSON.parse(
  readFileSync(fileURLToPath(new URL("./package.json", import.meta.url)), "utf-8"),
);

// `defineConfig` receives a plain object (not an async factory) so Vite can
// hash the config statically and reuse `node_modules/.vite/deps` across
// runs. With an async wrapper Vite produces a fresh config hash on each
// startup and re-bundles every dependency from scratch (~120 s cold start).
export default defineConfig({
  plugins: [
    paraglide({
      project: "./project.inlang",
      outdir: "./src/lib/paraglide",
    }),
    sveltekit(),
  ],
  define: {
    "import.meta.env.VITE_APP_VERSION": JSON.stringify(pkg.version),
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // Ignore Tauri's Rust source tree and BeardGit's own runtime
      // workspace under `.beardgit/`. The latter is critical: AI
      // background runs check out the current branch into
      // `.beardgit/ai-worktrees/<slug>/`, which on this repo means a
      // duplicate of the entire frontend (tsconfig.json, src/app.html,
      // docs/index.html, …). Without this guard Vite treats those copies
      // as new project files, fires HMR `page reload` events, and the
      // running dev UI blanks for several seconds. Production builds
      // aren't affected — only the dev server.
      ignored: ["**/src-tauri/**", "**/.beardgit/**"],
    },
  },
});

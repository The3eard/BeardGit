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

export default defineConfig(async () => ({
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
      ignored: ["**/src-tauri/**"],
    },
  },
}));

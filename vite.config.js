import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { paraglideVitePlugin as paraglide } from "@inlang/paraglide-js";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [
    paraglide({
      project: "./project.inlang",
      outdir: "./src/lib/paraglide",
    }),
    sveltekit(),
  ],
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

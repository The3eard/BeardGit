import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import path from "path";

export default defineConfig({
  plugins: [svelte()],
  // Keep SVG imports as file URLs (never inline as data URIs) so asset
  // imports are comparable by path in unit tests. Production bundling
  // goes through SvelteKit's own vite.config.js which is unaffected.
  build: {
    assetsInlineLimit: 0,
  },
  test: {
    include: ["src/**/*.test.ts"],
    setupFiles: ["./src/test/setup.ts"],
    environment: "jsdom",
    server: {
      deps: {
        // `@testing-library/svelte` needs to be transformed by Vite so
        // the Svelte 5 runes compile correctly under vitest.
        inline: ["@testing-library/svelte"],
      },
    },
    coverage: {
      provider: "v8",
      reporter: ["text", "text-summary", "html"],
      include: ["src/lib/**/*.ts"],
      exclude: ["src/lib/paraglide/**", "**/*.test.ts", "**/*.d.ts"],
    },
  },
  resolve: {
    alias: {
      $lib: path.resolve(__dirname, "src/lib"),
    },
    // Pick the browser-side Svelte entry so `mount()` works under
    // jsdom (Svelte 5's server build throws lifecycle_function_unavailable).
    conditions: ["browser"],
  },
});

import { defineConfig } from "vitest/config";
import path from "path";

export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'text-summary', 'html'],
      include: ['src/lib/**/*.ts'],
      exclude: ['src/lib/paraglide/**', '**/*.test.ts', '**/*.d.ts'],
    },
  },
  resolve: {
    alias: {
      "$lib": path.resolve(__dirname, "src/lib"),
    },
  },
});

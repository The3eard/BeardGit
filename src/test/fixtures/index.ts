/**
 * Shared fixture factories for vitest (`src/test/setup.ts` +
 * `src/test/e2e/`) and Playwright (`tests/visual/`). Pure functions —
 * no test-framework imports — so the same fixtures drive both layers.
 */

export * from "./repo";
export * from "./commits";
export * from "./branches";
export * from "./changes";
export * from "./mrs";
export * from "./issues";
export * from "./pipelines";
export * from "./theme";
export * from "./prefs";

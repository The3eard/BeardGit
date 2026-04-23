/**
 * Documented brand colors used in log/provider badges. These are
 * intentionally hardcoded because they represent third-party brand
 * identity, not UI theming. Additions require a spec bump + snapshot
 * update in brand-colors.test.ts.
 *
 * Do NOT import these from theme tokens; they deliberately do not shift
 * with the active theme.
 */
export const BRAND_COLORS = {
  anthropic: "#d97757",      // Claude / Anthropic logo accent
  github:    "#24292e",      // GitHub octocat dark
  gitlab:    "#fc6d26",      // GitLab tanuki orange
  openai:    "#10a37f",      // OpenAI green
  codex:     "#000000",      // Codex CLI
  gemini:    "#1a73e8",      // Google Gemini blue
} as const;

export type BrandKey = keyof typeof BRAND_COLORS;

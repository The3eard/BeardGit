/**
 * Stylelint configuration for BeardGit.
 *
 * Purpose: enforce that all color literals in component <style> blocks are
 * routed through CSS custom properties (theme tokens) or the brand-colors
 * allowlist. See docs/superpowers/plans/2026-04-23-theme-color-audit.md §C2.
 *
 * Uses stylelint-config-recommended (correctness-only) rather than
 * stylelint-config-standard to avoid enforcing style/formatting rules that
 * are out of scope for Phase C.
 *
 * Note: stylelint-plugin-svelte does not exist on npm; Svelte file support
 * is provided via postcss-html (stylelint-config-html/svelte).
 *
 * @type {import('stylelint').Config}
 */
module.exports = {
  extends: ["stylelint-config-html/svelte"],
  rules: {
    "color-no-hex": [true, { severity: "error" }],
    "color-named": ["never", { severity: "error" }],
    "function-no-unknown": [true, { ignoreFunctions: ["color-mix"] }],
  },
  overrides: [
    {
      // Plain CSS files parsed with the standard CSS syntax.
      files: ["src/**/*.css"],
      customSyntax: "postcss",
    },
    {
      // Documented sources of truth — hex literals permitted here.
      files: [
        "src/lib/stores/theme.ts",        // owns the hex values it distributes
        "src/lib/utils/status.ts",        // pre-theme-load fallback map
        "src/lib/ui/brand-colors.ts",     // the allowlist
        "src/lib/styles/*.css",           // shell stylesheets — fallbacks permitted
        "src/app.css",                    // root CSS — defines initial token defaults
        "src/routes/+layout.svelte",
        "src/routes/+page.svelte",
      ],
      rules: {
        "color-no-hex": null,
        "color-named": null,
      },
    },
  ],
};

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
    // Close the gap that let mode-dependent colours hide in <style> blocks.
    // `color-no-hex` catches `#fff` but not `rgba(255,255,255,0.15)`, and the
    // ESLint rule `no-hex-in-svelte` only inspects inline `style="…"` and
    // `style:prop` — so a white-on-transparent tint could sit in a component
    // stylesheet and be invisible in every light theme. That is exactly what
    // happened to the scrollbar thumb and the spinner track.
    //
    // Escape hatch is stylelint's own directive with a reason, e.g.
    //   /* stylelint-disable-next-line function-disallowed-list -- modal backdrop */
    // Reach for `var(--token)` or `color-mix()` first.
    "function-disallowed-list": [
      // Every functional colour notation, not just rgb/rgba: `hsl()`,
      // `oklch()` and friends are all viable ways to write the same
      // mode-dependent white tint.
      ["rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklab", "oklch", "color"],
      { severity: "error" },
    ],
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
        // NOTE: `function-disallowed-list` is deliberately NOT nulled here.
        // `src/app.css` is exactly where the white-on-white scrollbar tint
        // lived, so exempting it would leave the guard pointing away from
        // the crime scene. The few literals these files legitimately need
        // carry a per-line `stylelint-disable-next-line` with a reason.
      },
    },
  ],
};

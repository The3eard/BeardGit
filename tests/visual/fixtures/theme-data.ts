/**
 * Real derived `ThemeData` for the showcase themes, fed to
 * `resolve_startup_theme` / `get_theme` in `marketing.spec.ts` so each
 * theme renders faithfully.
 *
 * The data lives in `theme-data.json`, generated from the Rust theme
 * pipeline by `storage::theme::tests::regenerate_theme_fixtures` and
 * pinned byte-for-byte by `test_theme_fixtures_match_live_serialization`.
 * Regenerate with:
 *
 *     cargo test -p storage regenerate_theme_fixtures -- --ignored
 *
 * ## Why this is generated rather than hand-dumped
 *
 * It used to be a hand-pasted dump, taken while `ThemeEditor` still used
 * `#[serde(rename = "added-bg")]` — so every `editor` key was kebab-case
 * and disagreed with `ThemeEditorData`. An `as unknown as` cast on the
 * export hid that from `svelte-check` for months, and the screenshots
 * rendered with fallback syntax colors and no diff backgrounds.
 *
 * The annotation below is deliberately a plain type annotation, **not** a
 * cast: it is what makes a kebab-case regression a compile error rather
 * than a silent visual one.
 */
import type { ThemeData } from "../../../src/lib/types";
// The `with { type: "json" }` attribute is required, not decorative. Specs
// under `tests/` run through Playwright, which loads them with Node's own
// ESM loader — and with `"type": "module"` in package.json, Node rejects a
// bare JSON import. A missing attribute here does not fail this file alone:
// one unloadable module aborts test *collection* for the whole project
// ("Total: 0 tests in 0 files"), silently taking every visual baseline
// with it.
//
// The sibling import in `src/lib/stores/theme.editor-tokens.test.ts` has no
// attribute on purpose — that one runs under Vite/vitest, which resolves
// bare JSON imports natively. The asymmetry is correct; don't harmonise it.
import data from "./theme-data.json" with { type: "json" };

export const SHOWCASE_THEMES: Record<string, ThemeData> = data;

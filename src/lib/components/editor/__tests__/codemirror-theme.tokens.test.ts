/**
 * The editor must follow the app's theme, not carry its own colours.
 *
 * CodeMirror is a third-party plugin with its own theming system, so the
 * easy mistake is to hand it a snapshot of the current theme's hex values.
 * This codebase deliberately doesn't: every colour reaches CodeMirror as a
 * `var(--token)`, which means a theme switch reskins the editor through
 * CSS with no extension rebuild, no stale copy, and no second place for a
 * palette to drift.
 *
 * `createCodemirrorTheme` therefore takes only `isDark` — the one value it
 * genuinely has to bake in, because CodeMirror needs to know which of its
 * own defaults to use. It used to also take the resolved `[editor]` data
 * and ignore it, which made the prop look like the theming path while
 * actually forcing a full view rebuild on every same-mode theme change.
 */

import { describe, expect, it } from "vitest";
import { createCodemirrorTheme } from "../codemirror-theme";

/**
 * Recursively collect every string value in a nested spec object.
 *
 * CodeMirror's `Extension` graph is cyclic (facet providers reference the
 * plugin that owns them), so the visited set is required, not defensive.
 */
function stringValues(
  node: unknown,
  out: string[] = [],
  seen: WeakSet<object> = new WeakSet(),
): string[] {
  if (typeof node === "string") {
    out.push(node);
    return out;
  }
  if (!node || typeof node !== "object") return out;
  if (seen.has(node)) return out;
  seen.add(node);
  for (const value of Object.values(node)) stringValues(value, out, seen);
  return out;
}

/** Anything that looks like a baked-in colour rather than a token. */
const LITERAL_COLOUR =
  /#[0-9a-fA-F]{3,8}\b|\brgba?\(|\bhsla?\(|\boklch\(|\blab\(/;

describe("createCodemirrorTheme — colours come from theme tokens only", () => {
  it.each([true, false])("bakes in no literal colour (isDark=%s)", (isDark) => {
    const theme = createCodemirrorTheme(isDark);

    const literals = stringValues(theme).filter((v) => LITERAL_COLOUR.test(v));

    expect(literals).toEqual([]);
  });

  it("does reference the app's theme tokens", () => {
    // The inverse guard: a spec that produced no colours at all would
    // trivially satisfy the assertion above.
    const values = stringValues(createCodemirrorTheme(true));
    const tokens = new Set(
      values.flatMap((v) => [...v.matchAll(/var\((--[a-z0-9-]+)\)/g)].map((m) => m[1])),
    );

    for (const required of [
      "--bg-primary",
      "--text-primary",
      "--editor-cursor",
      "--editor-gutter-fg",
      "--diff-added-bg",
      "--diff-removed-bg",
      "--syntax-keyword",
      "--syntax-string",
    ]) {
      expect(tokens, `missing ${required}`).toContain(required);
    }
  });

  it("references the same token set in both modes", () => {
    // The tokens are identical between modes because none of them is
    // mode-dependent — what differs is CodeMirror's own `dark` flag, which
    // is exactly why `isDark` is the only value that has to be baked in.
    // A divergence here means a colour decision leaked into the extension
    // instead of living in `computeOverlays`/`applyTheme`.
    const tokensFor = (isDark: boolean) =>
      [
        ...new Set(
          stringValues(createCodemirrorTheme(isDark)).flatMap((v) =>
            [...v.matchAll(/var\((--[a-z0-9-]+)\)/g)].map((m) => m[1]),
          ),
        ),
      ].sort();

    expect(tokensFor(true)).toEqual(tokensFor(false));
    expect(tokensFor(true).length).toBeGreaterThan(10);
  });
});

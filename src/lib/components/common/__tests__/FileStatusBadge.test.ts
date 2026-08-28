/**
 * Guards the one invariant `check_theme_contrast` cannot see.
 *
 * The Rust audit measures each text token against the surfaces it is
 * drawn on — the page and the panel for `--text-muted`. `FileStatusBadge`
 * breaks that assumption if it paints its own background from the same
 * variable it uses for the letter: `background: color-mix(var(--st) 18%,
 * transparent)` creates a *fifth* surface that no theme code knows about,
 * and one that always costs contrast (it lightens under the text in dark
 * mode, darkens in light).
 *
 * Measured over all 31 bundled themes, `--text-muted` on its own 18 % tint
 * bottoms out at 4.04:1 on the page and 3.37:1 on a panel — under the
 * 4.5:1 the audit reports as met. `--text-secondary` bottoms out at 3.82.
 * So the rule is not "pick a brighter token", it is "an audited text token
 * never gets the tinted fill".
 *
 * This is a source-text check because the badge's colours come from CSS
 * custom properties resolved against a real theme, which jsdom does not
 * do: `getComputedStyle` returns the literal `color-mix(…)` string with
 * `var(--st)` unresolved.
 */

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const SOURCE = "src/lib/components/common/FileStatusBadge.svelte";

/** Text tokens `contrast_floor` in `crates/storage/src/theme.rs` audits. */
const AUDITED_TEXT_TOKENS = ["--text-primary", "--text-secondary", "--text-muted"];

describe("FileStatusBadge keeps audited text tokens off self-tinted fills", () => {
  const css = readFileSync(SOURCE, "utf8");

  /** `.is-foo { --st: var(--bar); … }` → `{ foo: "--bar", … }`, with body. */
  const kinds = [...css.matchAll(/\.is-([a-z]+)\s*\{([^}]*)\}/g)].map(([, kind, body]) => ({
    kind,
    body,
    token: body.match(/--st:\s*var\((--[a-z-]+)\)/)?.[1] ?? null,
  }));

  it("still parses the badge's kind rules", () => {
    // Positive anchor: a regex that stopped matching would make every
    // assertion below pass over an empty list.
    expect(kinds.length).toBeGreaterThanOrEqual(8);
    expect(kinds.every((k) => k.token !== null)).toBe(true);
    expect(kinds.map((k) => k.kind)).toContain("unknown");
  });

  it("tints its shared background from --st, which is what makes this matter", () => {
    expect(css).toMatch(/background:\s*color-mix\(in srgb,\s*var\(--st\)\s*\d+%/);
  });

  it("cancels that fill for every kind coloured by an audited text token", () => {
    const offenders = kinds
      .filter((k) => AUDITED_TEXT_TOKENS.includes(k.token!))
      .filter((k) => !/background:\s*(none|transparent)/.test(k.body))
      .map((k) => `.is-${k.kind} uses ${k.token} but keeps the tinted fill`);

    expect(
      offenders,
      "an audited text token drawn on an 18% tint of itself falls below the " +
        "4.5:1 the theme audit reports as met — see the comment in " +
        SOURCE,
    ).toEqual([]);
  });
});

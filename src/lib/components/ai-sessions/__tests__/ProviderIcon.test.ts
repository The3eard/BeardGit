/**
 * Unit tests for ProviderIcon.svelte.
 *
 * Asserts each supported provider resolves to its brand asset (with the
 * correct theme-aware variant for OpenAI/Codex and OpenCode), that the
 * rendered element is a square at the requested size, and that an unknown
 * provider falls back to the generic glyph.
 */
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import ProviderIcon from "../ProviderIcon.svelte";
import { activeTheme } from "$lib/stores/theme";
import type { ThemeData } from "$lib/types";

function fakeTheme(mode: "dark" | "light"): ThemeData {
  /* Only `meta.mode` is read by ProviderIcon; the rest is structural
     filler so the type-check passes. */
  return {
    meta: { id: `t-${mode}`, name: mode, mode, complementary: null },
    colors: {} as ThemeData["colors"],
    derived: {} as ThemeData["derived"],
    graph: {} as ThemeData["graph"],
    editor: null,
  };
}

beforeEach(() => activeTheme.set(fakeTheme("dark")));
afterEach(() => {
  cleanup();
  activeTheme.set(null);
});

describe("ProviderIcon", () => {
  it("renders the Claude Code asset for claude_code", () => {
    const { container } = render(ProviderIcon, {
      props: { provider: "claude_code", size: 20 },
    });
    const img = container.querySelector("img");
    expect(img?.getAttribute("src")).toMatch(/claude-code\.svg$/);
    expect(img?.getAttribute("width")).toBe("20");
    expect(img?.getAttribute("height")).toBe("20");
  });

  it("renders the OpenAI white monoblossom for codex on a dark theme", () => {
    activeTheme.set(fakeTheme("dark"));
    const { container } = render(ProviderIcon, {
      props: { provider: "codex" },
    });
    expect(container.querySelector("img")?.getAttribute("src")).toMatch(
      /openai-white\.svg$/,
    );
  });

  it("renders the OpenAI black monoblossom for codex on a light theme", () => {
    activeTheme.set(fakeTheme("light"));
    const { container } = render(ProviderIcon, {
      props: { provider: "codex" },
    });
    expect(container.querySelector("img")?.getAttribute("src")).toMatch(
      /openai-black\.svg$/,
    );
  });

  it("renders the OpenCode dark variant for open_code on a dark theme", () => {
    activeTheme.set(fakeTheme("dark"));
    const { container } = render(ProviderIcon, {
      props: { provider: "open_code" },
    });
    expect(container.querySelector("img")?.getAttribute("src")).toMatch(
      /opencode-dark\.svg$/,
    );
  });

  it("renders the OpenCode light variant for open_code on a light theme", () => {
    activeTheme.set(fakeTheme("light"));
    const { container } = render(ProviderIcon, {
      props: { provider: "open_code" },
    });
    expect(container.querySelector("img")?.getAttribute("src")).toMatch(
      /opencode-light\.svg$/,
    );
  });

  it("falls back to generic.svg for unknown providers", () => {
    const { container } = render(ProviderIcon, {
      // Intentionally pass an unrecognised key to exercise the fallback path.
      props: { provider: "unknown_provider" },
    });
    expect(container.querySelector("img")?.getAttribute("src")).toMatch(
      /generic\.svg$/,
    );
  });

  it("defaults to 20px square", () => {
    const { container } = render(ProviderIcon, {
      props: { provider: "codex" },
    });
    const img = container.querySelector("img");
    expect(img?.getAttribute("width")).toBe("20");
    expect(img?.getAttribute("height")).toBe("20");
  });
});

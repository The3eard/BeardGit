/**
 * Unit tests for ProviderIcon.svelte.
 *
 * Asserts each supported provider resolves to its brand asset, that the
 * rendered element is a square at the requested size, and that an unknown
 * provider falls back to the generic glyph.
 */
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import ProviderIcon from "../ProviderIcon.svelte";

afterEach(() => cleanup());

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

  it("renders the Codex asset for codex", () => {
    const { container } = render(ProviderIcon, {
      props: { provider: "codex" },
    });
    expect(container.querySelector("img")?.getAttribute("src")).toMatch(
      /codex\.svg$/,
    );
  });

  it("renders the OpenCode asset for open_code", () => {
    const { container } = render(ProviderIcon, {
      props: { provider: "open_code" },
    });
    expect(container.querySelector("img")?.getAttribute("src")).toMatch(
      /opencode\.svg$/,
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

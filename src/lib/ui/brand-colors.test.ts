import { describe, it, expect } from "vitest";
import { BRAND_COLORS } from "./brand-colors";

describe("BRAND_COLORS", () => {
  it("freezes the documented allowlist (update spec + snapshot together)", () => {
    expect(BRAND_COLORS).toMatchInlineSnapshot(`
      {
        "anthropic": "#d97757",
        "codex": "#000000",
        "gemini": "#1a73e8",
        "github": "#24292e",
        "gitlab": "#fc6d26",
        "openai": "#10a37f",
      }
    `);
  });

  it("every value is a valid 6-digit hex", () => {
    for (const [name, value] of Object.entries(BRAND_COLORS)) {
      expect(value, name).toMatch(/^#[0-9a-fA-F]{6}$/);
    }
  });
});

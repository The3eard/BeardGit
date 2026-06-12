import { describe, it, expect } from "vitest";
import { defaultGraphTheme } from "./graph-renderer";

describe("graph-renderer bisect theme defaults", () => {
  it("has bisect color fields in default theme", () => {
    const theme = defaultGraphTheme();
    expect(theme.bisectGoodColor).toContain("63, 185, 80");
    expect(theme.bisectBadColor).toContain("248, 81, 73");
    expect(theme.bisectSkipColor).toContain("139, 148, 158");
    expect(theme.bisectCurrentColor).toContain("227, 179, 65");
  });
});

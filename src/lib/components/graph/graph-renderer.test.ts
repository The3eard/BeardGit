import { describe, it, expect } from "vitest";
import { DEFAULT_GRAPH_THEME } from "./graph-renderer";

describe("graph-renderer bisect theme defaults", () => {
  it("has bisect color fields in default theme", () => {
    expect(DEFAULT_GRAPH_THEME.bisectGoodColor).toContain("63, 185, 80");
    expect(DEFAULT_GRAPH_THEME.bisectBadColor).toContain("248, 81, 73");
    expect(DEFAULT_GRAPH_THEME.bisectSkipColor).toContain("139, 148, 158");
    expect(DEFAULT_GRAPH_THEME.bisectCurrentColor).toContain("227, 179, 65");
  });
});

import { describe, it, expect } from "vitest";

const FALLBACK_COLORS: Record<string, string> = {
  success: "#3fb950",
  failed: "#f85149",
  running: "#58a6ff",
  pending: "#d29922",
  queued: "#d29922",
  canceled: "#666666",
  skipped: "#484f58",
  manual: "#bb80ff",
  timed_out: "#f85149",
};

function ciStatusColorFallback(status: string): string {
  return FALLBACK_COLORS[status] ?? "#666666";
}

describe("ciStatusColor fallbacks", () => {
  it("returns green for success", () => expect(ciStatusColorFallback("success")).toBe("#3fb950"));
  it("returns red for failed", () => expect(ciStatusColorFallback("failed")).toBe("#f85149"));
  it("returns blue for running", () => expect(ciStatusColorFallback("running")).toBe("#58a6ff"));
  it("returns yellow for pending", () => expect(ciStatusColorFallback("pending")).toBe("#d29922"));
  it("returns yellow for queued", () => expect(ciStatusColorFallback("queued")).toBe("#d29922"));
  it("returns grey for canceled", () => expect(ciStatusColorFallback("canceled")).toBe("#666666"));
  it("returns dark grey for skipped", () => expect(ciStatusColorFallback("skipped")).toBe("#484f58"));
  it("returns purple for manual", () => expect(ciStatusColorFallback("manual")).toBe("#bb80ff"));
  it("returns red for timed_out", () => expect(ciStatusColorFallback("timed_out")).toBe("#f85149"));
  it("returns default grey for unknown", () => expect(ciStatusColorFallback("xyz")).toBe("#666666"));
});

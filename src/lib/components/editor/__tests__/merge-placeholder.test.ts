import { describe, it, expect } from "vitest";
import { EditorState, Text } from "@codemirror/state";
import {
  decideSide,
  findPlaceholders,
  formatPlaceholder,
  parsePlaceholder,
  pendingMarker,
} from "../merge-placeholder";

function apply(doc: string, ...args: Parameters<typeof decideSide> extends [Text, ...infer R] ? R : never) {
  const state = EditorState.create({ doc });
  const changes = decideSide(state.doc, ...args);
  if (!changes) return null;
  return state.update({ changes }).state.doc.toString();
}

describe("placeholder format", () => {
  it("round-trips pending and decided markers", () => {
    const pending = pendingMarker(3);
    expect(formatPlaceholder(pending)).toBe("◆ CONFLICT 3");
    expect(parsePlaceholder(formatPlaceholder(pending))).toEqual(pending);

    const half = { index: 12, theirs: "accepted", ours: "pending" } as const;
    expect(parsePlaceholder(formatPlaceholder(half))).toEqual(half);
  });

  it("rejects ordinary lines and malformed markers", () => {
    expect(parsePlaceholder("let x = 1;")).toBeNull();
    expect(parsePlaceholder("◆ CONFLICT x")).toBeNull();
    expect(parsePlaceholder("◆ CONFLICT 1 [theirs:maybe ours:pending]")).toBeNull();
  });

  it("finds every placeholder with its line number", () => {
    const doc = Text.of(["a", formatPlaceholder(pendingMarker(0)), "b", formatPlaceholder(pendingMarker(1))]);
    expect(findPlaceholders(doc).map((h) => [h.marker.index, h.lineNumber])).toEqual([[0, 2], [1, 4]]);
  });
});

describe("decideSide", () => {
  const doc = ["head", "◆ CONFLICT 0", "tail"].join("\n");

  it("accepting one side inserts its lines and keeps the placeholder", () => {
    expect(apply(doc, 0, "theirs", "accepted", ["T1", "T2"])).toBe(
      ["head", "T1", "T2", "◆ CONFLICT 0 [theirs:accepted ours:pending]", "tail"].join("\n"),
    );
  });

  it("deciding the second side removes the placeholder line", () => {
    const half = apply(doc, 0, "theirs", "accepted", ["T1"])!;
    expect(apply(half, 0, "ours", "accepted", ["O1"])).toBe(["head", "T1", "O1", "tail"].join("\n"));
    expect(apply(half, 0, "ours", "discarded", ["O1"])).toBe(["head", "T1", "tail"].join("\n"));
  });

  it("discarding both sides leaves no trace of the conflict", () => {
    const half = apply(doc, 0, "theirs", "discarded", ["T1"])!;
    expect(apply(half, 0, "ours", "discarded", ["O1"])).toBe("head\ntail");
  });

  it("removes a trailing placeholder together with the newline before it", () => {
    const half = apply("head\n◆ CONFLICT 0", 0, "ours", "discarded", [])!;
    expect(apply(half, 0, "theirs", "discarded", [])).toBe("head");
  });

  it("handles a document that is only the placeholder", () => {
    const half = apply("◆ CONFLICT 0", 0, "ours", "accepted", ["O1"])!;
    expect(half).toBe("O1\n◆ CONFLICT 0 [theirs:pending ours:accepted]");
    expect(apply(half, 0, "theirs", "discarded", [])).toBe("O1");
  });

  it("accepting a side that deleted the block inserts nothing", () => {
    expect(apply(doc, 0, "theirs", "accepted", [])).toBe(
      ["head", "◆ CONFLICT 0 [theirs:accepted ours:pending]", "tail"].join("\n"),
    );
  });

  it("refuses a second decision on the same side or a missing placeholder", () => {
    const half = apply(doc, 0, "theirs", "accepted", ["T1"])!;
    expect(apply(half, 0, "theirs", "discarded", [])).toBeNull();
    expect(apply(doc, 7, "ours", "accepted", ["x"])).toBeNull();
  });
});

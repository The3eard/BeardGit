/**
 * Ensures GitGraph's "Create branch at {sha}..." action no longer
 * calls `window.prompt` — the dialog is the new entry point. We
 * cannot render the full canvas-based component, so we test the
 * extracted `buildCreateBranchSource` helper introduced alongside
 * the refactor.
 */

import { describe, it, expect } from "vitest";
import { buildCreateBranchSource } from "../GitGraph.helpers";

describe("GitGraph → CreateBranchDialog handoff", () => {
  it("wraps a commit oid into a { kind: 'commit' } source", () => {
    expect(buildCreateBranchSource("abc123")).toEqual({
      kind: "commit",
      oid: "abc123",
    });
  });
});

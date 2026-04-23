/**
 * Unit tests for the `suggestLocalName` helper used by
 * `CreateBranchDialog` to pre-fill the "Name" field when a remote
 * tracking ref is selected as the source.
 */

import { describe, it, expect } from "vitest";
import {
  suggestLocalName,
  type InitialSource,
} from "../suggest-local-name";

describe("suggestLocalName", () => {
  const remotes = ["origin", "upstream"];

  it("returns '' for the HEAD source", () => {
    const s: InitialSource = { kind: "head" };
    expect(suggestLocalName(s, remotes)).toBe("");
  });

  it("returns '' for a commit source", () => {
    const s: InitialSource = { kind: "commit", oid: "abc123" };
    expect(suggestLocalName(s, remotes)).toBe("");
  });

  it("returns '' when a local branch ref is passed", () => {
    const s: InitialSource = { kind: "ref", name: "feature/foo", oid: "d" };
    expect(suggestLocalName(s, remotes)).toBe("");
  });

  it("strips the matching remote prefix from a remote ref", () => {
    const s: InitialSource = { kind: "ref", name: "origin/feature/foo", oid: "d" };
    expect(suggestLocalName(s, remotes)).toBe("feature/foo");
  });

  it("strips the first matching remote when multiple remotes share a prefix head", () => {
    const s: InitialSource = { kind: "ref", name: "upstream/main", oid: "d" };
    expect(suggestLocalName(s, remotes)).toBe("main");
  });

  it("preserves nested slashes after stripping", () => {
    const s: InitialSource = {
      kind: "ref",
      name: "origin/release/2026.04/hotfix",
      oid: "d",
    };
    expect(suggestLocalName(s, remotes)).toBe("release/2026.04/hotfix");
  });

  it("returns '' when the remote list is empty", () => {
    const s: InitialSource = { kind: "ref", name: "origin/foo", oid: "d" };
    expect(suggestLocalName(s, [])).toBe("");
  });
});

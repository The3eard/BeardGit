import { describe, it, expect } from "vitest";
import { parseRemoteBranch } from "../parse-remote-branch";

describe("parseRemoteBranch", () => {
  it("splits a simple <remote>/<branch>", () => {
    expect(parseRemoteBranch("origin/main")).toEqual({
      remote: "origin",
      branch: "main",
    });
  });

  it("preserves nested branch segments after the first slash", () => {
    // `feature/foo` is a legitimate branch name on `origin`; only the
    // first slash separates remote from branch.
    expect(parseRemoteBranch("origin/feature/foo")).toEqual({
      remote: "origin",
      branch: "feature/foo",
    });
  });

  it("works with non-default remote names", () => {
    expect(parseRemoteBranch("upstream/release-2.0")).toEqual({
      remote: "upstream",
      branch: "release-2.0",
    });
  });

  it("returns null for the symbolic HEAD ref", () => {
    // origin/HEAD tracks the default branch and isn't directly deletable;
    // the menu must not offer it.
    expect(parseRemoteBranch("origin/HEAD")).toBeNull();
  });

  it("returns null for a single-segment value", () => {
    expect(parseRemoteBranch("origin")).toBeNull();
  });

  it("returns null for an empty string", () => {
    expect(parseRemoteBranch("")).toBeNull();
  });

  it("returns null for a leading slash", () => {
    expect(parseRemoteBranch("/main")).toBeNull();
  });

  it("returns null for a trailing slash", () => {
    expect(parseRemoteBranch("origin/")).toBeNull();
  });
});

import { describe, it, expect } from "vitest";
import { parseOpenProjectError } from "../errors";

describe("parseOpenProjectError", () => {
  it("recognises typed not_a_repo payload", () => {
    const out = parseOpenProjectError({ kind: "not_a_repo", path: "/tmp/foo" });
    expect(out).toEqual({ kind: "not_a_repo", path: "/tmp/foo" });
  });

  it("recognises typed other payload", () => {
    const out = parseOpenProjectError({ kind: "other", message: "boom" });
    expect(out).toEqual({ kind: "other", message: "boom" });
  });

  it("falls back to other for plain strings", () => {
    const out = parseOpenProjectError("plain string error");
    expect(out).toEqual({ kind: "other", message: "plain string error" });
  });

  it("returns null for an unrecognised shape", () => {
    expect(parseOpenProjectError({ foo: "bar" })).toBeNull();
  });
});

import { describe, it, expect } from "vitest";
import {
  getErrorCode,
  getErrorMessage,
  firstErrorLine,
  errorCodeMessage,
} from "../errors";

describe("getErrorCode", () => {
  it("returns the code from a structured IpcError", () => {
    expect(getErrorCode({ code: "not_a_repo", message: "/x" })).toBe(
      "not_a_repo",
    );
  });

  it("returns null for a plain string error", () => {
    expect(getErrorCode("plain string error")).toBeNull();
  });

  it("returns null when no string code is present", () => {
    expect(getErrorCode({ message: "boom" })).toBeNull();
    expect(getErrorCode({ code: 42 })).toBeNull();
    expect(getErrorCode(null)).toBeNull();
  });
});

describe("getErrorMessage", () => {
  it("passes plain strings through", () => {
    expect(getErrorMessage("boom")).toBe("boom");
  });

  it("reads .message from an IpcError object", () => {
    expect(getErrorMessage({ code: "not_a_repo", message: "/tmp/foo" })).toBe(
      "/tmp/foo",
    );
  });

  it("reads Error.message", () => {
    expect(getErrorMessage(new Error("nope"))).toBe("nope");
  });

  it("falls back to String() for odd shapes", () => {
    expect(getErrorMessage(42)).toBe("42");
  });
});

describe("firstErrorLine", () => {
  it("returns only the first line of a multi-line message", () => {
    expect(firstErrorLine("line1\nline2")).toBe("line1");
    expect(firstErrorLine({ code: "x", message: "a\r\nb" })).toBe("a");
  });
});

describe("errorCodeMessage", () => {
  it("maps known codes to a label", () => {
    expect(errorCodeMessage("auth_required")).toBe("Authentication required");
  });

  it("returns null for unmapped codes", () => {
    expect(errorCodeMessage("something_else")).toBeNull();
  });
});

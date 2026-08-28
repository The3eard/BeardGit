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

/**
 * The regression that made this suite matter: every Tauri command now
 * rejects with `{ code, message }`, and `String()` on an object is the
 * literal text "[object Object]". Three callsites compared the result
 * against known text to decide what to do next, so the damage was control
 * flow, not just a bad-looking toast.
 */
describe("an IpcError is not usefully stringifiable", () => {
  const rejection = { code: "error", message: "canceled" };

  it("String() on it loses the message entirely", () => {
    expect(String(rejection)).toBe("[object Object]");
  });

  it("getErrorMessage keeps it, so a text comparison still works", () => {
    expect(getErrorMessage(rejection)).toBe("canceled");
    expect(getErrorMessage(rejection).includes("canceled")).toBe(true);
  });

  it("and still works on the shapes that are not commands", () => {
    expect(getErrorMessage("canceled")).toBe("canceled");
    expect(getErrorMessage(new Error("canceled"))).toBe("canceled");
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

  it("maps not_a_repo and its legacy repo_not_found alias to the same label", () => {
    expect(errorCodeMessage("not_a_repo")).toBe("Not a git repository");
    expect(errorCodeMessage("repo_not_found")).toBe("Not a git repository");
  });

  it("maps the branch/checkout envelope codes to actionable labels", () => {
    expect(errorCodeMessage("would_lose_changes")).toBe(
      "Checkout would overwrite uncommitted changes — commit or stash first",
    );
    expect(errorCodeMessage("not_fully_merged")).toBe(
      "Branch has unmerged commits — delete with force to discard them",
    );
    expect(errorCodeMessage("branch_exists")).toBe(
      "A branch with that name already exists — choose a different name",
    );
  });

  it("returns null for unmapped codes", () => {
    expect(errorCodeMessage("something_else")).toBeNull();
  });
});

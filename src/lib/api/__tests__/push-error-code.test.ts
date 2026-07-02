/**
 * Store-level assertion for the typed error envelope (spec 05, Phase 3):
 * a push that fails auth surfaces its structured `code` through the same
 * `runMutation` façade the push call sites use, so the frontend can branch
 * on it rather than parsing free text.
 */
import { describe, it, expect, vi, beforeEach } from "vitest";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

import { pushRemote } from "$lib/api/tauri";
import { runMutation } from "$lib/api/runMutation";
import { getErrorCode } from "$lib/api/errors";

beforeEach(() => mocks.invoke.mockReset());

describe("push auth failure surfaces its code", () => {
  it("rejects with an IpcError whose code getErrorCode extracts", async () => {
    mocks.invoke.mockRejectedValueOnce({
      code: "auth_required",
      message: "authentication failed",
    });

    let caught: unknown;
    await runMutation({
      kind: "push",
      invoke: () => pushRemote("origin", "main", false),
      failureToastPrefix: "Push failed",
    }).catch((e) => {
      caught = e;
    });

    expect(getErrorCode(caught)).toBe("auth_required");
    expect(mocks.invoke).toHaveBeenCalledWith("push_remote", {
      remote: "origin",
      branch: "main",
      force: false,
    });
  });
});

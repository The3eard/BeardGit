/**
 * Registering the PR file shortcuts wires the bracket keys to
 * handlePrFileNav; unregistering removes them from the registry.
 */
import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { shortcuts, registerShortcuts, unregisterShortcuts } from "$lib/stores/shortcuts";
import { registerPrDiffShortcuts, unregisterPrDiffShortcuts } from "$lib/stores/mr-pr";

describe("PR diff shortcuts", () => {
  beforeEach(() => {
    // Clean slate.
    const ids = get(shortcuts).map((s) => s.id);
    unregisterShortcuts(ids);
  });

  it("registers prDiff.prev and prDiff.next with bracket keys", () => {
    registerPrDiffShortcuts({ onPrev: () => {}, onNext: () => {} });
    const list = get(shortcuts);
    const prev = list.find((s) => s.id === "prDiff.prev");
    const next = list.find((s) => s.id === "prDiff.next");
    expect(prev?.keys.key).toBe("[");
    expect(next?.keys.key).toBe("]");
  });

  it("unregisterPrDiffShortcuts removes both", () => {
    registerPrDiffShortcuts({ onPrev: () => {}, onNext: () => {} });
    unregisterPrDiffShortcuts();
    const list = get(shortcuts);
    expect(list.find((s) => s.id === "prDiff.prev")).toBeUndefined();
    expect(list.find((s) => s.id === "prDiff.next")).toBeUndefined();
  });
});

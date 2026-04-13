import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";
import { toasts, addToast, updateToast, removeToast } from "../toast";

beforeEach(() => {
  const current = get(toasts);
  current.forEach((t) => removeToast(t.id));
});

describe("toast store", () => {
  it("addToast creates a toast with defaults", () => {
    const id = addToast({ message: "Hello", type: "info" });
    const list = get(toasts);
    expect(list).toHaveLength(1);
    expect(list[0].id).toBe(id);
    expect(list[0].message).toBe("Hello");
    expect(list[0].type).toBe("info");
    expect(list[0].dismissible).toBe(true);
    expect(list[0].duration).toBe(5000);
  });

  it("addToast respects custom options", () => {
    const onclick = vi.fn();
    const id = addToast({
      message: "Custom",
      type: "error",
      dismissible: false,
      duration: null,
      actions: [{ label: "Retry", onclick }],
    });
    const t = get(toasts).find((t) => t.id === id)!;
    expect(t.dismissible).toBe(false);
    expect(t.duration).toBeNull();
    expect(t.actions).toHaveLength(1);
    expect(t.actions![0].label).toBe("Retry");
  });

  it("enforces max 3 toasts by removing oldest", () => {
    addToast({ message: "A", type: "info" });
    addToast({ message: "B", type: "info" });
    addToast({ message: "C", type: "info" });
    addToast({ message: "D", type: "info" });
    const list = get(toasts);
    expect(list).toHaveLength(3);
    expect(list.map((t) => t.message)).toEqual(["B", "C", "D"]);
  });

  it("updateToast mutates an existing toast", () => {
    const id = addToast({ message: "Old", type: "info" });
    updateToast(id, { message: "New", type: "success" });
    const t = get(toasts).find((t) => t.id === id)!;
    expect(t.message).toBe("New");
    expect(t.type).toBe("success");
  });

  it("updateToast ignores unknown id", () => {
    addToast({ message: "Existing", type: "info" });
    updateToast("nonexistent", { message: "Ghost" });
    const list = get(toasts);
    expect(list).toHaveLength(1);
    expect(list[0].message).toBe("Existing");
  });

  it("removeToast removes by id", () => {
    const id = addToast({ message: "Gone", type: "info" });
    expect(get(toasts)).toHaveLength(1);
    removeToast(id);
    expect(get(toasts)).toHaveLength(0);
  });
});

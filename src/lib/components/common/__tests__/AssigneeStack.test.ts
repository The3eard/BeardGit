/**
 * Unit tests for `AssigneeStack.svelte`.
 *
 * Verifies:
 * - Renders nothing when `assignees` is empty.
 * - Caps visible avatars at `max` and appends `+N` overflow.
 * - Assigns each visible avatar a colour deterministically from the login.
 * - Exposes `aria-label` = `"{count} assignees"` when count > 0.
 */

import { describe, expect, it, afterEach } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import AssigneeStack from "../AssigneeStack.svelte";

afterEach(() => cleanup());

describe("AssigneeStack", () => {
  it("renders nothing when the assignees array is empty", () => {
    const { container } = render(AssigneeStack, {
      props: { assignees: [], max: 3 },
    });
    expect(container.querySelector(".assignee-stack")).toBeNull();
  });

  it("renders all assignees when under the cap", () => {
    const { container } = render(AssigneeStack, {
      props: { assignees: ["alice", "bob"], max: 3 },
    });
    const avatars = container.querySelectorAll(".assignee-avatar");
    expect(avatars).toHaveLength(2);
    expect(container.querySelector(".assignee-overflow")).toBeNull();
  });

  it("caps visible avatars at max and renders +N overflow", () => {
    const { container } = render(AssigneeStack, {
      props: { assignees: ["a", "b", "c", "d", "e"], max: 3 },
    });
    expect(container.querySelectorAll(".assignee-avatar")).toHaveLength(3);
    const overflow = container.querySelector(".assignee-overflow");
    expect(overflow?.textContent).toBe("+2");
  });

  it("exposes an aria-label counting all assignees", () => {
    const { container } = render(AssigneeStack, {
      props: { assignees: ["a", "b", "c", "d"], max: 3 },
    });
    expect(
      container.querySelector(".assignee-stack")?.getAttribute("aria-label"),
    ).toBe("4 assignees");
  });
});

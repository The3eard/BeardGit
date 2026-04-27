/**
 * aiActiveTerminals — selection store + mutual-exclusion helper.
 *
 * The three AI-session selection stores (`selectedConversationId`,
 * `selectedBackgroundSessionId`, `selectedActiveTerminal`) are mutually
 * exclusive. Each row's `onSelect` uses a shared helper to set one and
 * clear the other two; this test covers all three pair-wise transitions.
 */
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { get } from "svelte/store";
import type { ActiveTerminal } from "../aiActiveTerminals";
import {
  selectedActiveTerminal,
  selectAiSessionRow,
} from "../aiActiveTerminals";
import { selectedConversationId } from "../aiConversations";
import { selectedBackgroundSessionId } from "../aiBackground";

const TAB_ACTIVE: ActiveTerminal = {
  kind: "tab",
  tabIndex: 0,
  info: {
    sessionId: 1,
    title: "Claude",
    cwd: "/repos/demo",
    provider: "claude_code",
  },
};

function reset() {
  selectedConversationId.set(null);
  selectedBackgroundSessionId.set(null);
  selectedActiveTerminal.set(null);
}

beforeEach(reset);
afterEach(reset);

describe("selectedActiveTerminal store", () => {
  it("defaults to null", () => {
    expect(get(selectedActiveTerminal)).toBeNull();
  });

  it("accepts an ActiveTerminal payload", () => {
    selectedActiveTerminal.set(TAB_ACTIVE);
    expect(get(selectedActiveTerminal)).toEqual(TAB_ACTIVE);
  });
});

describe("selectAiSessionRow — mutual exclusion", () => {
  it("selecting a conversation clears bg + active-terminal", () => {
    selectedBackgroundSessionId.set("bg-1");
    selectedActiveTerminal.set(TAB_ACTIVE);

    selectAiSessionRow({ kind: "conversation", id: "conv-1" });

    expect(get(selectedConversationId)).toBe("conv-1");
    expect(get(selectedBackgroundSessionId)).toBeNull();
    expect(get(selectedActiveTerminal)).toBeNull();
  });

  it("selecting a bg run clears conversation + active-terminal", () => {
    selectedConversationId.set("conv-1");
    selectedActiveTerminal.set(TAB_ACTIVE);

    selectAiSessionRow({ kind: "background", id: "bg-1" });

    expect(get(selectedBackgroundSessionId)).toBe("bg-1");
    expect(get(selectedConversationId)).toBeNull();
    expect(get(selectedActiveTerminal)).toBeNull();
  });

  it("selecting an active tab/segment clears conversation + bg", () => {
    selectedConversationId.set("conv-1");
    selectedBackgroundSessionId.set("bg-1");

    selectAiSessionRow({ kind: "active", active: TAB_ACTIVE });

    expect(get(selectedActiveTerminal)).toEqual(TAB_ACTIVE);
    expect(get(selectedConversationId)).toBeNull();
    expect(get(selectedBackgroundSessionId)).toBeNull();
  });
});

/**
 * The staging calls must carry the context the displayed diff was fetched
 * with.
 *
 * A `HunkSelection` is positional — hunk 2, lines 5–7 of the array this pane
 * rendered — and the backend re-derives its own diff to resolve those
 * indices. Ask it for a different context and libgit2 cuts the file into
 * different hunks with different line arrays, so the same indices name
 * different lines: the patch applies cleanly, the toast says it worked, and
 * the wrong lines are staged. With "show whole file" on, that is one click
 * away.
 *
 * The Rust side has the end-to-end proof
 * (`test_stage_hunks_honours_the_context_the_selection_was_made_with`). This
 * guards the half that lives here: that the argument is passed at all, from
 * every one of the three actions.
 */

import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  stageHunks: vi.fn(() => Promise.resolve()),
  unstageHunks: vi.fn(() => Promise.resolve()),
  discardHunks: vi.fn(() => Promise.resolve()),
}));

vi.mock("$lib/api/tauri", () => ({
  stageHunks: mocks.stageHunks,
  unstageHunks: mocks.unstageHunks,
  discardHunks: mocks.discardHunks,
}));

import StagingDiffEditor from "../StagingDiffEditor.svelte";
import {
  DEFAULT_DIFF_CONTEXT,
  FULL_FILE_CONTEXT,
  stagingDiffContext,
} from "$lib/stores/changes";
import { makeFileDiff } from "../../../../test/fixtures";

/** Select every changed line, which is what "Select all" does. */
async function selectAllLines() {
  await fireEvent.click(screen.getByRole("button", { name: "Select all" }));
}

describe("StagingDiffEditor sends the diff's context with the selection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    stagingDiffContext.set(DEFAULT_DIFF_CONTEXT);
  });

  afterEach(cleanup);

  it("stages with the default context when the pane is not expanded", async () => {
    render(StagingDiffEditor, {
      props: { diff: makeFileDiff({ path: "src/a.ts" }), isStaged: false, filename: "src/a.ts" },
    });

    await selectAllLines();
    await fireEvent.click(screen.getByRole("button", { name: "Stage selected" }));

    expect(mocks.stageHunks).toHaveBeenCalledWith(
      "src/a.ts",
      expect.any(Array),
      DEFAULT_DIFF_CONTEXT,
    );
  });

  it("stages with the full-file context once the pane is expanded", async () => {
    stagingDiffContext.set(FULL_FILE_CONTEXT);
    render(StagingDiffEditor, {
      props: { diff: makeFileDiff({ path: "src/a.ts" }), isStaged: false, filename: "src/a.ts" },
    });

    await selectAllLines();
    await fireEvent.click(screen.getByRole("button", { name: "Stage selected" }));

    expect(mocks.stageHunks).toHaveBeenCalledWith(
      "src/a.ts",
      expect.any(Array),
      FULL_FILE_CONTEXT,
    );
  });

  it("discards with the context too — the action that cannot be undone", async () => {
    stagingDiffContext.set(FULL_FILE_CONTEXT);
    render(StagingDiffEditor, {
      props: { diff: makeFileDiff({ path: "src/a.ts" }), isStaged: false, filename: "src/a.ts" },
    });

    await selectAllLines();
    await fireEvent.click(screen.getByRole("button", { name: "Discard selected" }));
    // Discard is behind a confirmation.
    await fireEvent.click(screen.getByTestId("dialog-confirm-btn"));

    expect(mocks.discardHunks).toHaveBeenCalledWith(
      "src/a.ts",
      expect.any(Array),
      FULL_FILE_CONTEXT,
    );
  });

  it("unstages with the context", async () => {
    stagingDiffContext.set(FULL_FILE_CONTEXT);
    render(StagingDiffEditor, {
      props: { diff: makeFileDiff({ path: "src/a.ts" }), isStaged: true, filename: "src/a.ts" },
    });

    await selectAllLines();
    await fireEvent.click(screen.getByRole("button", { name: "Unstage selected" }));

    expect(mocks.unstageHunks).toHaveBeenCalledWith(
      "src/a.ts",
      expect.any(Array),
      FULL_FILE_CONTEXT,
    );
  });
});

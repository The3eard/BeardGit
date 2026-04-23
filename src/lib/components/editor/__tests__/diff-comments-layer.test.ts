/**
 * Unit tests for the diff-comments-layer extension.
 *
 * Uses a bare EditorState (not a MergeView) since the extension must
 * compose into any ReadOnly editor view.
 */
import { describe, it, expect, vi } from "vitest";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { diffCommentsLayer } from "$lib/components/editor/diff-comments-layer";
import type { ForgeComment } from "$lib/types";

function mkComment(line: number, body: string, id = Math.random()): ForgeComment {
  return {
    id, author: "a", body, created_at: "", path: "f.ts",
    line, is_review: true, resolvable: null, resolved: null, discussion_id: null,
  };
}

describe("diffCommentsLayer", () => {
  it("renders a count bubble on lines with comments", () => {
    const parent = document.createElement("div");
    const view = new EditorView({
      state: EditorState.create({
        doc: "a\nb\nc\nd\n",
        extensions: [diffCommentsLayer({
          comments: [mkComment(2, "x"), mkComment(2, "y"), mkComment(4, "z")],
          onPost: vi.fn(),
          onReply: vi.fn(),
        })],
      }),
      parent,
    });
    const bubbles = parent.querySelectorAll(".cm-comment-bubble");
    expect(bubbles.length).toBe(2);
    expect([...bubbles].find((b) => b.textContent === "2")).toBeTruthy();
    expect([...bubbles].find((b) => b.textContent === "1")).toBeTruthy();
    view.destroy();
  });

  it("calls onPost with the clicked line + composer body", async () => {
    const onPost = vi.fn().mockResolvedValue(undefined);
    const parent = document.createElement("div");
    const view = new EditorView({
      state: EditorState.create({
        doc: "a\nb\nc\n",
        extensions: [diffCommentsLayer({
          comments: [],
          onPost,
          onReply: vi.fn(),
        })],
      }),
      parent,
    });
    // Simulate opening composer on line 2 via the layer's exposed test hook:
    const { __openComposerForTest } = await import("$lib/components/editor/diff-comments-layer");
    __openComposerForTest(view, 2);
    const ta = parent.querySelector("textarea.cm-comment-composer") as HTMLTextAreaElement;
    expect(ta).toBeTruthy();
    ta.value = "hello";
    const submit = parent.querySelector(".cm-comment-submit") as HTMLButtonElement;
    submit.click();
    await new Promise((r) => setTimeout(r, 0));
    expect(onPost).toHaveBeenCalledWith(2, "hello");
    view.destroy();
  });
});

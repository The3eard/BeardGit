/**
 * When `commentsLayer` is passed, DiffEditor composes the extension into
 * the right-side merge view so inline bubbles render on the correct
 * (new) side.
 */
import { describe, it, expect, vi } from "vitest";
import { render, waitFor } from "@testing-library/svelte";
import DiffEditor from "$lib/components/editor/DiffEditor.svelte";

describe("DiffEditor with commentsLayer", () => {
  it("renders a comment bubble in the gutter on line 2", async () => {
    const { container } = render(DiffEditor, {
      oldContent: "a\nb\nc\n",
      newContent: "a\nB\nc\n",
      commentsLayer: {
        comments: [{
          id: 1, author: "a", body: "x", created_at: "",
          path: "f.ts", line: 2, is_review: true,
          resolvable: null, resolved: null, discussion_id: null,
        }],
        onPost: vi.fn(),
        onReply: vi.fn(),
      },
    });
    await waitFor(() => expect(container.querySelector(".cm-comment-bubble")).toBeTruthy());
  });
});

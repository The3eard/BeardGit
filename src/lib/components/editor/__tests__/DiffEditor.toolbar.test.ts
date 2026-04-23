/**
 * DiffEditor renders the `toolbar` snippet in place of the default header.
 */
import { describe, it, expect, afterEach } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import { createRawSnippet } from "svelte";
import DiffEditor from "$lib/components/editor/DiffEditor.svelte";

afterEach(() => cleanup());

describe("DiffEditor toolbar slot", () => {
  it("renders the snippet when provided", () => {
    // When `toolbar` is set, the default `.diff-filename` span must not render.
    // Use createRawSnippet to provide a valid Svelte 5 snippet.
    const toolbarSnippet = createRawSnippet(() => ({
      render: () => '<span class="custom-toolbar">Toolbar</span>',
    }));
    const { getByText } = render(DiffEditor, {
      oldContent: "a", newContent: "b",
      onClose: () => {},
      toolbar: toolbarSnippet,
    });
    // The custom toolbar content should be visible.
    expect(getByText("Toolbar")).toBeTruthy();
    // The default .diff-filename span must not render.
    const hdr = document.querySelector(".diff-filename") as HTMLElement | null;
    expect(hdr).toBeNull();
  });
});

/**
 * When `placeholder` is passed, DiffEditor renders the message instead of
 * wiring a CodeMirror MergeView. Guards against the binary-blob diff
 * branch shipping a garbled editor view.
 */
import { describe, it, expect } from "vitest";
import { render } from "@testing-library/svelte";
import DiffEditor from "$lib/components/editor/DiffEditor.svelte";

describe("DiffEditor with placeholder", () => {
  it("renders the placeholder text verbatim", () => {
    const { getByText, container } = render(DiffEditor, {
      oldContent: "old",
      newContent: "new",
      placeholder: "Binary file",
    });
    expect(getByText("Binary file")).toBeTruthy();
    // The merge view container must not mount when a placeholder is set.
    expect(container.querySelector(".cm-mergeView")).toBeNull();
  });

  it("renders the normal editor when placeholder is absent", () => {
    const { container } = render(DiffEditor, {
      oldContent: "old\n", newContent: "new\n",
    });
    expect(container.querySelector(".diff-editor")).toBeTruthy();
  });
});

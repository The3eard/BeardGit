/**
 * Unit tests for the `editorPrefs` store.
 *
 * Covers the three behaviours `EditorSettings.svelte` actually depends
 * on:
 *  1. `loadEditorPrefs` hydrates the store from `getEditorPreferences`.
 *  2. `updateEditorPrefs` patches optimistically, persists via
 *     `setEditorPreferences`, and leaves the store on the new value.
 *  3. `updateEditorPrefs` reverts the store and rethrows when the
 *     persistence call rejects — so the calling control can re-sync
 *     its checkbox / select state.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import type { EditorPreferences } from "$lib/types";

const mocks = vi.hoisted(() => ({
  getEditorPreferences: vi.fn(),
  setEditorPreferences: vi.fn(),
}));

vi.mock("$lib/api/tauri", () => ({
  getEditorPreferences: mocks.getEditorPreferences,
  setEditorPreferences: mocks.setEditorPreferences,
}));

import {
  editorPrefs,
  loadEditorPrefs,
  updateEditorPrefs,
} from "../editorPrefs";

const BASE: EditorPreferences = {
  autocomplete: true,
  close_brackets: true,
  bracket_matching: true,
  highlight_active_line: true,
  highlight_selection_matches: true,
  fold_gutter: true,
  indent_on_input: true,
  line_wrapping: true,
  rectangular_selection: false,
  crosshair_cursor: false,
  indent_guides: false,
  snippets: true,
  keyword_completion: true,
  json_lint: true,
  color_picker: true,
  tab_size: 2,
  indent_with_tabs: false,
  respect_gitignore_in_tree: false,
  large_file_warning_kb: 256,
};

beforeEach(() => {
  mocks.getEditorPreferences.mockReset();
  mocks.setEditorPreferences.mockReset();
  editorPrefs.set(null);
});

afterEach(() => {
  editorPrefs.set(null);
});

describe("editorPrefs store", () => {
  it("loadEditorPrefs hydrates the store from the IPC", async () => {
    mocks.getEditorPreferences.mockResolvedValue(BASE);
    await loadEditorPrefs();
    expect(get(editorPrefs)).toEqual(BASE);
  });

  it("loadEditorPrefs leaves the store null when IPC rejects", async () => {
    mocks.getEditorPreferences.mockRejectedValue(new Error("ipc down"));
    await loadEditorPrefs();
    expect(get(editorPrefs)).toBeNull();
  });

  it("updateEditorPrefs patches optimistically and persists", async () => {
    editorPrefs.set(BASE);
    mocks.setEditorPreferences.mockResolvedValue(undefined);
    await updateEditorPrefs({ tab_size: 4, indent_with_tabs: true });
    expect(mocks.setEditorPreferences).toHaveBeenCalledWith({
      ...BASE,
      tab_size: 4,
      indent_with_tabs: true,
    });
    expect(get(editorPrefs)).toEqual({
      ...BASE,
      tab_size: 4,
      indent_with_tabs: true,
    });
  });

  it("updateEditorPrefs reverts the store and rethrows on persist failure", async () => {
    editorPrefs.set(BASE);
    mocks.setEditorPreferences.mockRejectedValue(new Error("disk full"));
    await expect(
      updateEditorPrefs({ autocomplete: false }),
    ).rejects.toThrow("disk full");
    // Reverted to the original value.
    expect(get(editorPrefs)).toEqual(BASE);
  });

  it("updateEditorPrefs throws when the store hasn't been hydrated", async () => {
    await expect(updateEditorPrefs({ tab_size: 4 })).rejects.toThrow(
      /not loaded/i,
    );
    expect(mocks.setEditorPreferences).not.toHaveBeenCalled();
  });
});

/**
 * Editor-preferences IPC wrappers — verify each TS function maps to the
 * expected Tauri command name and payload shape.
 *
 * The wrappers themselves are thin (`invoke(name, args)`); the
 * load-bearing concerns are:
 *  - the snake_case command name on the wire,
 *  - the `prefs` argument key (Tauri auto-converts to snake_case for the
 *    Rust handler, but the JS-side wrapper key matters),
 *  - that the wrapper preserves the invoke return value untouched.
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import type { EditorPreferences } from "$lib/types";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

import { getEditorPreferences, setEditorPreferences } from "../tauri";

const SAMPLE: EditorPreferences = {
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
  tab_size: 2,
  indent_with_tabs: false,
  respect_gitignore_in_tree: false,
  large_file_warning_kb: 256,
};

beforeEach(() => {
  mocks.invoke.mockReset();
});

describe("editor-preferences wrappers", () => {
  it("getEditorPreferences invokes 'get_editor_preferences' with no args", async () => {
    mocks.invoke.mockResolvedValue(SAMPLE);
    const out = await getEditorPreferences();
    expect(mocks.invoke).toHaveBeenCalledWith("get_editor_preferences");
    expect(out).toEqual(SAMPLE);
  });

  it("setEditorPreferences invokes 'set_editor_preferences' with { prefs }", async () => {
    mocks.invoke.mockResolvedValue(undefined);
    const next: EditorPreferences = { ...SAMPLE, tab_size: 4, indent_with_tabs: true };
    await setEditorPreferences(next);
    expect(mocks.invoke).toHaveBeenCalledWith("set_editor_preferences", {
      prefs: next,
    });
  });
});

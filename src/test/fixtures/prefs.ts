/**
 * Factories for user-preference fixtures: SidebarNavLayout,
 * EditorPreferences. Mirror the Rust defaults so the bootstrap mocks
 * don't trigger unexpected layout/editor behaviour in tests.
 */

import type {
  EditorPreferences,
  SidebarNavLayout,
} from "../../lib/types";

export function makeSidebarNavLayout(
  overrides: Partial<SidebarNavLayout> = {},
): SidebarNavLayout {
  return {
    order: [],
    hidden: [],
    ...overrides,
  };
}

export function makeEditorPreferences(
  overrides: Partial<EditorPreferences> = {},
): EditorPreferences {
  return {
    autocomplete: true,
    close_brackets: true,
    bracket_matching: true,
    highlight_active_line: true,
    highlight_selection_matches: true,
    fold_gutter: true,
    indent_on_input: true,
    line_wrapping: false,
    rectangular_selection: false,
    crosshair_cursor: false,
    indent_guides: false,
    snippets: true,
    keyword_completion: true,
    json_lint: true,
    color_picker: true,
    tab_size: 2,
    indent_with_tabs: false,
    respect_gitignore_in_tree: true,
    large_file_warning_kb: 256,
    ...overrides,
  };
}

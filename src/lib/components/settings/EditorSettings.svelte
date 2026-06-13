<!--
  EditorSettings.svelte — Settings → Editor category.

  Owns the persisted preferences for the in-app file editor (PR3 will
  consume them to gate CodeMirror extensions). Two grouped sections —
  "Extensions" (10 boolean toggles for CodeMirror plugins) and
  "Behavior" (tab size, indent kind, gitignore filtering, large-file
  warning threshold) — both rendered inside a single `Card` so the
  category header isn't duplicated above each section.

  Persistence model: every control is bound to `$editorPrefs?.<field>`
  and on change calls `updateEditorPrefs({ <field>: value })`. The store
  patches optimistically and persists via the `set_editor_preferences`
  IPC; on persist failure it reverts the store and rethrows so the
  control re-syncs to the (reverted) value.

  All controls are disabled until the store hydrates (`$editorPrefs ===
  null`) — this avoids painting a default-looking state for a fraction
  of a second on cold start before the persisted values arrive.
-->
<script module lang="ts">
  import type { SettingDescriptor } from "./settings-index";

  /**
   * Global-search descriptors for the settings exposed by this
   * category. Strings are duplicated (not pulled from Paraglide)
   * because `settingsIndex` runs at module scope, before Paraglide's
   * current-locale getters are bound. Search is English-only for v1.
   */
  export const settingsIndex: SettingDescriptor[] = [
    {
      id: "editor.extensions",
      label: "Editor extensions",
      description:
        "Toggle CodeMirror plugins used by the in-app file editor (autocomplete, bracket matching, code folding, line wrapping, etc.).",
      category: "editor",
      anchor: "extensions",
    },
    {
      id: "editor.tab-size",
      label: "Tab size",
      description: "Visual width of an indent in the in-app editor.",
      category: "editor",
      anchor: "tab-size",
    },
    {
      id: "editor.indent-with-tabs",
      label: "Indent with tabs",
      description:
        "When enabled, the in-app editor inserts tab characters for indentation; otherwise spaces.",
      category: "editor",
      anchor: "indent-with-tabs",
    },
    {
      id: "editor.gitignore-filter",
      label: "Respect .gitignore in file tree",
      description:
        "Off by default — gitignored files stay visible so you can edit untracked / build-output files. Turn on to hide them.",
      category: "editor",
      anchor: "gitignore-filter",
    },
    {
      id: "editor.large-file-warning",
      label: "Warn when opening large files",
      description:
        "Threshold (in KB) above which the editor warns before opening a file.",
      category: "editor",
      anchor: "large-file-warning",
    },
    {
      id: "editor.behavior",
      label: "Editor behavior",
      description:
        "Indentation, file-tree filtering, and large-file warnings for the in-app editor.",
      category: "editor",
      anchor: "behavior",
    },
    {
      id: "editor.smart",
      label: "Smart editing",
      description:
        "Per-language helpers — code snippets, keyword completion, JSON lint, inline color picker.",
      category: "editor",
      anchor: "smart",
    },
    {
      id: "editor.indent-guides",
      label: "Indent guides",
      description: "Vertical lines marking indentation depth.",
      category: "editor",
      anchor: "extensions",
    },
  ];
</script>

<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import { Card, SettingSection, FormRow, Switch } from "$lib/components/ui";
  import { editorPrefs, updateEditorPrefs } from "$lib/stores/editorPrefs";
  import type { EditorPreferences } from "$lib/types";

  const TAB_SIZE_OPTIONS = [2, 4, 8] as const;

  /** True while the persisted preferences are still being loaded. */
  const loading = $derived($editorPrefs === null);

  type BoolKey = {
    [K in keyof EditorPreferences]: EditorPreferences[K] extends boolean
      ? K
      : never;
  }[keyof EditorPreferences];

  /**
   * Generic checkbox handler — patches a single boolean field. On
   * persist failure the store reverts itself, so we only need to flip
   * the input back to match.
   */
  async function handleToggle(field: BoolKey, event: Event) {
    const input = event.target as HTMLInputElement;
    const value = input.checked;
    try {
      await updateEditorPrefs({ [field]: value } as Partial<EditorPreferences>);
    } catch {
      input.checked = !value;
    }
  }

  async function handleTabSizeChange(event: Event) {
    const select = event.target as HTMLSelectElement;
    const next = Number.parseInt(select.value, 10);
    const previous = $editorPrefs?.tab_size ?? 2;
    try {
      await updateEditorPrefs({ tab_size: next });
    } catch {
      select.value = String(previous);
    }
  }

  async function handleLargeFileWarningChange(event: Event) {
    const input = event.target as HTMLInputElement;
    // Coerce to integer KB; the backend clamps to 1..=2048 so we don't
    // need to validate on the FE beyond rejecting NaN.
    const raw = Number.parseInt(input.value, 10);
    if (!Number.isFinite(raw)) {
      input.value = String($editorPrefs?.large_file_warning_kb ?? 256);
      return;
    }
    const previous = $editorPrefs?.large_file_warning_kb ?? 256;
    try {
      await updateEditorPrefs({ large_file_warning_kb: raw });
      // Re-sync the input to the (clamped) stored value so users see
      // the canonical number even if they typed an out-of-range one.
      const stored = $editorPrefs?.large_file_warning_kb ?? raw;
      input.value = String(stored);
    } catch {
      input.value = String(previous);
    }
  }
</script>

<Card
  title={m.settings_category_editor()}
  description={m.settings_cat_editor_description()}
>
  <div data-setting-anchor="extensions" id="extensions">
    <SettingSection
      title={m.settings_editor_extensions()}
      description={m.settings_editor_extensions_description()}
    >
      <FormRow
        label={m.settings_editor_autocomplete()}
        for="editor-autocomplete-toggle"
      >
        <Switch
          id="editor-autocomplete-toggle"
          testid="editor-autocomplete-toggle"
          checked={$editorPrefs?.autocomplete ?? true}
          disabled={loading}
          onchange={(e) => handleToggle("autocomplete", e)}
        />
      </FormRow>

      <FormRow
        label={m.settings_editor_close_brackets()}
        for="editor-close-brackets-toggle"
      >
        <Switch
          id="editor-close-brackets-toggle"
          testid="editor-close-brackets-toggle"
          checked={$editorPrefs?.close_brackets ?? true}
          disabled={loading}
          onchange={(e) => handleToggle("close_brackets", e)}
        />
      </FormRow>

      <FormRow
        label={m.settings_editor_bracket_matching()}
        for="editor-bracket-matching-toggle"
      >
        <Switch
          id="editor-bracket-matching-toggle"
          checked={$editorPrefs?.bracket_matching ?? true}
          disabled={loading}
          onchange={(e) => handleToggle("bracket_matching", e)}
        />
      </FormRow>

      <FormRow
        label={m.settings_editor_highlight_active_line()}
        for="editor-highlight-active-line-toggle"
      >
        <Switch
          id="editor-highlight-active-line-toggle"
          checked={$editorPrefs?.highlight_active_line ?? true}
          disabled={loading}
          onchange={(e) => handleToggle("highlight_active_line", e)}
        />
      </FormRow>

      <FormRow
        label={m.settings_editor_highlight_selection_matches()}
        for="editor-highlight-selection-matches-toggle"
      >
        <Switch
          id="editor-highlight-selection-matches-toggle"
          checked={$editorPrefs?.highlight_selection_matches ?? true}
          disabled={loading}
          onchange={(e) => handleToggle("highlight_selection_matches", e)}
        />
      </FormRow>

      <FormRow
        label={m.settings_editor_fold_gutter()}
        for="editor-fold-gutter-toggle"
      >
        <Switch
          id="editor-fold-gutter-toggle"
          checked={$editorPrefs?.fold_gutter ?? true}
          disabled={loading}
          onchange={(e) => handleToggle("fold_gutter", e)}
        />
      </FormRow>

      <FormRow
        label={m.settings_editor_indent_on_input()}
        for="editor-indent-on-input-toggle"
      >
        <Switch
          id="editor-indent-on-input-toggle"
          checked={$editorPrefs?.indent_on_input ?? true}
          disabled={loading}
          onchange={(e) => handleToggle("indent_on_input", e)}
        />
      </FormRow>

      <FormRow
        label={m.settings_editor_line_wrapping()}
        for="editor-line-wrapping-toggle"
      >
        <Switch
          id="editor-line-wrapping-toggle"
          checked={$editorPrefs?.line_wrapping ?? true}
          disabled={loading}
          onchange={(e) => handleToggle("line_wrapping", e)}
        />
      </FormRow>

      <FormRow
        label={m.settings_editor_rectangular_selection()}
        for="editor-rectangular-selection-toggle"
      >
        <Switch
          id="editor-rectangular-selection-toggle"
          checked={$editorPrefs?.rectangular_selection ?? false}
          disabled={loading}
          onchange={(e) => handleToggle("rectangular_selection", e)}
        />
      </FormRow>

      <FormRow
        label={m.settings_editor_crosshair_cursor()}
        for="editor-crosshair-cursor-toggle"
      >
        <Switch
          id="editor-crosshair-cursor-toggle"
          checked={$editorPrefs?.crosshair_cursor ?? false}
          disabled={loading}
          onchange={(e) => handleToggle("crosshair_cursor", e)}
        />
      </FormRow>

      <FormRow
        label={m.settings_editor_indent_guides()}
        for="editor-indent-guides-toggle"
        helperText={m.settings_editor_indent_guides_description()}
      >
        <Switch
          id="editor-indent-guides-toggle"
          testid="editor-indent-guides-toggle"
          checked={$editorPrefs?.indent_guides ?? false}
          disabled={loading}
          onchange={(e) => handleToggle("indent_guides", e)}
        />
      </FormRow>
    </SettingSection>
  </div>

  <div data-setting-anchor="smart" id="smart">
    <SettingSection
      title={m.settings_editor_smart()}
      description={m.settings_editor_smart_description()}
    >
      <FormRow
        label={m.settings_editor_snippets()}
        for="editor-snippets-toggle"
        helperText={m.settings_editor_snippets_description()}
      >
        <Switch
          id="editor-snippets-toggle"
          testid="editor-snippets-toggle"
          checked={$editorPrefs?.snippets ?? true}
          disabled={loading}
          onchange={(e) => handleToggle("snippets", e)}
        />
      </FormRow>

      <FormRow
        label={m.settings_editor_keyword_completion()}
        for="editor-keyword-completion-toggle"
        helperText={m.settings_editor_keyword_completion_description()}
      >
        <Switch
          id="editor-keyword-completion-toggle"
          testid="editor-keyword-completion-toggle"
          checked={$editorPrefs?.keyword_completion ?? true}
          disabled={loading}
          onchange={(e) => handleToggle("keyword_completion", e)}
        />
      </FormRow>

      <FormRow
        label={m.settings_editor_json_lint()}
        for="editor-json-lint-toggle"
        helperText={m.settings_editor_json_lint_description()}
      >
        <Switch
          id="editor-json-lint-toggle"
          testid="editor-json-lint-toggle"
          checked={$editorPrefs?.json_lint ?? true}
          disabled={loading}
          onchange={(e) => handleToggle("json_lint", e)}
        />
      </FormRow>

      <FormRow
        label={m.settings_editor_color_picker()}
        for="editor-color-picker-toggle"
        helperText={m.settings_editor_color_picker_description()}
      >
        <Switch
          id="editor-color-picker-toggle"
          testid="editor-color-picker-toggle"
          checked={$editorPrefs?.color_picker ?? true}
          disabled={loading}
          onchange={(e) => handleToggle("color_picker", e)}
        />
      </FormRow>
    </SettingSection>
  </div>

  <div data-setting-anchor="behavior" id="behavior">
    <SettingSection
      title={m.settings_editor_behavior()}
      description={m.settings_editor_behavior_description()}
    >
      <div data-setting-anchor="tab-size" id="tab-size">
        <FormRow
          label={m.settings_editor_tab_size()}
          for="editor-tab-size-select"
        >
          <select
            id="editor-tab-size-select"
            class="bg-select"
            data-testid="editor-tab-size-select"
            value={$editorPrefs?.tab_size ?? 2}
            disabled={loading}
            onchange={handleTabSizeChange}
          >
            {#each TAB_SIZE_OPTIONS as opt (opt)}
              <option value={opt}>{opt}</option>
            {/each}
          </select>
        </FormRow>
      </div>

      <div data-setting-anchor="indent-with-tabs" id="indent-with-tabs">
        <FormRow
          label={m.settings_editor_indent_with_tabs()}
          for="editor-indent-with-tabs-toggle"
          helperText={m.settings_editor_indent_with_tabs_description()}
        >
          <Switch
            id="editor-indent-with-tabs-toggle"
            checked={$editorPrefs?.indent_with_tabs ?? false}
            disabled={loading}
            onchange={(e) => handleToggle("indent_with_tabs", e)}
          />
        </FormRow>
      </div>

      <div data-setting-anchor="gitignore-filter" id="gitignore-filter">
        <FormRow
          label={m.settings_editor_respect_gitignore()}
          for="editor-respect-gitignore-toggle"
          helperText={m.settings_editor_respect_gitignore_description()}
        >
          <Switch
            id="editor-respect-gitignore-toggle"
            checked={$editorPrefs?.respect_gitignore_in_tree ?? false}
            disabled={loading}
            onchange={(e) => handleToggle("respect_gitignore_in_tree", e)}
          />
        </FormRow>
      </div>

      <div data-setting-anchor="large-file-warning" id="large-file-warning">
        <FormRow
          label={m.settings_editor_large_file_warning()}
          for="editor-large-file-warning-input"
        >
          <div class="bg-numeric-with-unit">
            <input
              id="editor-large-file-warning-input"
              type="number"
              class="bg-number-input"
              data-testid="editor-large-file-warning-input"
              min="1"
              max="2048"
              step="32"
              value={$editorPrefs?.large_file_warning_kb ?? 256}
              disabled={loading}
              onchange={handleLargeFileWarningChange}
            />
            <span class="bg-numeric-unit">
              {m.settings_editor_large_file_warning_unit()}
            </span>
          </div>
        </FormRow>
      </div>
    </SettingSection>
  </div>
</Card>

<style>
  .bg-select {
    padding: 5px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    outline: none;
    cursor: pointer;
    min-width: 96px;
    font-family: inherit;
  }

  .bg-select:focus {
    border-color: var(--accent-primary);
  }

  .bg-numeric-with-unit {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .bg-number-input {
    width: 96px;
    padding: 5px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    font-family: inherit;
    outline: none;
  }

  .bg-number-input:focus {
    border-color: var(--accent-primary);
  }

  .bg-numeric-unit {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
  }
</style>

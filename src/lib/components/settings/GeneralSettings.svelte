<!--
  GeneralSettings.svelte — the first row of the MT-5 IA.

  Hosts the "touches everything" toggles that don't belong in a
  more specific category: language, theme, theme-follows-system,
  UI scale. The actual markup + state logic lives in the extracted
  `LookAndFeelSection.svelte` so the `<Card>` here owns the single
  visible heading — the previous inline implementation rendered the
  "Look & feel" title twice (once on the Card, once on the inner
  `SettingSection`), which this split removes.
-->
<script module lang="ts">
  import type { SettingDescriptor } from "./settings-index";

  /**
   * Global-search descriptors for the settings exposed by this
   * category. The label/description strings are duplicated here
   * (not fetched from Paraglide) because `settingsIndex` runs at
   * module scope — before Paraglide's current-locale getters are
   * bound. Search is English-only for v1; i18n of search hits is
   * tracked in a follow-up.
   */
  export const settingsIndex: SettingDescriptor[] = [
    {
      id: "general.language",
      label: "Language",
      description: "Change the interface language. Requires a reload.",
      category: "general",
      anchor: "language",
    },
    {
      id: "general.theme-auto",
      label: "Follow system theme",
      description:
        "When enabled, BeardGit switches theme based on the OS dark-mode setting.",
      category: "general",
      anchor: "theme-auto",
    },
    {
      id: "general.theme",
      label: "Theme",
      description: "Pick a built-in colour theme.",
      category: "general",
      anchor: "theme",
    },
    {
      id: "general.ui-scale",
      label: "UI scale",
      description: "Zoom the entire interface by a fixed percentage.",
      category: "general",
      anchor: "ui-scale",
    },
    {
      id: "general.diff-show-whitespace",
      label: "Show whitespace in diffs",
      description:
        "Render spaces and tabs as glyphs (· / →) so whitespace-only changes are visible in the side-by-side diff viewer.",
      category: "general",
      anchor: "diff-show-whitespace",
    },
    {
      id: "general.diff-line-wrapping",
      label: "Wrap long lines in diffs",
      description:
        "Soft-wrap long lines in every diff view (commit, PR/MR, stash, tag, and the staging panel in Changes) instead of clipping them. Independent from the editor's own line-wrapping preference.",
      category: "general",
      anchor: "diff-line-wrapping",
    },
  ];
</script>

<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import { Card, FormRow } from "$lib/components/ui";
  import LookAndFeelSection from "./LookAndFeelSection.svelte";
  import {
    diffShowWhitespace,
    updateDiffShowWhitespace,
    diffLineWrapping,
    updateDiffLineWrapping,
  } from "$lib/stores/diffSettings";

  async function handleToggleDiffWhitespace(event: Event) {
    const input = event.target as HTMLInputElement;
    try {
      await updateDiffShowWhitespace(input.checked);
    } catch {
      // Persistence failed — re-sync the checkbox to the (reverted)
      // store state. The store reverts inside `updateDiffShowWhitespace`.
      input.checked = !input.checked;
    }
  }

  async function handleToggleDiffLineWrapping(event: Event) {
    const input = event.target as HTMLInputElement;
    try {
      await updateDiffLineWrapping(input.checked);
    } catch {
      input.checked = !input.checked;
    }
  }
</script>

<Card
  title={m.settings_general_theme_section_title()}
  description={m.settings_general_theme_section_description()}
>
  <LookAndFeelSection />
</Card>

<Card
  title={m.settings_general_diff_section_title()}
  description={m.settings_general_diff_section_description()}
>
  <div class="diff-settings-body">
    <div data-setting-anchor="diff-show-whitespace">
      <FormRow
        label={m.settings_general_diff_show_whitespace_label()}
        for="diff-show-whitespace-toggle"
        helperText={m.settings_general_diff_show_whitespace_hint()}
      >
        <input
          id="diff-show-whitespace-toggle"
          type="checkbox"
          class="bg-checkbox"
          data-testid="diff-show-whitespace-toggle"
          checked={$diffShowWhitespace}
          onchange={handleToggleDiffWhitespace}
        />
      </FormRow>
    </div>
    <div data-setting-anchor="diff-line-wrapping">
      <FormRow
        label={m.settings_general_diff_line_wrapping_label()}
        for="diff-line-wrapping-toggle"
        helperText={m.settings_general_diff_line_wrapping_hint()}
      >
        <input
          id="diff-line-wrapping-toggle"
          type="checkbox"
          class="bg-checkbox"
          data-testid="diff-line-wrapping-toggle"
          checked={$diffLineWrapping}
          onchange={handleToggleDiffLineWrapping}
        />
      </FormRow>
    </div>
  </div>
</Card>

<style>
  /* The parent <Card> owns the single visible "Diff display" heading;
     this wrapper only keeps the vertical rhythm of the removed inner
     SettingSection. */
  .diff-settings-body {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .bg-checkbox {
    accent-color: var(--accent-primary);
    width: 16px;
    height: 16px;
    cursor: pointer;
  }
</style>

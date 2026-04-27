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
  ];
</script>

<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import { Card } from "$lib/components/ui";
  import LookAndFeelSection from "./LookAndFeelSection.svelte";
</script>

<Card
  title={m.settings_general_theme_section_title()}
  description={m.settings_general_theme_section_description()}
>
  <LookAndFeelSection />
</Card>

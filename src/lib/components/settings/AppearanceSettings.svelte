<!--
  AppearanceSettings.svelte — visual-polish category.

  The old pre-MT-5 version of this file carried theme + language +
  UI-scale. Those have moved to `GeneralSettings` (Task 4.1) on the
  theory that they touch every surface of the app — Appearance is
  reserved for purely visual preferences (colors, density, fonts,
  graph styling) that the spec calls out as future work.

  We ship a clearly-labeled stub `SettingSection` so the category
  isn't empty-empty but also doesn't invent settings just to fill
  space.
-->
<script module lang="ts">
  import type { SettingDescriptor } from "./settings-index";

  /**
   * Empty for now — populated when the fine-grained visual
   * controls land. General-category settings (theme/language/scale)
   * are indexed by `GeneralSettings.svelte` instead.
   */
  export const settingsIndex: SettingDescriptor[] = [];
</script>

<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import { Card, SettingSection } from "$lib/components/ui";
</script>

<Card
  title={m.settings_appearance_section_title()}
  description={m.settings_appearance_section_description()}
>
  <SettingSection
    title={m.settings_appearance_section_title()}
    description={m.settings_appearance_section_description()}
  >
    <p class="appearance-stub">{m.settings_coming_soon()}</p>
  </SettingSection>
</Card>

<style>
  .appearance-stub {
    color: var(--text-secondary);
    font-size: 12px;
    margin: 0;
    line-height: 1.5;
  }
</style>

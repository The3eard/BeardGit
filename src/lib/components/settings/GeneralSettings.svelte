<!--
  GeneralSettings.svelte — the first row of the MT-5 IA.

  Houses the "touches everything" toggles that don't belong in a
  more specific category: language, theme, theme-follows-system,
  UI scale. Content migrated verbatim from the old
  `AppearanceSettings.svelte` (which is being narrowed to visual
  polish in Task 4.2).

  Uses the shared primitives (`Card`, `SettingSection`, `FormRow`)
  so there's zero inline card/button CSS left in the file.
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
  import { onMount } from "svelte";
  import { currentLocale, changeLocale } from "$lib/stores/locale";
  import {
    listThemes,
    getThemeAuto,
    setTheme,
    setThemeAuto,
    getUiScale,
    setUiScale,
  } from "$lib/api/tauri";
  import { activeTheme, applyUiScale } from "$lib/stores/theme";
  import type { ThemeMeta } from "$lib/types";
  import * as m from "$lib/paraglide/messages";
  import { Card, SettingSection, FormRow } from "$lib/components/ui";

  const languages = [
    { tag: "en-US", label: "English (US)" },
    { tag: "es-ES", label: "Español (ES)" },
  ];

  const scaleOptions = [80, 90, 100, 110, 125, 150];

  let themes = $state<ThemeMeta[]>([]);
  let themeAuto = $state(true);
  let selectedThemeId = $state("");
  let uiScale = $state(100);

  onMount(async () => {
    themes = await listThemes();
    themeAuto = await getThemeAuto();
    const current = $activeTheme;
    if (current) selectedThemeId = current.meta.id;
    uiScale = await getUiScale();
  });

  async function handleLanguageChange(event: Event) {
    const select = event.target as HTMLSelectElement;
    await changeLocale(select.value);
  }

  async function handleThemeChange(event: Event) {
    const select = event.target as HTMLSelectElement;
    selectedThemeId = select.value;
    if (themeAuto) {
      themeAuto = false;
      await setThemeAuto(false);
    }
    await setTheme(select.value);
  }

  async function handleAutoToggle(event: Event) {
    const input = event.target as HTMLInputElement;
    themeAuto = input.checked;
    await setThemeAuto(themeAuto);
  }

  async function handleScaleChange(event: Event) {
    const select = event.target as HTMLSelectElement;
    const scale = parseInt(select.value, 10);
    uiScale = scale;
    await applyUiScale(scale);
    await setUiScale(scale);
  }
</script>

<Card
  title={m.settings_general_theme_section_title()}
  description={m.settings_general_theme_section_description()}
>
  <SettingSection title={m.settings_general_theme_section_title()}>
    <div data-setting-anchor="language">
      <FormRow label={m.settings_language()} for="language-select">
        <select
          id="language-select"
          class="bg-select"
          value={$currentLocale}
          onchange={handleLanguageChange}
        >
          {#each languages as lang (lang.tag)}
            <option value={lang.tag}>{lang.label}</option>
          {/each}
        </select>
      </FormRow>
    </div>

    <div data-setting-anchor="theme-auto">
      <FormRow label={m.settings_theme_auto()} for="theme-auto">
        <input
          id="theme-auto"
          type="checkbox"
          class="bg-checkbox"
          checked={themeAuto}
          onchange={handleAutoToggle}
        />
      </FormRow>
    </div>

    <div data-setting-anchor="theme">
      <FormRow label={m.settings_theme()} for="theme-select">
        <select
          id="theme-select"
          class="bg-select"
          value={selectedThemeId}
          onchange={handleThemeChange}
        >
          {#each themes as theme (theme.id)}
            <option value={theme.id}>{theme.name}</option>
          {/each}
        </select>
      </FormRow>
    </div>

    <div data-setting-anchor="ui-scale">
      <FormRow label={m.settings_ui_scale()} for="scale-select">
        <select
          id="scale-select"
          class="bg-select"
          value={uiScale}
          onchange={handleScaleChange}
        >
          {#each scaleOptions as opt (opt)}
            <option value={opt}>{opt}%</option>
          {/each}
        </select>
      </FormRow>
    </div>
  </SettingSection>
</Card>

<style>
  .bg-select {
    padding: 5px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 12px;
    outline: none;
    cursor: pointer;
    min-width: 160px;
    font-family: inherit;
  }

  .bg-select:focus {
    border-color: var(--accent-blue);
  }

  .bg-checkbox {
    accent-color: var(--accent-blue);
    width: 16px;
    height: 16px;
    cursor: pointer;
  }
</style>

<!--
  LookAndFeelSection.svelte — extracted Look & Feel block.

  Owns the "touches everything" preferences: language, follow-system
  theme, theme selection, and UI scale. Lifted out of
  `GeneralSettings.svelte` so the parent category Card renders the
  single "Look & feel" heading — the inner `<SettingSection>` used to
  duplicate that title next to the Card header (spec problem 1), and
  centralising the logic here leaves the General component as a thin
  shell that can host additional rows later without shuffling state.

  Deliberately NOT wrapped in a `<Card>`: the parent owns the card
  chrome so we avoid the duplicated header. Each `FormRow` keeps its
  `data-setting-anchor` from the original markup so search deep-links
  resolve to the same elements.

  Search descriptors live on `GeneralSettings.svelte` (the category
  component) — keeping this file presentation-only so it can be reused
  elsewhere without dragging a settings-index coupling along.
-->
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
  import { FormRow, Switch } from "$lib/components/ui";

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

<div data-testid="look-and-feel-heading" class="look-and-feel-body">
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
      <Switch id="theme-auto" checked={themeAuto} onchange={handleAutoToggle} />
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
</div>

<style>
  /* The parent <Card> owns the single visible "Look & feel" heading;
     this wrapper only keeps the vertical rhythm the removed inner
     SettingSection used to provide. */
  .look-and-feel-body {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

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
    border-color: var(--accent-primary);
  }

</style>

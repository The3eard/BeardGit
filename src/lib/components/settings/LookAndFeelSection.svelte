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
    checkThemeContrast,
  } from "$lib/api/tauri";
  import { activeTheme, applyUiScale } from "$lib/stores/theme";
  import type { ThemeContrastReport, ThemeMeta } from "$lib/types";
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
  /* Non-blocking accessibility notice for the selected theme. Bundled
     themes always come back clean (a Rust test enforces that), so anything
     here is a theme the user wrote — and we report it rather than
     "correcting" colours they chose. */
  let contrast = $state<ThemeContrastReport | null>(null);

  onMount(async () => {
    themes = await listThemes();
    themeAuto = await getThemeAuto();
    uiScale = await getUiScale();
  });

  /* Tracks `activeTheme` rather than reading it once on mount. With
     follow-system-theme on, an OS dark/light flip swaps the theme through
     the `theme-changed` listener without going through `handleThemeChange`
     — so a one-shot read left both the selector and the contrast notice
     showing the previous theme. Same for `handleAutoToggle`, which can
     switch the theme backend-side the moment it is enabled. */
  $effect(() => {
    const current = $activeTheme;
    if (!current) return;
    selectedThemeId = current.meta.id;
    void refreshContrast(current.meta.id);
  });

  async function refreshContrast(themeId: string) {
    try {
      const report = await checkThemeContrast(themeId);
      // Unmeasurable tokens count as "worth telling the user about" too.
      // Gating on `warnings` alone meant a theme whose colours could not be
      // parsed rendered no notice at all — reported as clean precisely
      // because it had never been checked.
      const worthShowing =
        report.warnings.length > 0 || report.unaudited.length > 0;
      contrast = worthShowing ? report : null;
    } catch {
      // Advisory only — a failed audit must never block theme selection.
      contrast = null;
    }
  }

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
    // `$effect` above picks up the resulting `activeTheme` change and
    // re-audits; no explicit refresh needed here.
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

    {#if contrast}
      <div class="contrast-notice" data-testid="theme-contrast-notice">
        <p class="contrast-notice__lead">
          {m.settings_theme_contrast_lead()}
        </p>
        <ul class="contrast-notice__list">
          {#each contrast.warnings as warning (warning.token)}
            <li>
              <code>{warning.token}</code>
              {m.settings_theme_contrast_ratio({
                ratio: warning.ratio.toFixed(2),
                required: warning.required.toFixed(1),
              })}
            </li>
          {/each}
          {#each contrast.unaudited as token (token)}
            <li>
              <code>{token}</code>
              {m.settings_theme_contrast_unmeasurable()}
            </li>
          {/each}
        </ul>
      </div>
    {/if}
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
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    outline: none;
    cursor: pointer;
    min-width: 160px;
    font-family: inherit;
  }

  .bg-select:focus {
    border-color: var(--accent-primary);
  }

  /* Advisory, not an error: the theme still applies. Warning colours
     rather than danger, and no dismiss affordance — it disappears when the
     user picks a theme that passes. */
  .contrast-notice {
    margin-top: 8px;
    padding: 10px 12px;
    border: 1px solid var(--accent-orange);
    border-radius: 6px;
    background: var(--bg-secondary);
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    line-height: 1.5;
  }

  .contrast-notice__lead {
    margin: 0;
    color: var(--text-primary);
  }

  .contrast-notice__list {
    margin: 6px 0 0;
    padding-left: 18px;
  }

  .contrast-notice__list code {
    font-family: var(--font-mono);
    color: var(--text-primary);
  }
</style>

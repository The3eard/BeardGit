<script lang="ts">
  import { onMount } from "svelte";
  import { currentLocale, changeLocale } from "../../stores/locale";
  import { listThemes, getThemeAuto, setTheme, setThemeAuto, getUiScale, setUiScale } from "../../api/tauri";
  import { activeTheme, applyUiScale } from "../../stores/theme";
  import type { ThemeMeta } from "../../types";
  import * as m from "$lib/paraglide/messages";

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

  async function handleLanguageChange(e: Event) {
    const select = e.target as HTMLSelectElement;
    await changeLocale(select.value);
  }

  async function handleThemeChange(e: Event) {
    const select = e.target as HTMLSelectElement;
    selectedThemeId = select.value;
    if (themeAuto) {
      themeAuto = false;
      await setThemeAuto(false);
    }
    await setTheme(select.value);
  }

  async function handleAutoToggle() {
    themeAuto = !themeAuto;
    await setThemeAuto(themeAuto);
  }

  async function handleScaleChange(e: Event) {
    const select = e.target as HTMLSelectElement;
    const scale = parseInt(select.value, 10);
    uiScale = scale;
    applyUiScale(scale);
    await setUiScale(scale);
  }
</script>

<div class="appearance-card">
  <h2 class="card-title">{m.settings_appearance()}</h2>

  <div class="setting-row">
    <label for="language-select">{m.settings_language()}</label>
    <select id="language-select" class="setting-select" value={$currentLocale} onchange={handleLanguageChange}>
      {#each languages as lang}
        <option value={lang.tag}>{lang.label}</option>
      {/each}
    </select>
  </div>

  <div class="setting-row">
    <div class="setting-label-group">
      <label for="theme-auto">{m.settings_theme_auto()}</label>
    </div>
    <input id="theme-auto" type="checkbox" checked={themeAuto} onchange={handleAutoToggle} />
  </div>

  <div class="setting-row">
    <label for="theme-select">{m.settings_theme()}</label>
    <select id="theme-select" class="setting-select" value={selectedThemeId} onchange={handleThemeChange}>
      {#each themes as theme}
        <option value={theme.id}>{theme.name}</option>
      {/each}
    </select>
  </div>

  <div class="setting-row">
    <label for="scale-select">{m.settings_ui_scale()}</label>
    <select id="scale-select" class="setting-select" value={uiScale} onchange={handleScaleChange}>
      {#each scaleOptions as opt}
        <option value={opt}>{opt}%</option>
      {/each}
    </select>
  </div>
</div>

<style>
  .appearance-card { max-width: 480px; margin: 48px auto; padding: 32px; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 8px; }
  .card-title { font-size: 18px; font-weight: 600; color: var(--text-primary); margin-bottom: 24px; }

  .setting-row { display: flex; align-items: center; justify-content: space-between; padding: 12px 0; border-bottom: 1px solid var(--border); }
  .setting-row:last-child { border-bottom: none; }
  .setting-label-group { display: flex; flex-direction: column; gap: 2px; }

  label { font-size: 13px; font-weight: 500; color: var(--text-primary); }

  .setting-select { padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border); border-radius: 6px; color: var(--text-primary); font-size: 13px; outline: none; cursor: pointer; min-width: 160px; }
  .setting-select:focus { border-color: var(--accent-blue); }
  .setting-select:disabled { opacity: 0.5; cursor: not-allowed; }

  input[type="checkbox"] { accent-color: var(--accent-blue); width: 16px; height: 16px; cursor: pointer; }
</style>

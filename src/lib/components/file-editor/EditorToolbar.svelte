<!--
  EditorToolbar.svelte — action row above the editor.

  Owns the Save / Save+Stage button and the external-change reload
  banner. Save is shift-click aware: a plain click writes the file, a
  Shift+Click writes and stages it. Disabled when no tab is active or
  the buffer hasn't diverged from disk.

  Mod+S / Mod+Shift+S are wired in `EditorPane.svelte` via the
  CodeMirror keymap so the toolbar mirror is purely a visual
  affordance — both routes resolve to the same `saveActive` store
  helper.
-->
<script lang="ts">
  import { Button } from "$lib/components/ui";
  import {
    activeTabPath,
    clearExternalChange,
    reloadActive,
    saveActive,
    tabs,
  } from "$lib/stores/fileEditor";
  import * as m from "$lib/paraglide/messages";

  /** Active tab snapshot — `null` when no buffer is open. */
  let active = $derived(
    $tabs.find((t) => t.path === $activeTabPath) ?? null,
  );

  let canSave = $derived(!!active && active.status === "ok" && active.dirty);

  /**
   * Track whether the Shift key is currently held so the Save button can
   * morph into "Save and stage" in real time. Listens at window scope
   * because focus may be inside the CodeMirror editor when Shift is
   * pressed; resets on `blur` so a tab/window switch with Shift held
   * doesn't leave the toggle stuck.
   */
  let shiftHeld = $state(false);
  $effect(() => {
    function onDown(e: KeyboardEvent) {
      if (e.key === "Shift") shiftHeld = true;
    }
    function onUp(e: KeyboardEvent) {
      if (e.key === "Shift") shiftHeld = false;
    }
    function onBlur() {
      shiftHeld = false;
    }
    window.addEventListener("keydown", onDown);
    window.addEventListener("keyup", onUp);
    window.addEventListener("blur", onBlur);
    return () => {
      window.removeEventListener("keydown", onDown);
      window.removeEventListener("keyup", onUp);
      window.removeEventListener("blur", onBlur);
    };
  });

  /** Computed label / tooltip — flips to the stage variant while Shift is held. */
  let stageMode = $derived(shiftHeld && canSave);
  let saveLabel = $derived(stageMode ? m.editor_save_and_stage() : m.editor_save());
  let saveTooltip = $derived(
    stageMode ? m.editor_save_and_stage_tooltip() : m.editor_save_tooltip(),
  );

  function handleSaveClick(event: MouseEvent) {
    if (!canSave) return;
    void saveActive({ stage: event.shiftKey });
  }

  function reload() {
    void reloadActive();
  }

  function keepMine() {
    if (active) clearExternalChange(active.path);
  }
</script>

<div class="editor-toolbar">
  {#if active && active.externalChange}
    <div class="external-banner" role="status">
      <span class="banner-msg">
        <strong>{m.editor_external_change_title()}</strong>
        — {m.editor_external_change_body({ name: active.name })}
      </span>
      <span class="banner-actions">
        <Button variant="primary" size="xs" onclick={reload}>
          {m.editor_external_change_reload()}
        </Button>
        <Button variant="neutral" size="xs" onclick={keepMine}>
          {m.editor_external_change_keep()}
        </Button>
      </span>
    </div>
  {/if}
  <div class="toolbar-actions">
    <Button
      variant="primary"
      size="sm"
      icon={stageMode ? "" : ""}
      disabled={!canSave}
      description={saveTooltip}
      onclick={handleSaveClick}
    >
      {saveLabel}
    </Button>
  </div>
</div>

<style>
  .editor-toolbar {
    display: flex;
    flex-direction: column;
    border-bottom: 1px solid var(--border);
    background: var(--bg-toolbar);
    flex-shrink: 0;
  }
  .external-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 6px 12px;
    background: var(--overlay-accent-orange);
    color: var(--text-primary);
    border-bottom: 1px solid var(--border);
    font-size: 12px;
  }
  .banner-msg {
    flex: 1;
    min-width: 0;
  }
  .banner-actions {
    display: inline-flex;
    gap: 6px;
    flex-shrink: 0;
  }
  .toolbar-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    padding: 6px 10px;
  }
</style>

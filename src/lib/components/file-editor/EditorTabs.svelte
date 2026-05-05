<!--
  EditorTabs.svelte — horizontal scrollable tab strip above the editor.

  Each tab carries the file name, a dirty/external indicator pair, and a
  close button. Click selects the tab; middle-click closes it. Closing a
  dirty tab routes through the shared `ConfirmDialog` so the user can
  confirm discarding changes.
-->
<script lang="ts">
  import { IconButton } from "$lib/components/ui";
  import ConfirmDialog from "$lib/components/common/ConfirmDialog.svelte";
  import * as m from "$lib/paraglide/messages";
  import {
    activeTabPath,
    closeTab,
    setActiveTab,
    tabs,
    type EditorTab,
  } from "$lib/stores/fileEditor";

  let confirmTarget = $state<EditorTab | null>(null);

  /** Click handler — promotes the tab to active. */
  function onSelect(path: string) {
    setActiveTab(path);
  }

  /**
   * Close handler. Dirty tabs trigger the confirm dialog; clean ones
   * close immediately.
   */
  function requestClose(tab: EditorTab) {
    if (tab.dirty) {
      confirmTarget = tab;
    } else {
      void closeTab(tab.path);
    }
  }

  /** Auxiliary-button handler — middle-click acts as close. */
  function onAuxClick(e: MouseEvent, tab: EditorTab) {
    if (e.button === 1) {
      e.preventDefault();
      requestClose(tab);
    }
  }

  function confirmClose() {
    if (confirmTarget) {
      void closeTab(confirmTarget.path);
    }
    confirmTarget = null;
  }
  function cancelClose() {
    confirmTarget = null;
  }
</script>

{#if $tabs.length > 0}
  <div class="editor-tabs" role="tablist" aria-label="Open files">
    {#each $tabs as tab (tab.path)}
      {@const active = $activeTabPath === tab.path}
      <div
        class="tab"
        class:active
        role="tab"
        tabindex={active ? 0 : -1}
        aria-selected={active}
        title={tab.path}
        onmousedown={(e) => onAuxClick(e, tab)}
      >
        <button
          class="tab-label"
          type="button"
          onclick={() => onSelect(tab.path)}
        >
          <span class="name">{tab.name}</span>
          {#if tab.externalChange}
            <span
              class="indicator external"
              aria-label={m.editor_external_indicator()}
              title={m.editor_external_indicator()}
            >&#9888;</span>
          {/if}
          {#if tab.dirty}
            <span
              class="indicator dirty"
              aria-label={m.editor_dirty_indicator()}
              title={m.editor_dirty_indicator()}
            >&bull;</span>
          {/if}
        </button>
        <IconButton
          icon={""}
          description={m.editor_close_tab()}
          size="xs"
          tone="danger"
          onclick={() => requestClose(tab)}
        />
      </div>
    {/each}
  </div>
{/if}

{#if confirmTarget}
  <ConfirmDialog
    title={m.editor_close_tab_unsaved_title()}
    message={m.editor_close_tab_unsaved_body({ name: confirmTarget.name })}
    confirmLabel={m.editor_close_tab_unsaved_confirm()}
    destructive
    onConfirm={confirmClose}
    onCancel={cancelClose}
  />
{/if}

<style>
  .editor-tabs {
    display: flex;
    flex-shrink: 0;
    overflow-x: auto;
    overflow-y: hidden;
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--border);
    scrollbar-width: thin;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 6px 4px 10px;
    border-right: 1px solid var(--border);
    cursor: default;
    color: var(--text-secondary);
    background: transparent;
    position: relative;
    min-width: 90px;
    max-width: 220px;
  }
  .tab:hover {
    background: var(--overlay-hover);
    color: var(--text-primary);
  }
  .tab.active {
    background: var(--bg-primary);
    color: var(--text-primary);
  }
  .tab.active::after {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 2px;
    background: var(--accent-blue);
  }
  .tab-label {
    background: none;
    border: none;
    cursor: pointer;
    color: inherit;
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 4px 0;
    display: flex;
    align-items: center;
    gap: 4px;
    overflow: hidden;
    flex: 1;
    min-width: 0;
  }
  .tab-label .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .indicator {
    font-size: 12px;
    line-height: 1;
    flex-shrink: 0;
  }
  .indicator.dirty {
    color: var(--accent-blue);
  }
  .indicator.external {
    color: var(--accent-orange);
    font-size: 11px;
  }
</style>

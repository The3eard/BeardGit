<!--
  SectionFooter.svelte — sticky Save/Discard strip shown at the bottom
  of the RepoConfigPage right pane when the active section is dirty.

  Renders nothing when `dirty === false`.
-->
<script lang="ts">
  import { Button } from "$lib/components/ui";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    dirty: boolean;
    saving: boolean;
    onSave: () => void;
    onDiscard: () => void;
  }

  let { dirty, saving, onSave, onDiscard }: Props = $props();
</script>

{#if dirty}
  <div
    class="section-footer"
    data-testid="section-footer"
    role="region"
    aria-label="Unsaved changes"
  >
    <span class="label">{m.repo_config_unsaved_footer()}</span>
    <div class="actions">
      <Button onclick={onDiscard} testid="section-footer-discard">
        {m.repo_config_discard()}
      </Button>
      <Button
        variant="primary"
        loading={saving}
        onclick={onSave}
        testid="section-footer-save"
      >
        {m.repo_config_save()}
      </Button>
    </div>
  </div>
{/if}

<style>
  .section-footer {
    position: sticky;
    bottom: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 14px;
    margin-top: 12px;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border);
  }
  .label {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }
  .actions {
    display: flex;
    gap: 8px;
  }
</style>

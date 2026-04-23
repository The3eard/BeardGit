<!--
  NavigationGuardDialog.svelte — Save/Discard/Keep-editing prompt shown
  when the user tries to leave a dirty section in the Repo settings
  view. Works for any of: switching section, leaving the view, or
  switching the active project.
-->
<script lang="ts">
  import { Button, Dialog } from "$lib/components/ui";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    open: boolean;
    sectionLabel: string;
    saving: boolean;
    onSave: () => void;
    onDiscard: () => void;
    onCancel: () => void;
  }

  let { open, sectionLabel, saving, onSave, onDiscard, onCancel }: Props = $props();

  // The paraglide key is declared with a `{section}` placeholder (see
  // messages/en-US.json). Paraglide's runtime accepts an object of
  // named params — pass `sectionLabel` under `section`.
  let title = $derived(
    m.repo_config_guard_title({ section: sectionLabel }),
  );
</script>

<Dialog {open} onClose={onCancel} {title} size="sm">
  <p data-testid="repo-config-guard-body">
    {m.repo_config_guard_body()}
  </p>
  {#snippet footer()}
    <Button onclick={onCancel} testid="repo-config-guard-cancel">
      {m.repo_config_keep_editing()}
    </Button>
    <Button variant="danger" onclick={onDiscard} testid="repo-config-guard-discard">
      {m.repo_config_discard()}
    </Button>
    <Button
      variant="primary"
      loading={saving}
      onclick={onSave}
      testid="repo-config-guard-save"
    >
      {m.repo_config_save()}
    </Button>
  {/snippet}
</Dialog>

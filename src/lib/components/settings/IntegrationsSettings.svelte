<!--
  IntegrationsSettings.svelte — the Integrations settings page.

  Post-Spec-4-Phase-8 shape:
  - A top-level `<ConnectionHowTo />` dropdown (Phase 7).
  - A single dense `<Card>` titled "Connections" (Phase 8) holding
    four rows: GitHub, GitLab, gh CLI, glab CLI — one per integration,
    each rendered by the shared `<ConnectionRow>` primitive.

  The old PAT card + CLI card split is gone — provider tokens and
  CLI auth are now adjacent in a single compact grid so the page
  matches the rest of the post-IA settings surfaces.
-->
<script module lang="ts">
  import type { SettingDescriptor } from "./settings-index";

  export const settingsIndex: SettingDescriptor[] = [
    {
      id: "integrations.token",
      label: "Connections",
      description:
        "GitHub / GitLab accounts and gh / glab CLI authentication.",
      category: "integrations",
      anchor: "connections",
    },
  ];
</script>

<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import ConnectionHowTo from "./ConnectionHowTo.svelte";
  import ConnectionRow from "./ConnectionRow.svelte";
  import { Button, Card } from "$lib/components/ui";
  import { forgeEnabled } from "$lib/stores/provider";
  import { setCategory } from "$lib/stores/settingsRoute";
</script>

<div class="integrations-page">
  {#if !$forgeEnabled}
    <!-- The switch lives in General → Integrations next to the AI one;
         this page only points there. Not mounting the rows matters: each
         `ConnectionRow` checks its status on mount, and the CLI rows shell
         out to `gh` / `glab auth status`, which goes to the network. -->
    <Card
      title={m.settings_integrations_connections_section()}
      description={m.settings_integrations_connections_description()}
    >
      <div class="disabled-notice" data-testid="integrations-disabled">
        <p>{m.integrations_settings_disabled_notice()}</p>
        <Button variant="neutral" onclick={() => setCategory("general", "forge-enabled")}>
          {m.ai_settings_disabled_action()}
        </Button>
      </div>
    </Card>
  {:else}
    <ConnectionHowTo />

    <Card
      title={m.settings_integrations_connections_section()}
      description={m.settings_integrations_connections_description()}
    >
      <div class="connections-grid" data-setting-anchor="connections">
        <ConnectionRow kind="github" />
        <ConnectionRow kind="gitlab" />
        <ConnectionRow kind="gh" />
        <ConnectionRow kind="glab" />
      </div>
    </Card>
  {/if}
</div>

<style>
  .integrations-page {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .connections-grid {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .disabled-notice {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
  }
  .disabled-notice p {
    margin: 0;
  }
</style>

<!--
  IntegrationsSettings.svelte — the former "Connection" tab, now a
  top-level category in the MT-5 IA.

  Content migrated verbatim:
   - ConnectionHowTo — collapsible walkthrough for PAT vs CLI auth.
   - ProviderSetup — GitHub / GitLab token entry + remove.
   - CliAuthSection — gh / glab installed-and-authed status + auth
     button.

  Wrapped in shared `Card` + `SettingSection` so the category picks
  up the new IA spacing and heading conventions.
-->
<script module lang="ts">
  import type { SettingDescriptor } from "./settings-index";

  export const settingsIndex: SettingDescriptor[] = [
    {
      id: "integrations.token",
      label: "Personal access tokens",
      description:
        "Connect BeardGit to GitHub or GitLab with a Personal Access Token.",
      category: "integrations",
      anchor: "tokens",
    },
    {
      id: "integrations.cli",
      label: "CLI authentication",
      description: "Check gh and glab login state without leaving Settings.",
      category: "integrations",
      anchor: "cli",
    },
  ];
</script>

<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import ProviderSetup from "../auth/ProviderSetup.svelte";
  import CliAuthSection from "./CliAuthSection.svelte";
  import ConnectionHowTo from "./ConnectionHowTo.svelte";
  import { Card, SettingSection } from "$lib/components/ui";
</script>

<Card>
  <ConnectionHowTo />
</Card>

<Card
  title={m.settings_integrations_token_section()}
  description={m.settings_integrations_token_description()}
>
  <SettingSection title={m.settings_token_auth()}>
    <div data-setting-anchor="tokens">
      <ProviderSetup />
    </div>
  </SettingSection>
</Card>

<Card
  title={m.settings_integrations_cli_section()}
  description={m.settings_integrations_cli_description()}
>
  <SettingSection title={m.cli_auth_title()}>
    <div data-setting-anchor="cli">
      <CliAuthSection />
    </div>
  </SettingSection>
</Card>

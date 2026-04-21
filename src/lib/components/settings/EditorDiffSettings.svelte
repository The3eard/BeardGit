<!--
  EditorDiffSettings.svelte — new category.

  There are no editor/diff-specific settings today. Rather than
  invent settings to fill the section we ship a clearly-labeled
  stub `SettingSection`, matching the direction the spec gives for
  empty categories.
-->
<script module lang="ts">
  import type { SettingDescriptor } from "./settings-index";

  /** No settings yet — search index intentionally empty. */
  export const settingsIndex: SettingDescriptor[] = [];
</script>

<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import { Card, SettingSection } from "$lib/components/ui";
</script>

<Card
  title={m.settings_editor_stub_title()}
  description={m.settings_editor_stub_description()}
>
  <SettingSection
    title={m.settings_editor_stub_title()}
    description={m.settings_editor_stub_description()}
  >
    <p class="editor-stub">{m.settings_coming_soon()}</p>
  </SettingSection>
</Card>

<style>
  .editor-stub {
    color: var(--text-secondary);
    font-size: 12px;
    margin: 0;
    line-height: 1.5;
  }
</style>

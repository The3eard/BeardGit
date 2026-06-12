<!--
  StatusBar — the lean 5-slot app status strip.

  The bar replaces the former git-aware statusbar (branch count, ahead
  /behind, dirty count, provider name) with a strictly app-level set of
  signals. Repo-specific state already lives in the sidebar, branch
  chip, and graph header — duplicating it here was noise.

  Slot order (left → right):

      [ Tasks ] | [ Forge ] | [ AI ] | [ Network ]   (spacer)   [ Version ]

  Heights: 22 px total. 1 px 40%-opacity dividers between slots.

  Behaviour:
    - Clicking **Tasks** toggles the `tasksPopoverOpen` store; the
      popover itself is mounted in `+page.svelte`.
    - Clicking **Forge / AI / Version** deep-links to the corresponding
      Settings sub-section: the section key is written to
      `pendingSettingsSection` and then `activeViewStore` flips to
      `"settings"`. `SettingsPage.svelte` subscribes to the store and
      mirrors the value into its local active-section state.
    - **Network** is non-interactive and hides itself while online.
-->
<script lang="ts">
  import RepoSlot from "./statusbar/RepoSlot.svelte";
  import TasksSlot from "./statusbar/TasksSlot.svelte";
  import ForgeSlot from "./statusbar/ForgeSlot.svelte";
  import AiSlot from "./statusbar/AiSlot.svelte";
  import NetworkSlot from "./statusbar/NetworkSlot.svelte";
  import HelpSlot from "./statusbar/HelpSlot.svelte";
  import VersionSlot from "./statusbar/VersionSlot.svelte";
  import { toggleTasksPopover } from "$lib/stores/tasksPopover";
  import { activeViewStore, pendingSettingsSection } from "$lib/stores/navigation";

  /**
   * Deep-link map — translates a slot's logical target (`"ai"`,
   * `"integrations"`, `"updates"`) to the matching Settings sub-section
   * id in `SettingsPage.svelte`. `"integrations"` renders as the
   * Connection tab today because that's where GitHub / GitLab OAuth
   * lives; the naming reflects the slot's intent, not the tab label.
   */
  const SECTION_MAP: Record<string, string> = {
    ai: "ai",
    integrations: "connection",
    updates: "updates",
  };

  /**
   * Navigate to a Settings sub-section. The section key is translated
   * via `SECTION_MAP` and stashed in `pendingSettingsSection` so
   * `SettingsPage` can pick it up on mount (or immediately, if Settings
   * is already active).
   */
  function onNavigate(section: string): void {
    const target = SECTION_MAP[section] ?? section;
    pendingSettingsSection.set(target);
    activeViewStore.set("settings");
  }
</script>

<footer class="status-bar" data-testid="statusbar">
  <div class="status-left">
    <RepoSlot onOpenView={(view) => activeViewStore.set(view)} />
    <TasksSlot onOpen={toggleTasksPopover} />
    <span class="divider" aria-hidden="true"></span>
    <ForgeSlot {onNavigate} />
    <span class="divider" aria-hidden="true"></span>
    <AiSlot {onNavigate} />
    <span class="divider" aria-hidden="true"></span>
    <NetworkSlot />
  </div>
  <div class="status-right">
    <HelpSlot />
    <span class="divider" aria-hidden="true"></span>
    <VersionSlot {onNavigate} />
  </div>
</footer>

<style>
  .status-bar {
    height: 22px;
    min-height: 22px;
    background: var(--bg-toolbar);
    border-top: 1px solid var(--border);
    display: flex;
    align-items: stretch;
    justify-content: space-between;
    /* Lateral breathing room — slots otherwise sit flush against the
       window edges. */
    padding: 0 10px;
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    user-select: none;
  }

  .status-left,
  .status-right {
    display: flex;
    align-items: stretch;
    height: 100%;
  }

  .divider {
    width: 1px;
    background: var(--border);
    opacity: 0.4;
    align-self: center;
    height: 60%;
    flex-shrink: 0;
  }
</style>

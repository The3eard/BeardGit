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
    - Clicking **Tasks** toggles the `tasksDrawerOpen` store; the drawer
      itself is mounted in `+page.svelte`.
    - Clicking **Forge / AI / Version** dispatches the corresponding
      Settings sub-section key via `onNavigate` (which the parent wires
      to `activeViewStore.set("settings")`). Until the Settings IA
      overhaul (MT-5) supports sub-section deep-linking, only the
      top-level Settings view is opened.
    - **Network** is non-interactive and hides itself while online.
-->
<script lang="ts">
  import TasksSlot from "./statusbar/TasksSlot.svelte";
  import ForgeSlot from "./statusbar/ForgeSlot.svelte";
  import AiSlot from "./statusbar/AiSlot.svelte";
  import NetworkSlot from "./statusbar/NetworkSlot.svelte";
  import VersionSlot from "./statusbar/VersionSlot.svelte";
  import { toggleTasksDrawer } from "$lib/stores/tasksDrawer";
  import { activeViewStore } from "$lib/stores/navigation";

  /**
   * Navigate to a Settings sub-section. Until MT-5 ships sub-section
   * deep-linking we just open the top-level Settings view — the
   * `section` argument is threaded through for future use (and so the
   * slot-click tests can assert the correct key without exposing the
   * Settings internals).
   */
  function onNavigate(section: string): void {
    void section;
    activeViewStore.set("settings");
  }
</script>

<footer class="status-bar" data-testid="statusbar">
  <div class="status-left">
    <TasksSlot onOpen={toggleTasksDrawer} />
    <span class="divider" aria-hidden="true"></span>
    <ForgeSlot {onNavigate} />
    <span class="divider" aria-hidden="true"></span>
    <AiSlot {onNavigate} />
    <span class="divider" aria-hidden="true"></span>
    <NetworkSlot />
  </div>
  <div class="status-right">
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
    padding: 0;
    font-size: 11px;
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

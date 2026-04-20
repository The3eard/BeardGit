<!--
  RepoConfigDialog.svelte — the top-level "Repo settings" dialog.

  Composes the shared MT-5 `Dialog` + `CategoryNav` primitives and the
  per-section Svelte components (`GeneralSection`, `VisibilitySection`,
  `FeaturesSection`, `ProtectionSection`, `LabelsSection`).

  Lifecycle:
    1. On open, calls `probe_forge_cli_status` (Phase 7 gating).
       - `NotInstalled` → empty state with an "install gh/glab" link.
       - `UnsupportedForge` → "not supported" empty state.
       - `Installed` + `!authenticated` → "authenticate first" empty
         state with a deep-link to Settings → Integrations.
    2. If the CLI is installed + authenticated, calls
       `loadRemoteRepoConfig(repoPath)` and populates the store.
    3. Renders `CategoryNav` tabs; each tab maps to one Section.
    4. Save button diffs `before → current` via `buildPatch` and calls
       `applyRemoteRepoConfig`. Partial failures are surfaced as a
       mixed toast.

  Cancel / backdrop / Esc → if the patch is non-empty, prompt the user
  to confirm discarding edits. Otherwise close straight away.
-->
<script lang="ts">
  import { Button, CategoryNav, Dialog, Card } from "$lib/components/ui";
  import { addToast } from "$lib/stores/toast";
  import { activeProject } from "$lib/stores/projects";
  import { pendingSettingsSection } from "$lib/stores/navigation";
  import { activeViewStore } from "$lib/stores/navigation";
  import {
    repoConfigDialogOpen,
    repoConfigStore,
    repoConfigPatch,
    repoConfigDirty,
    setLoadedConfig,
    setLoading,
    setLoadError,
    resetRepoConfigStore,
    commitSavedConfig,
  } from "$lib/stores/repoConfig";
  import type { ForgeCliStatus } from "$lib/types/repoConfig";
  import {
    applyRemoteRepoConfig,
    loadRemoteRepoConfig,
    probeForgeCliStatus,
  } from "$lib/api/tauri";
  import GeneralSection from "./GeneralSection.svelte";
  import VisibilitySection from "./VisibilitySection.svelte";
  import FeaturesSection from "./FeaturesSection.svelte";
  import ProtectionSection from "./ProtectionSection.svelte";
  import LabelsSection from "./LabelsSection.svelte";

  type Forge = "github" | "gitlab";

  const sections = [
    { id: "general", label: "General", icon: "\uF085" },
    { id: "visibility", label: "Visibility", icon: "\uF06E" },
    { id: "features", label: "Features", icon: "\uF1E6" },
    { id: "protection", label: "Protection", icon: "\uF023" },
    { id: "labels", label: "Labels", icon: "\uF02B" },
  ] as const;

  let activeId = $state<string>("general");
  let saving = $state(false);
  let probeStatus = $state<ForgeCliStatus | null>(null);
  let probeError = $state<string | null>(null);
  let discardConfirmOpen = $state(false);

  /** Forge hint for child sections — purely for UI gating. */
  let forge = $derived.by<Forge | null>(() => {
    const repo = $activeProject;
    if (!repo) return null;
    // We don't round-trip this through the backend — the existing
    // `ProjectInfo` carries a `provider` hint used by the sidebar. Fall
    // back to `null` if we can't tell, which makes the protection tab
    // render its "not supported" state.
    const kind = (repo as unknown as { provider?: string }).provider;
    if (kind === "github" || kind === "gitlab") return kind;
    return null;
  });

  let repoPath = $derived($activeProject?.path ?? null);

  $effect(() => {
    if ($repoConfigDialogOpen) {
      void openDialogLifecycle();
    } else {
      // Wipe state so the next open starts fresh.
      resetRepoConfigStore();
      probeStatus = null;
      probeError = null;
      activeId = "general";
    }
  });

  async function openDialogLifecycle() {
    if (!repoPath) {
      probeError = "No active repository.";
      return;
    }
    probeStatus = null;
    probeError = null;
    try {
      const status = await probeForgeCliStatus(repoPath);
      probeStatus = status;
      if (status.kind === "installed" && status.authenticated) {
        await reloadConfig();
      }
    } catch (e) {
      probeError = String(e);
    }
  }

  async function reloadConfig() {
    if (!repoPath) return;
    setLoading(repoPath);
    try {
      const config = await loadRemoteRepoConfig(repoPath);
      setLoadedConfig(repoPath, config);
    } catch (e) {
      setLoadError(String(e));
    }
  }

  function goToIntegrations() {
    pendingSettingsSection.set("integrations");
    activeViewStore.set("settings");
    $repoConfigDialogOpen = false;
  }

  async function save() {
    const patch = $repoConfigPatch;
    if (!patch || !repoPath) return;
    saving = true;
    try {
      const result = await applyRemoteRepoConfig(repoPath, patch);
      if (result.failures.length === 0) {
        addToast({ message: "Repository settings saved", type: "success" });
      } else if (result.fields_updated.length === 0) {
        addToast({
          message: `Save failed: ${result.failures
            .map((f) => `${f.field} — ${f.message}`)
            .join("; ")}`,
          type: "error",
          duration: 9000,
        });
      } else {
        addToast({
          message: `Saved ${result.fields_updated.length} field(s); ${result.failures.length} failed.`,
          type: "warning",
          duration: 9000,
        });
      }
      // Reload so `before` reflects the server's new truth.
      if (repoPath) {
        try {
          const fresh = await loadRemoteRepoConfig(repoPath);
          commitSavedConfig(fresh);
        } catch {
          // Leave `before`/`current` as-is on reload failure — save
          // succeeded, only the refresh didn't.
        }
      }
    } catch (e) {
      addToast({ message: String(e), type: "error" });
    } finally {
      saving = false;
    }
  }

  function requestClose() {
    if ($repoConfigDirty) {
      discardConfirmOpen = true;
    } else {
      $repoConfigDialogOpen = false;
    }
  }

  function confirmDiscard() {
    discardConfirmOpen = false;
    $repoConfigDialogOpen = false;
  }

  function cancelDiscard() {
    discardConfirmOpen = false;
  }
</script>

<Dialog
  open={$repoConfigDialogOpen}
  onClose={requestClose}
  title="Repository settings"
  size="lg"
>
  <div class="repo-config-shell" data-testid="repo-config-dialog">
    <aside class="repo-config-nav" aria-label="Sections">
      <CategoryNav categories={[...sections]} bind:activeId />
    </aside>
    <section class="repo-config-body">
      {#if probeError}
        <Card title="Cannot open repo settings" description={probeError} />
      {:else if probeStatus === null}
        <p class="hint">Checking forge CLI availability…</p>
      {:else if probeStatus.kind === "unsupported_forge"}
        <Card
          title="Not a GitHub or GitLab repository"
          description="Remote repository configuration is only available for repos hosted on GitHub or GitLab. Add a supported remote and try again."
        />
      {:else if probeStatus.kind === "not_installed"}
        <Card
          title="Forge CLI not installed"
          description="Install the gh (GitHub) or glab (GitLab) CLI, then reopen this dialog."
        >
          {#snippet actions()}
            <Button onclick={goToIntegrations}>
              Go to Settings → Integrations
            </Button>
          {/snippet}
        </Card>
      {:else if probeStatus.kind === "installed" && !probeStatus.authenticated}
        <Card
          title="Authenticate with the forge"
          description="Sign in to the forge CLI so BeardGit can read and write repository settings on your behalf."
        >
          {#snippet actions()}
            <Button variant="primary" onclick={goToIntegrations}>
              Go to Settings → Integrations
            </Button>
          {/snippet}
        </Card>
      {:else if $repoConfigStore.loading}
        <p class="hint">Loading configuration…</p>
      {:else if $repoConfigStore.error}
        <Card title="Failed to load" description={$repoConfigStore.error} />
      {:else if $repoConfigStore.current}
        <div class="repo-config-section" data-testid={`repo-config-section-${activeId}`}>
          {#if activeId === "general"}
            <GeneralSection />
          {:else if activeId === "visibility"}
            <VisibilitySection />
          {:else if activeId === "features"}
            <FeaturesSection />
          {:else if activeId === "protection"}
            <ProtectionSection {forge} {repoPath} />
          {:else if activeId === "labels"}
            <LabelsSection />
          {/if}
        </div>
      {/if}
    </section>
  </div>
  {#snippet footer()}
    <Button onclick={requestClose}>Cancel</Button>
    <span data-testid="repo-config-save-wrap">
      <Button
        variant="primary"
        loading={saving}
        disabled={!$repoConfigDirty}
        onclick={save}
      >
        Save
      </Button>
    </span>
  {/snippet}
</Dialog>

<Dialog
  bind:open={discardConfirmOpen}
  title="Discard unsaved changes?"
  size="sm"
>
  <p data-testid="repo-config-discard-message">
    You have unsaved repository settings. Closing the dialog will lose them.
  </p>
  {#snippet footer()}
    <Button onclick={cancelDiscard}>Keep editing</Button>
    <Button variant="danger" onclick={confirmDiscard}>Discard</Button>
  {/snippet}
</Dialog>

<style>
  .repo-config-shell {
    display: grid;
    grid-template-columns: 180px 1fr;
    gap: 16px;
    min-height: 380px;
  }

  .repo-config-nav {
    border-right: 1px solid var(--border);
    padding-right: 8px;
  }

  .repo-config-body {
    overflow-y: auto;
    padding-left: 4px;
  }

  .repo-config-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .hint {
    font-size: 12px;
    color: var(--text-secondary);
    margin: 0;
  }
</style>

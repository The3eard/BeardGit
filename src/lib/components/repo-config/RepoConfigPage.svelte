<!--
  RepoConfigPage.svelte — main-area shell for the Repo settings view.

  Left pane: CategoryNav with the provider-driven section list.
  Right pane: active section's editor, with inline loading / error /
  auth-failure states and a sticky Save/Discard footer when dirty.

  Navigation guards intercept:
    1. section switches via CategoryNav,
    2. leaving the view (handled in +page.svelte using `pendingLeave`),
    3. active-project changes (handled in +page.svelte).

  External entry points call `requestGuardedNavigation(run)` via a
  parent `bind:this` reference.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { Button, Card, CategoryNav, Skeleton } from "$lib/components/ui";
  import * as m from "$lib/paraglide/messages";
  import { activeProject } from "$lib/stores/projects";
  import { activeProvider } from "$lib/stores/provider";
  import { addToast } from "$lib/stores/toast";
  import {
    activeViewStore,
    pendingSettingsSection,
  } from "$lib/stores/navigation";
  import {
    applyRemoteRepoConfig,
    probeForgeCliStatus,
  } from "$lib/api/tauri";
  import {
    repoConfigStore,
    setLoadedConfig,
    setLoading,
    setLoadError,
    commitSavedConfig,
    updateCurrent,
  } from "$lib/stores/repoConfig";
  import {
    repoConfigRoute,
    setSection,
    pendingRepoConfigSection,
  } from "$lib/stores/repoConfigRoute";
  import {
    sectionsForProvider,
    type SectionId,
  } from "$lib/repo-config/sections";
  import {
    isSectionDirty,
    sectionPatchFor,
    resetSectionFields,
  } from "$lib/repo-config/patches";
  import { loadConfig, invalidate } from "$lib/repo-config/loader";
  import type { ApplyResult } from "$lib/types/repoConfig";
  import NavigationGuardDialog from "./NavigationGuardDialog.svelte";
  import SectionFooter from "./SectionFooter.svelte";
  import GeneralSection from "./GeneralSection.svelte";
  import VisibilitySection from "./VisibilitySection.svelte";
  import FeaturesSection from "./FeaturesSection.svelte";
  import ProtectionSection from "./ProtectionSection.svelte";
  import LabelsSection from "./LabelsSection.svelte";

  type Forge = "github" | "gitlab";

  let repoPath = $derived($activeProject?.path ?? null);
  // Matches the gating logic used by the sidebar entry — both hinge on
  // the globally-active provider connection. `ProjectInfo` doesn't
  // carry a `provider` field, so trying to derive one off `$activeProject`
  // always collapses to null and renders the "not supported" card even
  // for repos the user has full access to.
  let providerKind = $derived.by<Forge | null>(() => {
    const k = $activeProvider?.kind;
    return k === "github" || k === "gitlab" ? k : null;
  });

  let sections = $derived(
    providerKind ? sectionsForProvider(providerKind) : [],
  );

  let activeSection = $derived<SectionId>($repoConfigRoute.section);
  let activeSectionLabel = $derived(
    sections.find((s) => s.id === activeSection)?.label ?? activeSection,
  );

  let authRequired = $state(false);
  let pendingAction = $state<null | (() => void)>(null);
  let saving = $state(false);

  let dirty = $derived.by(() => {
    const before = $repoConfigStore.before;
    const current = $repoConfigStore.current;
    if (!before || !current) return false;
    return isSectionDirty(activeSection, before, current);
  });

  onMount(() => {
    // IMPORTANT: consume the pending deep-link target BEFORE kicking
    // off ensureLoaded, otherwise a reactive race can leave the
    // store/route out of sync during the first load.
    const pending = get(pendingRepoConfigSection);
    if (pending) {
      setSection(pending);
      pendingRepoConfigSection.set(null);
    }

    if (!providerKind || !repoPath) return;
    void ensureProbe();
    void ensureLoaded();
  });

  async function ensureProbe(): Promise<void> {
    if (!repoPath) return;
    try {
      const status = await probeForgeCliStatus(repoPath);
      if (status.kind === "installed" && !status.authenticated) {
        authRequired = true;
      }
    } catch {
      // Probe failure is non-fatal — config load will surface a real error.
    }
  }

  async function ensureLoaded(): Promise<void> {
    if (!repoPath) return;
    if (
      $repoConfigStore.repoPath === repoPath &&
      $repoConfigStore.current &&
      $repoConfigStore.before
    ) {
      return;
    }
    setLoading(repoPath);
    try {
      const config = await loadConfig(repoPath);
      setLoadedConfig(repoPath, config);
    } catch (e) {
      const msg = String(e);
      // Match the structured `RepoConfigError::NotAuthenticated` Display
      // prefix produced by the Rust side. A bare /auth/i substring match
      // fires on unrelated error text (e.g. "author"), which historically
      // surfaced the auth-required empty state for non-auth failures.
      if (/^(?:Error: )?CLI not authenticated[:,]/.test(msg)) {
        authRequired = true;
      }
      setLoadError(msg);
    }
  }

  function goToIntegrations(): void {
    pendingSettingsSection.set("integrations");
    activeViewStore.set("settings");
  }

  function reportApplyResult(result: ApplyResult, sectionLabel: string): void {
    const failureLines = result.failures.map((f) =>
      f.message
        ? `${sectionLabel} — ${f.field}: ${f.message}`
        : `${sectionLabel} — ${f.field} could not be saved`,
    );
    if (result.failures.length === 0) {
      addToast({ message: m.repo_config_saved_toast(), type: "success" });
      return;
    }
    if (result.fields_updated.length === 0) {
      addToast({
        message: failureLines.join("\n"),
        type: "error",
        duration: 9000,
      });
      return;
    }
    addToast({
      message: [
        `Saved ${result.fields_updated.length} field(s).`,
        ...failureLines,
      ].join("\n"),
      type: "warning",
      duration: 9000,
    });
  }

  async function saveActiveSection(): Promise<boolean> {
    const before = $repoConfigStore.before;
    const current = $repoConfigStore.current;
    if (!before || !current || !repoPath) return false;
    const patch = sectionPatchFor(activeSection, before, current);
    saving = true;
    try {
      const result = await applyRemoteRepoConfig(repoPath, patch);
      invalidate(repoPath);
      const fresh = await loadConfig(repoPath, { force: true });
      commitSavedConfig(fresh);
      reportApplyResult(result, activeSectionLabel);
      return result.failures.length === 0;
    } catch (e) {
      reportApplyResult(
        { fields_updated: [], failures: [{ field: "*", message: String(e) }] },
        activeSectionLabel,
      );
      return false;
    } finally {
      saving = false;
    }
  }

  function discardActiveSection(): void {
    const before = $repoConfigStore.before;
    if (!before) return;
    updateCurrent((draft) => {
      resetSectionFields(activeSection, draft, before);
    });
  }

  function tryNavigate(run: () => void): void {
    if (!dirty) {
      run();
      return;
    }
    pendingAction = run;
  }

  function onSelectSection(id: string): void {
    if (id === activeSection) return;
    tryNavigate(() => setSection(id));
  }

  async function guardSave(): Promise<void> {
    const ok = await saveActiveSection();
    if (ok && pendingAction) {
      const run = pendingAction;
      pendingAction = null;
      run();
    } else if (!ok) {
      pendingAction = null;
    }
  }

  function guardDiscard(): void {
    discardActiveSection();
    if (pendingAction) {
      const run = pendingAction;
      pendingAction = null;
      run();
    }
  }

  function guardCancel(): void {
    pendingAction = null;
  }

  /** Public entry point for +page.svelte to route external navigation through the guard. */
  export function requestGuardedNavigation(run: () => void): void {
    tryNavigate(run);
  }
</script>

<div class="repo-config-page" data-testid="repo-config-page">
  {#if !providerKind}
    <Card
      title={m.repo_config_not_supported()}
      description="This project doesn't have a supported GitHub or GitLab remote."
    />
  {:else}
    <aside class="nav" aria-label="Repo settings sections">
      <CategoryNav
        categories={[...sections]}
        activeId={activeSection}
        onSelect={onSelectSection}
      />
    </aside>
    <section class="body">
      {#if authRequired}
        <Card
          title={m.repo_config_authenticate_title()}
          description={m.repo_config_authenticate_body()}
        >
          {#snippet actions()}
            <Button variant="primary" onclick={goToIntegrations}>
              {m.repo_config_go_to_integrations()}
            </Button>
          {/snippet}
        </Card>
      {:else if $repoConfigStore.loading}
        <div data-testid="repo-config-loading" aria-label={m.repo_config_loading()}>
          <Skeleton rows={6} />
        </div>
      {:else if $repoConfigStore.error}
        <Card title="Failed to load" description={$repoConfigStore.error}>
          {#snippet actions()}
            <Button onclick={() => void ensureLoaded()}>
              {m.repo_config_retry()}
            </Button>
          {/snippet}
        </Card>
      {:else if $repoConfigStore.current}
        <div
          class="section-slot"
          data-testid={`repo-config-section-${activeSection}`}
        >
          {#if activeSection === "general"}
            <GeneralSection />
          {:else if activeSection === "visibility"}
            <VisibilitySection />
          {:else if activeSection === "features"}
            <FeaturesSection />
          {:else if activeSection === "protection"}
            <ProtectionSection forge={providerKind} {repoPath} />
          {:else if activeSection === "labels"}
            <LabelsSection />
          {/if}
        </div>
        <SectionFooter
          {dirty}
          {saving}
          onSave={() => void saveActiveSection()}
          onDiscard={discardActiveSection}
        />
      {/if}
    </section>
  {/if}
</div>

<NavigationGuardDialog
  open={pendingAction !== null}
  sectionLabel={activeSectionLabel}
  {saving}
  onSave={() => void guardSave()}
  onDiscard={guardDiscard}
  onCancel={guardCancel}
/>

<style>
  .repo-config-page {
    display: grid;
    grid-template-columns: 200px 1fr;
    gap: 16px;
    height: 100%;
    min-height: 0;
    padding: 16px;
  }
  .nav {
    border-right: 1px solid var(--border);
    padding-right: 8px;
    overflow-y: auto;
  }
  .body {
    position: relative;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .section-slot {
    flex: 1;
    min-height: 0;
  }
</style>

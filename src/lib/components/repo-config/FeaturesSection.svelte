<!--
  FeaturesSection.svelte — Issues/Wiki toggles + default-branch picker.

  The default-branch dropdown is populated from the existing
  `branches` store (`$lib/stores/branches`) which is already kept in
  sync by the repo lifecycle. We only surface *local* branches: setting
  the default branch to a remote-only ref would be surprising and is
  not what the forge CLIs expect.

  Features toggles are non-destructive so no confirmation dialog is
  needed here — the plan explicitly carves out destructive confirms to
  Visibility (archive) and Labels (delete).
-->
<script lang="ts">
  import { FormRow } from "$lib/components/ui";
  import { repoConfigStore, updateCurrent } from "$lib/stores/repoConfig";
  import { localBranches } from "$lib/stores/branches";

  let current = $derived($repoConfigStore.current);

  function toggleIssues(event: Event) {
    const checked = (event.target as HTMLInputElement).checked;
    updateCurrent((draft) => {
      draft.issues_enabled = checked;
    });
  }

  function toggleWiki(event: Event) {
    const checked = (event.target as HTMLInputElement).checked;
    updateCurrent((draft) => {
      draft.wiki_enabled = checked;
    });
  }

  function setDefaultBranch(event: Event) {
    const value = (event.target as HTMLSelectElement).value;
    updateCurrent((draft) => {
      draft.default_branch = value;
    });
  }

  // Dedupe + sort alphabetically for a stable dropdown. If the current
  // default branch isn't in the local list (rare: it was deleted
  // locally but still exists remotely) we add it so the user isn't
  // locked out of seeing the real value.
  let branchOptions = $derived.by(() => {
    const names = new Set<string>($localBranches.map((b) => b.name));
    if (current?.default_branch) names.add(current.default_branch);
    return [...names].sort();
  });
</script>

<div class="repo-config-features" data-testid="repo-config-features">
  {#if current}
    <FormRow
      label="Issues"
      for="repo-config-issues"
      helperText="When disabled, the Issues tab is hidden on the forge."
    >
      <input
        id="repo-config-issues"
        type="checkbox"
        class="bg-toggle"
        checked={current.issues_enabled}
        onchange={toggleIssues}
        data-testid="repo-config-issues"
      />
    </FormRow>

    <FormRow
      label="Wiki"
      for="repo-config-wiki"
      helperText="When disabled, the Wiki tab is hidden on the forge."
    >
      <input
        id="repo-config-wiki"
        type="checkbox"
        class="bg-toggle"
        checked={current.wiki_enabled}
        onchange={toggleWiki}
        data-testid="repo-config-wiki"
      />
    </FormRow>

    <FormRow
      label="Default branch"
      for="repo-config-default-branch"
      helperText="Branch opened by default when visitors land on the repo page."
    >
      <select
        id="repo-config-default-branch"
        class="bg-select"
        value={current.default_branch}
        onchange={setDefaultBranch}
        data-testid="repo-config-default-branch"
      >
        {#each branchOptions as branch (branch)}
          <option value={branch}>{branch}</option>
        {/each}
      </select>
    </FormRow>
  {/if}
</div>

<style>
  .repo-config-features {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .bg-select {
    padding: 4px 8px;
    background: var(--bg-input);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-family: inherit;
    font-size: 12px;
    min-width: 200px;
  }

  .bg-select:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .bg-toggle {
    width: 14px;
    height: 14px;
  }
</style>

<!--
  ProtectionSection.svelte — GitHub-only branch-protection editor.

  Three modes:
    1. GitLab repo → render a neutral "not supported" Card.
    2. GitHub repo, no branch selected → prompt to pick one.
    3. GitHub repo, branch selected → load current rules via
       `get_branch_protection`, render toggles + Save button that calls
       `set_branch_protection`.

  Rules are local to this component — they do not live on
  `RemoteRepoConfig` because branch protection is a per-branch concept
  and not a field of the repo itself. The dialog's top-level Save
  button only drives the repo-config patch; protection gets its own
  "Apply protection" button inside this card.
-->
<script lang="ts">
  import { Button, Card, Field, FormRow } from "$lib/components/ui";
  import type { BranchProtection } from "$lib/types/repoConfig";

  /** Local alias — keeps this file independent of the provider store. */
  type Forge = "github" | "gitlab";
  import { getBranchProtection, setBranchProtection } from "$lib/api/tauri";
  import { localBranches } from "$lib/stores/branches";
  import { addToast } from "$lib/stores/toast";

  interface Props {
    /** Detected forge — controls whether protection is supported. */
    forge: Forge | null;
    /** Active repo path; forwarded to the Tauri commands. */
    repoPath: string | null;
  }

  let { forge, repoPath }: Props = $props();

  let selectedBranch = $state<string>("");
  let loading = $state(false);
  let saving = $state(false);
  let rules = $state<BranchProtection>(emptyRules());
  let error = $state<string | null>(null);

  function emptyRules(): BranchProtection {
    return {
      require_pull_request: false,
      required_approvals: 0,
      require_status_checks: false,
      status_check_contexts: [],
      require_up_to_date: false,
      require_conversation_resolution: false,
      enforce_admins: false,
    };
  }

  let branchNames = $derived($localBranches.map((b) => b.name).sort());

  async function onBranchChange(event: Event) {
    const next = (event.target as HTMLSelectElement).value;
    selectedBranch = next;
    error = null;
    if (!next || !repoPath) return;
    loading = true;
    try {
      const loaded = await getBranchProtection(repoPath, next);
      rules = loaded ?? emptyRules();
    } catch (e) {
      error = String(e);
      rules = emptyRules();
    } finally {
      loading = false;
    }
  }

  function setRule<K extends keyof BranchProtection>(
    key: K,
    value: BranchProtection[K],
  ) {
    rules = { ...rules, [key]: value };
  }

  async function saveRules() {
    if (!repoPath || !selectedBranch) return;
    saving = true;
    error = null;
    try {
      await setBranchProtection(repoPath, selectedBranch, rules);
      addToast({
        message: `Branch protection saved for ${selectedBranch}`,
        type: "success",
      });
    } catch (e) {
      error = String(e);
      addToast({ message: String(e), type: "error" });
    } finally {
      saving = false;
    }
  }
</script>

<div class="repo-config-protection" data-testid="repo-config-protection">
  {#if forge !== "github"}
    <Card
      title="Not supported on this provider"
      description="Branch protection is only available for GitHub repositories at the moment. GitLab support is tracked in the project backlog."
    />
  {:else}
    <Field label="Branch" description="Pick a branch to configure protection for.">
      <select
        class="bg-select"
        value={selectedBranch}
        onchange={onBranchChange}
        data-testid="repo-config-protection-branch"
      >
        <option value="">Select a branch…</option>
        {#each branchNames as branch (branch)}
          <option value={branch}>{branch}</option>
        {/each}
      </select>
    </Field>

    {#if loading}
      <p class="hint">Loading current protection…</p>
    {:else if selectedBranch}
      <div class="rules">
        <FormRow
          label="Require pull request before merging"
          for="protect-require-pr"
        >
          <input
            id="protect-require-pr"
            type="checkbox"
            checked={rules.require_pull_request}
            onchange={(e) =>
              setRule(
                "require_pull_request",
                (e.target as HTMLInputElement).checked,
              )}
            data-testid="protect-require-pr"
          />
        </FormRow>

        {#if rules.require_pull_request}
          <FormRow label="Required approvals" for="protect-approvals">
            <input
              id="protect-approvals"
              type="number"
              min="0"
              max="10"
              class="bg-input-number"
              value={rules.required_approvals}
              oninput={(e) =>
                setRule(
                  "required_approvals",
                  Number((e.target as HTMLInputElement).value),
                )}
              data-testid="protect-approvals"
            />
          </FormRow>
        {/if}

        <FormRow
          label="Require status checks to pass"
          for="protect-require-status"
        >
          <input
            id="protect-require-status"
            type="checkbox"
            checked={rules.require_status_checks}
            onchange={(e) =>
              setRule(
                "require_status_checks",
                (e.target as HTMLInputElement).checked,
              )}
            data-testid="protect-require-status"
          />
        </FormRow>

        <FormRow
          label="Require branches to be up-to-date before merging"
          for="protect-up-to-date"
        >
          <input
            id="protect-up-to-date"
            type="checkbox"
            checked={rules.require_up_to_date}
            onchange={(e) =>
              setRule(
                "require_up_to_date",
                (e.target as HTMLInputElement).checked,
              )}
            data-testid="protect-up-to-date"
          />
        </FormRow>

        <FormRow
          label="Require conversation resolution before merging"
          for="protect-resolve-conversations"
        >
          <input
            id="protect-resolve-conversations"
            type="checkbox"
            checked={rules.require_conversation_resolution}
            onchange={(e) =>
              setRule(
                "require_conversation_resolution",
                (e.target as HTMLInputElement).checked,
              )}
            data-testid="protect-resolve-conversations"
          />
        </FormRow>

        <FormRow label="Include administrators" for="protect-enforce-admins">
          <input
            id="protect-enforce-admins"
            type="checkbox"
            checked={rules.enforce_admins}
            onchange={(e) =>
              setRule(
                "enforce_admins",
                (e.target as HTMLInputElement).checked,
              )}
            data-testid="protect-enforce-admins"
          />
        </FormRow>

        {#if error}
          <p class="error" role="alert" data-testid="protect-error">{error}</p>
        {/if}

        <div class="actions">
          <Button variant="primary" loading={saving} onclick={saveRules}>
            Apply protection
          </Button>
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .repo-config-protection {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .rules {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 4px;
  }

  .hint {
    font-size: 12px;
    color: var(--text-secondary);
    margin: 0;
  }

  .error {
    font-size: 11px;
    color: var(--accent-red);
    margin: 0;
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

  .bg-input-number {
    width: 72px;
    padding: 4px 8px;
    background: var(--bg-input);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-family: inherit;
    font-size: 12px;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 8px;
  }
</style>

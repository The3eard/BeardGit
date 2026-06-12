<!--
  CreateReleaseDialog — modal for creating a new release. Tag picker has two
  modes: pick an existing tag, or create + push a new tag inline before
  creating the release. In "new" mode the create+push+release flow runs as
  a single TaskManager task so partial failure is visible. GitHub-only
  flags (draft, prerelease, generate_notes) are hidden for GitLab.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import Button from "$lib/components/ui/Button.svelte";
  import { Checkbox } from "$lib/components/ui";
  import { activeProvider } from "../../stores/provider";
  import {
    doCreateRelease,
    doCreateTagAndRelease,
    refreshReleases,
  } from "../../stores/releases";
  import { getBranches, listTagsPaginated } from "../../api/tauri";
  import * as m from "$lib/paraglide/messages";
  import type {
    BranchInfo,
    TagInfo,
    CreateReleaseInput,
  } from "../../types";

  let { onClose }: { onClose: () => void } = $props();

  let isGitHub = $derived($activeProvider?.kind === "github");

  let mode = $state<"existing" | "new">("existing");
  let tags = $state<TagInfo[]>([]);
  let branches = $state<BranchInfo[]>([]);

  let selectedTag = $state("");
  let newTagName = $state("");
  let sourceBranch = $state("");
  let remote = $state("origin");

  let name = $state("");
  let body = $state("");
  let draft = $state(false);
  let prerelease = $state(false);
  let generateNotes = $state(false);

  let submitting = $state(false);
  let errorMsg = $state("");

  onMount(async () => {
    try {
      const [t, b] = await Promise.all([
        listTagsPaginated(50, 1),
        getBranches(),
      ]);
      tags = t;
      branches = b;
      const head = branches.find((x) => x.is_head && !x.is_remote);
      if (head) sourceBranch = head.name;
      if (tags.length > 0 && !selectedTag) selectedTag = tags[0].name;
    } catch {
      /* ignore — user can still type values */
    }
  });

  function keydown(e: KeyboardEvent): void {
    if (e.key === "Escape") onClose();
  }

  async function submit(): Promise<void> {
    const effectiveTag =
      mode === "existing" ? selectedTag : newTagName.trim();
    if (!effectiveTag) {
      errorMsg = m.release_tag_required();
      return;
    }
    if (mode === "new" && !sourceBranch) {
      errorMsg = m.release_source_ref_required();
      return;
    }
    if (!name.trim()) {
      errorMsg = m.release_name_required();
      return;
    }
    submitting = true;
    errorMsg = "";
    try {
      const input: CreateReleaseInput = {
        tag: effectiveTag,
        target_commit: mode === "new" ? sourceBranch : "",
        name: name.trim(),
        body,
        draft: isGitHub ? draft : false,
        prerelease: isGitHub ? prerelease : false,
        generate_notes: isGitHub ? generateNotes : false,
      };
      if (mode === "new") {
        await doCreateTagAndRelease(effectiveTag, sourceBranch, remote, input);
        // List refreshes when the release-created event fires; also refresh now.
        void refreshReleases();
      } else {
        await doCreateRelease(input);
      }
      onClose();
    } catch (e) {
      errorMsg = String(e);
    } finally {
      submitting = false;
    }
  }

  let localBranches = $derived(branches.filter((b) => !b.is_remote));
</script>

<svelte:window onkeydown={keydown} />

<button
  class="backdrop"
  type="button"
  onclick={onClose}
  aria-label={m.release_cancel()}
></button>
<div
  class="dialog"
  role="dialog"
  aria-modal="true"
  aria-label={m.release_new_button()}
>
  <h3 class="dialog-title">{m.release_create_title()}</h3>

  <div class="form-field">
    <div class="label">{m.release_tag_mode_label()}</div>
    <div class="mode-tabs">
      <button
        type="button"
        class="tab"
        class:active={mode === "existing"}
        onclick={() => (mode = "existing")}
      >{m.release_tag_mode_existing()}</button>
      <button
        type="button"
        class="tab"
        class:active={mode === "new"}
        onclick={() => (mode = "new")}
      >{m.release_tag_mode_new()}</button>
    </div>
  </div>

  {#if mode === "existing"}
    <div class="form-field">
      <label for="tag-select">{m.release_tag_label()}</label>
      <select id="tag-select" bind:value={selectedTag}>
        {#each tags as t (t.name)}
          <option value={t.name}>{t.name}</option>
        {/each}
      </select>
    </div>
  {:else}
    <div class="form-field">
      <label for="tag-name">{m.release_new_tag_name_label()}</label>
      <input id="tag-name" type="text" bind:value={newTagName} placeholder="v1.2.3" />
    </div>
    <div class="form-field">
      <label for="src-branch">{m.release_source_branch_label()}</label>
      <select id="src-branch" bind:value={sourceBranch}>
        {#each localBranches as b (b.name)}
          <option value={b.name}>{b.name}</option>
        {/each}
      </select>
    </div>
    <div class="form-field">
      <label for="remote">{m.release_remote_label()}</label>
      <input id="remote" type="text" bind:value={remote} />
    </div>
  {/if}

  <div class="form-field">
    <label for="r-name">{m.release_name_label()}</label>
    <input id="r-name" type="text" bind:value={name} />
  </div>

  <div class="form-field">
    <label for="r-body">{m.release_body_label()}</label>
    <textarea id="r-body" rows="6" bind:value={body}></textarea>
  </div>

  {#if isGitHub}
    <div class="form-field inline">
      <span class="inline-toggle">
        <Checkbox
          id="release-draft"
          checked={draft}
          onchange={(e) => { draft = (e.target as HTMLInputElement).checked; }}
        />
        <label for="release-draft">{m.release_draft_label()}</label>
      </span>
    </div>
    <div class="form-field inline">
      <span class="inline-toggle">
        <Checkbox
          id="release-prerelease"
          checked={prerelease}
          onchange={(e) => { prerelease = (e.target as HTMLInputElement).checked; }}
        />
        <label for="release-prerelease">{m.release_prerelease_label()}</label>
      </span>
    </div>
    <div class="form-field inline">
      <span class="inline-toggle">
        <Checkbox
          id="release-generate-notes"
          checked={generateNotes}
          onchange={(e) => { generateNotes = (e.target as HTMLInputElement).checked; }}
        />
        <label for="release-generate-notes">{m.release_generate_notes_label()}</label>
      </span>
    </div>
  {/if}

  {#if errorMsg}<p class="error-msg">{errorMsg}</p>{/if}

  <div class="dialog-actions">
    <Button variant="neutral" onclick={onClose}>
      {m.release_cancel()}
    </Button>
    <Button
      variant="primary"
      disabled={submitting}
      onclick={submit}
    >
      {submitting ? "…" : m.release_create_button()}
    </Button>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5); /* beardgit:allow-hex: modal backdrop neutral */
    z-index: 100;
    border: none;
    padding: 0;
  }
  .dialog {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 20px;
    z-index: 101;
    min-width: 480px;
    max-width: 560px;
    max-height: 80vh;
    overflow-y: auto;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4); /* beardgit:allow-hex: drop shadow neutral */
  }
  .dialog-title {
    margin: 0 0 16px;
    font-size: 16px;
  }
  .form-field {
    margin-bottom: 12px;
  }
  .form-field label,
  .form-field .label {
    display: block;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 4px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .form-field.inline .inline-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .form-field.inline label {
    display: inline;
    font-size: 12px;
    text-transform: none;
    color: var(--text-primary);
    cursor: pointer;
    font-weight: normal;
    letter-spacing: 0;
    margin-bottom: 0;
  }
  .form-field input[type="text"],
  .form-field select,
  .form-field textarea {
    width: 100%;
    padding: 6px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: inherit;
    box-sizing: border-box;
  }
  .form-field textarea {
    resize: vertical;
    min-height: 80px;
  }
  .mode-tabs {
    display: flex;
    border: 1px solid var(--border);
    border-radius: 4px;
    overflow: hidden;
  }
  .tab {
    flex: 1;
    padding: 4px 8px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 12px;
  }
  .tab.active {
    background: var(--accent-primary);
    color: var(--text-primary);
  }
  .error-msg {
    padding: 6px 10px;
    background: var(--overlay-accent-red);
    border: 1px solid color-mix(in srgb, var(--accent-red) 30%, transparent);
    border-radius: 4px;
    color: var(--accent-red);
    font-size: 12px;
    margin-bottom: 12px;
  }
  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 12px;
  }
</style>

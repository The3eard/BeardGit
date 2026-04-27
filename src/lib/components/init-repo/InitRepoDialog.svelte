<!--
  InitRepoDialog.svelte — drives the "initialize this folder as a Git repo"
  flow when the user opens a non-repo path.

  Subscribes to `initRepoRequest`. When the store flips to a request, the
  dialog opens, scans the folder via `count_folder_contents`, and lets the
  user choose whether to:

    1. Initialize the folder as a Git repo (always),
    2. Write a multi-purpose `.gitignore` and create the initial commit, and
    3. Create a remote on a connected provider (GitHub or GitLab) and push.

  Submits the whole pipeline through the `init_repo` Tauri command, then
  hands the path to `openProjectTab` so the freshly-initialised repo opens
  as a normal project tab. Errors from the backend pipeline are typed
  (`{ step, message, provider? }`) and translated into localised banner
  text via `formatStepError`.

  Reuses the shared `Dialog` primitive for backdrop / focus-trap / Esc
  semantics, and `addToast` for the success toast.
-->
<script lang="ts">
  import { initRepoRequest, closeInitRepoDialog } from "$lib/stores/initRepoDialog";
  import { providerStatus } from "$lib/stores/provider";
  import { initRepo, countFolderContents } from "$lib/api/tauri";
  import type { FolderCount, InitRepoOptions, RemoteOption } from "$lib/api/tauri";
  import { openProjectTab } from "$lib/stores/projects";
  import { MULTI_PURPOSE_GITIGNORE } from "$lib/data/gitignore-template";
  import { addToast } from "$lib/stores/toast";
  import { Button, Dialog } from "$lib/components/ui";
  import * as m from "$lib/paraglide/messages";

  // ── Local reactive state ─────────────────────────────────────────────
  let path = $state<string | null>(null);
  let count = $state<FolderCount | null>(null);
  let createRemote = $state(true);
  let providerIndex = $state(0);
  let name = $state("");
  let isPrivate = $state(true);
  let initialCommit = $state(true);
  let inFlight = $state(false);
  let currentStep = $state<string | null>(null);
  let bannerMessage = $state<string | null>(null);
  let remoteMode = $state<"create" | "existing">("create");
  let remoteUrl = $state("");

  // ── Derived ──────────────────────────────────────────────────────────
  let open = $derived($initRepoRequest !== null);
  let providerCount = $derived($providerStatus.providers.length);
  let canCreateRemote = $derived(providerCount > 0);

  const URL_PREFIXES = [
    "https://",
    "http://",
    "ssh://",
    "git@",
    "/",
    "./",
    "../",
  ] as const;
  let urlLooksValid = $derived(
    URL_PREFIXES.some((p) => remoteUrl.trim().startsWith(p)),
  );
  let existingValid = $derived(remoteUrl.trim().length > 0);
  let createValid = $derived(canCreateRemote && /^[A-Za-z0-9._-]+$/.test(name));
  let remoteValid = $derived(
    !createRemote || (remoteMode === "create" ? createValid : existingValid),
  );

  let primary = $derived.by(() => {
    if (!createRemote && !initialCommit) return m.init_repo_action_init();
    if (!createRemote && initialCommit) return m.init_repo_action_init_commit();
    if (createRemote && remoteMode === "create" && initialCommit)
      return m.init_repo_action_init_commit_push();
    if (createRemote && remoteMode === "create" && !initialCommit)
      return m.init_repo_action_init_remote();
    if (createRemote && remoteMode === "existing" && initialCommit)
      return m.init_repo_action_init_commit_push_existing();
    return m.init_repo_action_init_wire_origin();
  });

  let primaryTooltip = $derived.by(() => {
    const lines: string[] = [];
    lines.push(m.init_repo_step_git_init());
    if (initialCommit) {
      lines.push(m.init_repo_step_write_gitignore());
      lines.push(m.init_repo_step_stage_commit());
    }
    if (createRemote && remoteMode === "create" && canCreateRemote) {
      const provider =
        $providerStatus.providers[providerIndex]?.kind === "gitlab"
          ? "GitLab"
          : "GitHub";
      lines.push(m.init_repo_step_create_repo_on({ provider }));
      lines.push(m.init_repo_step_wire_origin_new());
    } else if (createRemote && remoteMode === "existing" && remoteUrl.trim()) {
      lines.push(m.init_repo_step_wire_origin_url({ url: remoteUrl.trim() }));
    }
    if (createRemote && initialCommit)
      lines.push(m.init_repo_step_push_main({ branch: "main" }));
    return "• " + lines.join("\n• ");
  });

  // ── Effects ──────────────────────────────────────────────────────────
  // Reset state when the request changes (open / close / new path).
  $effect(() => {
    const req = $initRepoRequest;
    if (!req) {
      path = null;
      count = null;
      bannerMessage = null;
      currentStep = null;
      inFlight = false;
      return;
    }
    path = req.path;
    // Basename: trim trailing slash, take last non-empty path component.
    name = req.path.replace(/\/+$/, "").split("/").filter(Boolean).pop() ?? "";
    bannerMessage = null;
    currentStep = null;
    void countFolderContents(req.path).then((c) => {
      count = c;
    });
    remoteUrl = "";
    if ($providerStatus.providers.length === 0) {
      createRemote = false;
      remoteMode = "existing";
    } else {
      providerIndex = $providerStatus.active_index ?? 0;
      createRemote = true;
      remoteMode = "create";
    }
    isPrivate = true;
    initialCommit = true;
  });

  // ── Behaviour ────────────────────────────────────────────────────────
  function cancel() {
    closeInitRepoDialog();
  }

  async function submit() {
    if (!path) return;
    if (!remoteValid) return;
    inFlight = true;
    bannerMessage = null;
    currentStep = m.init_repo_step_init();

    const remote: RemoteOption | null = !createRemote
      ? null
      : remoteMode === "create" && canCreateRemote
        ? {
            kind: "create",
            providerIndex,
            name,
            private: isPrivate,
            pushAfter: initialCommit,
          }
        : remoteMode === "existing"
          ? {
              kind: "use_existing",
              url: remoteUrl.trim(),
              pushAfter: initialCommit,
            }
          : null;

    const opts: InitRepoOptions = {
      path,
      initialBranch: "main",
      gitignore: initialCommit ? MULTI_PURPOSE_GITIGNORE : null,
      initialCommit,
      remote,
    };

    const successPath = path;
    const successName = name;
    const mode = remote?.kind ?? "none";
    const pushAfterFinal = remote?.pushAfter ?? false;

    try {
      const out = await initRepo(opts);
      closeInitRepoDialog();
      if (mode === "create" && out.web_url) {
        const host = new URL(out.web_url).host;
        addToast({
          type: "success",
          message: m.init_repo_success_toast({ name: successName, host }),
        });
      } else if (mode === "use_existing" && pushAfterFinal) {
        addToast({
          type: "success",
          message: m.init_repo_success_toast_existing({ name: successName }),
        });
      } else {
        addToast({
          type: "success",
          message: m.init_repo_success_toast_local({ name: successName }),
        });
      }
      await openProjectTab(successPath);
    } catch (err: unknown) {
      bannerMessage = formatStepError(err);
    } finally {
      inFlight = false;
      currentStep = null;
    }
  }

  function formatStepError(err: unknown): string {
    if (err && typeof err === "object" && "step" in err && "message" in err) {
      const e = err as { step: string; message: string; provider?: string };
      switch (e.step) {
        case "init":
          return m.init_repo_error_init({ message: e.message });
        case "commit":
          return m.init_repo_error_commit({ message: e.message });
        case "create_remote":
          return m.init_repo_error_create_remote({
            provider: e.provider ?? "provider",
            message: e.message,
          });
        case "add_origin":
          return m.init_repo_error_add_origin({ message: e.message });
        case "push":
          return m.init_repo_error_push({ message: e.message });
      }
      return e.message;
    }
    return String(err);
  }

  function formatBytes(b: number): string {
    if (b < 1024) return `${b} B`;
    if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
    if (b < 1024 * 1024 * 1024) return `${(b / 1024 / 1024).toFixed(1)} MB`;
    return `${(b / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }

  function providerKindLabel(kind: "github" | "gitlab"): string {
    return kind === "github" ? "GitHub" : "GitLab";
  }
</script>

<Dialog {open} onClose={cancel} title={m.init_repo_title()} size="md">
  <p class="dialog-intro">{m.init_repo_intro({ path: path ?? "" })}</p>

  <fieldset class="section" disabled={inFlight}>
    <label class="checkbox" title={m.tooltip_add_remote()}>
      <input type="checkbox" bind:checked={createRemote} />
      <span>{m.init_repo_add_remote()}</span>
    </label>

    {#if createRemote}
      <div class="mode-group" role="radiogroup">
        <label class="radio" title={m.tooltip_mode_create()}>
          <input
            type="radio"
            name="remote-mode"
            value="create"
            bind:group={remoteMode}
            disabled={!canCreateRemote}
          />
          <span>
            {m.init_repo_mode_create({
              provider:
                $providerStatus.providers[providerIndex]?.kind === "gitlab"
                  ? "GitLab"
                  : "GitHub",
            })}
          </span>
        </label>
        {#if !canCreateRemote}
          <div class="hint">{m.init_repo_create_remote_disabled_hint()}</div>
        {/if}

        <label class="radio" title={m.tooltip_mode_existing()}>
          <input
            type="radio"
            name="remote-mode"
            value="existing"
            bind:group={remoteMode}
          />
          <span>{m.init_repo_mode_existing()}</span>
        </label>
      </div>

      {#if remoteMode === "create" && canCreateRemote}
        {#if providerCount > 1}
          <label class="field" title={m.tooltip_provider()}>
            <span class="field-label">{m.init_repo_provider_label()}</span>
            <select bind:value={providerIndex}>
              {#each $providerStatus.providers as p, i (i)}
                <option value={i}>{providerKindLabel(p.kind)}</option>
              {/each}
            </select>
          </label>
        {/if}

        <label class="field" title={m.tooltip_name()}>
          <span class="field-label">{m.init_repo_name_label()}</span>
          <input
            type="text"
            bind:value={name}
            pattern="[A-Za-z0-9._-]+"
            maxlength="100"
            required
          />
        </label>

        <div class="field">
          <span class="field-label">{m.init_repo_visibility_label()}</span>
          <label class="radio" title={m.tooltip_visibility_public()}>
            <input
              type="radio"
              name="visibility"
              value={false}
              bind:group={isPrivate}
            />
            <span>{m.init_repo_visibility_public()}</span>
          </label>
          <label class="radio" title={m.tooltip_visibility_private()}>
            <input
              type="radio"
              name="visibility"
              value={true}
              bind:group={isPrivate}
            />
            <span>{m.init_repo_visibility_private()}</span>
          </label>
        </div>
      {:else if remoteMode === "existing"}
        <label class="field" title={m.tooltip_remote_url()}>
          <span class="field-label">{m.init_repo_url_label()}</span>
          <input
            type="text"
            bind:value={remoteUrl}
            placeholder="https://github.com/me/repo.git"
            required
          />
        </label>
        {#if remoteUrl.trim().length > 0 && !urlLooksValid}
          <div class="hint">{m.init_repo_url_format_hint()}</div>
        {/if}
      {/if}
    {/if}
  </fieldset>

  <fieldset class="section" disabled={inFlight}>
    <label class="checkbox" title={m.tooltip_commit_files()}>
      <input type="checkbox" bind:checked={initialCommit} />
      <span>
        {createRemote
          ? m.init_repo_commit_files_and_push()
          : m.init_repo_commit_files()}
      </span>
    </label>
    {#if initialCommit && count}
      <div class="hint">
        {count.truncated
          ? m.init_repo_files_preview_truncated({ files: count.files })
          : m.init_repo_files_preview({
              files: count.files,
              size: formatBytes(count.bytes),
            })}
      </div>
    {/if}
  </fieldset>

  {#if bannerMessage}
    <div class="banner banner-error">{bannerMessage}</div>
  {/if}
  {#if inFlight && currentStep}
    <div class="banner banner-progress">{currentStep}</div>
  {/if}

  {#snippet footer()}
    <Button onclick={cancel} description={m.tooltip_init_repo_cancel()}>
      {m.init_repo_cancel()}
    </Button>
    <Button
      variant="primary"
      loading={inFlight}
      disabled={!remoteValid}
      onclick={submit}
      description={primaryTooltip}
    >
      {primary}
    </Button>
  {/snippet}
</Dialog>

<style>
  .dialog-intro {
    margin: 0 0 8px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .section {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 8px;
  }
  .checkbox,
  .radio {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }
  .mode-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .field-label {
    font-size: 11px;
    color: var(--text-secondary);
  }
  .hint {
    font-size: 11px;
    color: var(--text-secondary);
  }
  .banner {
    padding: 8px 12px;
    border-radius: 4px;
    font-size: 12px;
    margin-top: 8px;
  }
  .banner-error {
    background: color-mix(in srgb, var(--accent-red) 15%, transparent);
    color: var(--accent-red);
  }
  .banner-progress {
    background: color-mix(in srgb, var(--accent-blue) 15%, transparent);
    color: var(--accent-blue);
  }
  input[type="text"],
  select {
    padding: 6px 8px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 12px;
  }
</style>

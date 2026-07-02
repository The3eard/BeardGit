<!--
  CloneRepoDialog.svelte — runs the "git clone" flow from the "+" tab-bar
  menu. Wired up like InitRepoDialog (`Dialog` primitive for backdrop /
  focus-trap, store-driven visibility, addToast on success), but the
  pipeline is much smaller: paste a URL, pick a parent folder, hit Clone.

  Submits through the `clone_repo` Tauri command. Errors are typed
  (`{ step: "invalid_url" | "invalid_destination" | "destination_exists"
   | "clone", ... }`) and translated into localised banner text via
  `formatStepError`.
-->
<script lang="ts">
  import { open as openFolderPicker } from "@tauri-apps/plugin-dialog";

  import { cloneDialogOpen, closeCloneDialog } from "$lib/stores/cloneDialog";
  import { cloneRepo } from "$lib/api/tauri";
  import { getErrorCode, getErrorMessage } from "$lib/api/errors";
  import { openProjectTab } from "$lib/stores/projects";
  import { addToast } from "$lib/stores/toast";
  import { Button, Dialog } from "$lib/components/ui";
  import * as m from "$lib/paraglide/messages";

  // ── Local reactive state ─────────────────────────────────────────────
  let url = $state("");
  let parentDir = $state("");
  let inFlight = $state(false);
  let bannerMessage = $state<string | null>(null);
  let currentStep = $state<string | null>(null);

  // ── Derived ──────────────────────────────────────────────────────────
  let open = $derived($cloneDialogOpen);

  const URL_PREFIXES = [
    "https://",
    "http://",
    "ssh://",
    "git://",
    "file://",
    "git@",
    "/",
    "./",
    "../",
  ] as const;

  let trimmedUrl = $derived(url.trim());
  let trimmedParent = $derived(parentDir.trim());
  let urlLooksValid = $derived(
    URL_PREFIXES.some((p) => trimmedUrl.startsWith(p)),
  );
  let canSubmit = $derived(
    urlLooksValid && trimmedParent.length > 0 && !inFlight,
  );

  // Best-effort preview of where the clone will land. Mirrors
  // `derive_repo_name` on the Rust side.
  let previewName = $derived.by(() => {
    if (!urlLooksValid) return "";
    const noSlash = trimmedUrl.replace(/\/+$/, "");
    const path = noSlash.includes(":") && !noSlash.startsWith("/")
      ? noSlash.split(":").pop() ?? ""
      : noSlash;
    const last = path.split("/").pop() ?? "";
    return last.replace(/\.git$/, "");
  });
  let previewPath = $derived(
    previewName && trimmedParent
      ? `${trimmedParent.replace(/\/+$/, "")}/${previewName}`
      : "",
  );

  // ── Effects ──────────────────────────────────────────────────────────
  // Reset transient state every time the dialog opens, so reopening
  // after a failed attempt presents a clean slate.
  $effect(() => {
    if ($cloneDialogOpen) {
      url = "";
      parentDir = "";
      bannerMessage = null;
      currentStep = null;
      inFlight = false;
    }
  });

  // ── Behaviour ────────────────────────────────────────────────────────
  function cancel() {
    closeCloneDialog();
  }

  async function browse() {
    const picked = await openFolderPicker({
      directory: true,
      multiple: false,
      title: m.clone_dialog_destination_label(),
    });
    if (typeof picked === "string") {
      parentDir = picked;
    }
  }

  async function submit() {
    if (!canSubmit) return;
    inFlight = true;
    bannerMessage = null;
    currentStep = m.clone_dialog_step_cloning({ url: trimmedUrl });

    try {
      const out = await cloneRepo({ url: trimmedUrl, parentDir: trimmedParent });
      closeCloneDialog();
      addToast({
        type: "success",
        message: m.clone_dialog_success_toast({
          name: out.name,
          path: out.path,
        }),
      });
      await openProjectTab(out.path);
    } catch (err: unknown) {
      bannerMessage = formatStepError(err);
    } finally {
      inFlight = false;
      currentStep = null;
    }
  }

  function formatStepError(err: unknown): string {
    // `clone_repo` now rejects with an IpcError `{ code, message }`; each
    // clone-pipeline step maps to a stable code. For `destination_exists`
    // the offending path is carried in `message`.
    const message = getErrorMessage(err);
    switch (getErrorCode(err)) {
      case "invalid_url":
        return m.clone_dialog_error_invalid_url({ message });
      case "invalid_destination":
        return m.clone_dialog_error_invalid_destination({ message });
      case "destination_exists":
        return m.clone_dialog_error_destination_exists({ path: message });
      case "clone_failed":
        return m.clone_dialog_error_clone({ message });
      default:
        return message;
    }
  }
</script>

<Dialog {open} onClose={cancel} title={m.clone_dialog_title()} size="md">
  <p class="dialog-intro">{m.clone_dialog_intro()}</p>

  <fieldset class="section" disabled={inFlight}>
    <label class="field" title={m.tooltip_clone_url()}>
      <span class="field-label">{m.clone_dialog_url_label()}</span>
      <input
        type="text"
        bind:value={url}
        placeholder={m.clone_dialog_url_placeholder()}
        autocomplete="off"
        spellcheck="false"
        required
      />
    </label>
    {#if trimmedUrl.length > 0 && !urlLooksValid}
      <div class="hint">{m.clone_dialog_url_format_hint()}</div>
    {/if}

    <label class="field" title={m.tooltip_clone_destination()}>
      <span class="field-label">{m.clone_dialog_destination_label()}</span>
      <div class="picker">
        <input
          type="text"
          bind:value={parentDir}
          placeholder={m.clone_dialog_destination_placeholder()}
          required
        />
        <Button onclick={browse} description={m.tooltip_clone_destination()}>
          {m.clone_dialog_destination_browse()}
        </Button>
      </div>
    </label>
    {#if previewPath}
      <div class="hint">
        {m.clone_dialog_destination_preview({ path: previewPath })}
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
    <Button onclick={cancel} description={m.tooltip_clone_cancel()}>
      {m.init_repo_cancel()}
    </Button>
    <Button
      variant="primary"
      loading={inFlight}
      disabled={!canSubmit}
      onclick={submit}
    >
      {m.clone_dialog_action_clone()}
    </Button>
  {/snippet}
</Dialog>

<style>
  .dialog-intro {
    margin: 0 0 8px;
    font-size: var(--font-size-sm);
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
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .field-label {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
  }
  .picker {
    display: flex;
    gap: 6px;
    align-items: stretch;
  }
  .picker input[type="text"] {
    flex: 1;
  }
  .hint {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    word-break: break-all;
  }
  .banner {
    padding: 8px 12px;
    border-radius: 4px;
    font-size: var(--font-size-sm);
    margin-top: 8px;
  }
  .banner-error {
    background: color-mix(in srgb, var(--accent-red) 15%, transparent);
    color: var(--accent-red);
  }
  .banner-progress {
    background: color-mix(in srgb, var(--accent-primary) 15%, transparent);
    color: var(--accent-primary);
  }
  input[type="text"] {
    padding: 6px 8px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: var(--font-size-sm);
  }
</style>

<!--
  NewRequestDialog.svelte — small modal for creating a new project
  request from the CollectionsTree's "+ New" button.

  The dialog is bound via `open` / `onClose`. Project-only in this
  iteration (Issue 4 walkback): the previous dialog supported both
  project-file and global-DB-row creation; the global path has been
  removed because the sidebar tree no longer surfaces global items
  at all.

  On submit the dialog:
  - composes a minimal `.http` body from the chosen method and URL,
  - writes the file via the existing `requests_save` Tauri command
    under `<project>/.beardgit/requests/<rel-path>` (auto-appending
    `.http` when missing),
  - flips `currentSource` to the newly created item so the editor
    opens immediately, and
  - bumps `treeReloadSignal` so `CollectionsTree` reloads the tree
    without a full panel remount.

  Validation is purely client-side and minimal: empty name and any
  existing tree-leaf path are blocked before the IPC fires.
  Server-side errors surface as inline `error` text rather than
  toasts so the dialog stays self-contained.
-->
<script lang="ts" module>
  /**
   * A pre-existing tree node used only for duplicate-name validation.
   * Mirrors the project tree shape that `CollectionsTree.svelte`
   * exposes (the same backend `TreeNode` shape, narrowed to the
   * fields the dialog actually reads).
   */
  export type ExistingNode = {
    /** `"folder"` or `"file"`. Folders are skipped during validation. */
    kind: string;
    /** Project-relative `rel_path`. */
    rel_path: string;
    /** Display name. */
    name: string;
    /** Children (for folder traversal). */
    children: ExistingNode[];
  };
</script>

<script lang="ts">
  import { requestsSave } from "$lib/api/tauri";
  import { runMutation } from "$lib/api/runMutation";
  import { Button, Dialog, Field } from "$lib/components/ui";
  import { activeProject } from "$lib/stores/projects";
  import { currentSource, treeReloadSignal } from "./stores";

  let {
    open = $bindable(false),
    existingNodes,
    onClose,
  }: {
    /** Two-way bindable open flag. */
    open: boolean;
    /** Existing project tree, used to reject duplicate paths. */
    existingNodes: ExistingNode[];
    /** Called after the dialog closes (Esc, Cancel, or successful create). */
    onClose: () => void;
  } = $props();

  /** HTTP method the user picked from the dropdown. */
  let method = $state<"GET" | "POST" | "PUT" | "PATCH" | "DELETE">("GET");
  /**
   * Request name (becomes the file name on disk). Plain `myreq`
   * lands at `myreq.http`; `folder/myreq` nests under `folder/`. The
   * `.http` extension is appended automatically when missing — see
   * [`normaliseProjectPath`].
   */
  let name = $state("");
  /** Optional URL prefilled into the request line. */
  let url = $state("");
  /** Inline error message; nulled when input changes. */
  let error = $state<string | null>(null);
  /** True while the IPC is in flight; disables submit/cancel. */
  let busy = $state(false);

  /** Reset internal state every time the dialog re-opens. */
  $effect(() => {
    if (open) {
      method = "GET";
      name = "";
      url = "";
      error = null;
      busy = false;
    }
  });

  /** Project-relative path of the active project, empty when none. */
  let projectPath = $derived($activeProject?.path ?? "");

  /**
   * Walk the existing tree and gather every leaf's `rel_path` for the
   * duplicate check. Folders are not paths the user can collide with.
   */
  function collectLeafPaths(nodes: ExistingNode[]): Set<string> {
    const out = new Set<string>();
    function walk(list: ExistingNode[]) {
      for (const n of list) {
        if (n.kind === "file") {
          out.add(n.rel_path);
        }
        if (n.children?.length) walk(n.children);
      }
    }
    walk(nodes);
    return out;
  }

  /**
   * Ensure the typed name maps onto a `.http` file. Auto-appends the
   * extension when missing so users can type `users/get` and the file
   * lands at `users/get.http`.
   */
  function normaliseProjectPath(input: string): string {
    const trimmed = input.trim();
    if (!trimmed) return "";
    return trimmed.toLowerCase().endsWith(".http") ? trimmed : `${trimmed}.http`;
  }

  /**
   * Build the canonical minimal `.http` body the editor will round-trip
   * through `requests_save` / `requests_load`. Mirrors the shape used
   * by the seeded forge packs so the in-app editor sees the same input
   * format every other code path produces.
   */
  function composeHttpBody(displayName: string): string {
    const target = url.trim() || "about:blank";
    return `# @name ${displayName}\n${method} ${target}\n`;
  }

  /**
   * Reject names that would either escape the requests directory or
   * produce an invalid filename on common platforms. The list mirrors
   * Win32's reserved characters plus `..` traversal and any control
   * char — anything that survives this check is safe to hand to the
   * filesystem on macOS, Linux, and Windows.
   */
  function validateName(input: string): string | null {
    if (!input) return "Name is required";
    if (input.startsWith("/") || input.startsWith("\\")) {
      return "Name cannot start with a slash";
    }
    if (input.includes("..")) {
      return "Name cannot contain '..'";
    }
    // Characters that break filenames on at least one supported OS.
    // Forward slash is allowed (used to nest into folders).
    if (/[\\:*?"<>|]/.test(input)) {
      return "Name contains an invalid character";
    }
    // Reject any control character (tab, newline, etc.) — these would
    // round-trip badly through the filesystem and the editor alike.
    // eslint-disable-next-line no-control-regex
    if (/[\x00-\x1f]/.test(input)) {
      return "Name contains a control character";
    }
    return null;
  }

  /** Form submit handler. */
  async function handleCreate() {
    if (busy) return;
    error = null;

    const trimmed = name.trim();
    const validationError = validateName(trimmed);
    if (validationError) {
      error = validationError;
      return;
    }

    if (!projectPath) {
      error = "No active project";
      return;
    }
    const relPath = normaliseProjectPath(trimmed);
    const existing = collectLeafPaths(existingNodes);
    if (existing.has(relPath)) {
      error = `A request already exists at ${relPath}`;
      return;
    }
    busy = true;
    try {
      await runMutation({
        kind: "requests_save",
        invoke: () =>
          requestsSave("project", relPath, projectPath, composeHttpBody(relPath)),
        successToast: () => `Created ${relPath}`,
        failureToastPrefix: "Create request failed",
      });
      currentSource.set({ kind: "project", path: relPath });
      treeReloadSignal.update((n) => n + 1);
      open = false;
      onClose();
    } catch {
      // runMutation already surfaced the failure toast; keep the dialog open.
    } finally {
      busy = false;
    }
  }

  function cancel() {
    if (busy) return;
    open = false;
    onClose();
  }

  /** Submit on Enter from inside the form. */
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !busy) {
      const target = e.target as HTMLElement | null;
      if (target?.tagName === "TEXTAREA") return;
      e.preventDefault();
      void handleCreate();
    }
  }

  const METHODS: Array<"GET" | "POST" | "PUT" | "PATCH" | "DELETE"> = [
    "GET",
    "POST",
    "PUT",
    "PATCH",
    "DELETE",
  ];
</script>

<Dialog bind:open title="New request" size="sm" onClose={cancel}>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="form" onkeydown={handleKeydown} role="form">
    <Field label="Method">
      <select class="select" bind:value={method} disabled={busy}>
        {#each METHODS as m (m)}
          <option value={m}>{m}</option>
        {/each}
      </select>
    </Field>

    <Field
      label="Name"
      description={`Type "myreq" to save at the root, or "folder/myreq" to nest. The .http extension is added automatically.`}
    >
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="text"
        class="input"
        bind:value={name}
        placeholder="users/get"
        spellcheck="false"
        autocomplete="off"
        autofocus
        disabled={busy}
        data-testid="new-request-name"
      />
    </Field>

    <Field label="URL" description="Optional. Defaults to about:blank.">
      <input
        type="text"
        class="input"
        bind:value={url}
        placeholder="https://api.example.com/things"
        spellcheck="false"
        autocomplete="off"
        disabled={busy}
      />
    </Field>

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="neutral" onclick={cancel} disabled={busy}>Cancel</Button>
    <Button
      variant="primary"
      onclick={handleCreate}
      disabled={busy || name.trim().length === 0}
      testid="new-request-submit"
    >
      Create
    </Button>
  {/snippet}
</Dialog>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .input,
  .select {
    width: 100%;
    padding: 6px 10px;
    font-size: var(--font-size-md);
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    box-sizing: border-box;
  }

  .input:focus,
  .select:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .error {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--accent-red);
  }
</style>

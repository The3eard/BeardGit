<!--
  Onboarding prompt shown when no `currentSource` is selected.

  Two ways to bootstrap a project's `requests/` folder:
    - **Create new request** (primary): opens the same NewRequestDialog
      that `CollectionsTree`'s "+ New" button uses, via the shared
      `newRequestOpen` store. The dialog itself lives in CollectionsTree.
    - Quickstart (secondary): seeds nine `.http` samples against the
      public JSONPlaceholder + httpbin APIs so the user can hit Send
      against a working endpoint immediately.

  The previous "Start empty" path was removed — it just created an
  empty env file, which the backend now does on its own whenever the
  requests folder is read (see `ensure_default_env`). Picking "Create
  new request" + saving the dialog ends up in the same effective state
  with one click less.

  After a successful Quickstart we bump `treeReloadSignal` so the
  CollectionsTree re-fetches and auto-select the natural landing file
  (`quickstart/jsonplaceholder/list-posts.http`).
-->
<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { activeProject } from "$lib/stores/projects";
  import { Button, Card } from "$lib/components/ui";
  import { currentSource, treeReloadSignal, newRequestOpen } from "./stores";

  $: projectPath = $activeProject?.path ?? "";
  let busy = false;
  let error: string | null = null;

  /** Fire the shared `newRequestOpen` store; CollectionsTree owns the dialog. */
  function openNewRequest() {
    if (!projectPath) {
      error = "No active project";
      return;
    }
    error = null;
    newRequestOpen.set(true);
  }

  /** Run the Quickstart seed and auto-select its first file. */
  async function runQuickstart() {
    if (!projectPath) {
      error = "No active project";
      return;
    }
    busy = true;
    error = null;
    try {
      const written = await invoke<string[]>("requests_seed_quickstart", {
        projectPath,
      });
      treeReloadSignal.update((n) => n + 1);
      if (written.length > 0) {
        // Quickstart's first written path is
        // `quickstart/jsonplaceholder/list-posts.http` — the most
        // natural file to hit Send on.
        currentSource.set({ kind: "project", path: written[0] });
      }
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="seed-prompt">
  <Card
    title="No requests yet"
    description="Create a new request to start from scratch, or load the test set to play with public APIs."
  >
    <div class="seed-prompt__actions">
      <Button
        variant="primary"
        size="lg"
        icon={""}
        disabled={!projectPath}
        onclick={openNewRequest}
      >
        Create new request
      </Button>
      <Button
        variant="neutral"
        icon={""}
        loading={busy}
        disabled={!projectPath}
        onclick={runQuickstart}
      >
        Load test set (public APIs)
      </Button>
    </div>

    {#if error}
      <p class="seed-prompt__error" role="alert">{error}</p>
    {/if}
  </Card>
</div>

<style>
  .seed-prompt {
    padding: 24px;
    max-width: 480px;
    margin: 40px auto;
    width: 100%;
    box-sizing: border-box;
  }

  .seed-prompt__actions {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .seed-prompt__error {
    margin: 12px 0 0 0;
    font-size: 12px;
    color: var(--accent-red);
    line-height: 1.45;
  }
</style>

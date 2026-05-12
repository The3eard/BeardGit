<!--
  UrlBar.svelte — Method selector + URL input + Send/Cancel + CopyAsMenu.

  Wires the Send button to the backend `requests_save` (persist the
  in-memory edit) followed by `requests_run` (execute against the
  current env). The backend executor reads `.http` files from disk, so
  the save-before-run step is mandatory.

  Cancel generates a per-run UUID ticket id, threads it into
  `requests_run`, and on click invokes `requests_cancel(ticketId)` so
  the backend `CancellationToken` actually fires and aborts the
  in-flight `reqwest` future. The local `runState = "canceled"` flip
  is preserved so the UI updates immediately even if the cancel IPC
  loses the race against a near-instant response.

  Visuals use the shared `Button` primitive (primary Send, danger
  Cancel) and the canonical `bg-select` / `bg-input` recipes so the
  bar matches every other input row in the app.
-->
<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { get } from "svelte/store";
  import { EditorView } from "@codemirror/view";
  import {
    autocompletion,
    startCompletion,
  } from "@codemirror/autocomplete";
  import { Button } from "$lib/components/ui";
  import {
    currentRequest,
    currentSource,
    currentEnv,
    runState,
    lastResponse,
    lastResponseError,
    requestDocToHttp,
  } from "./stores";
  import { activeProject } from "$lib/stores/projects";
  import CopyAsMenu from "./CopyAsMenu.svelte";
  import MiniCodeInput from "./MiniCodeInput.svelte";
  import { varCompletion } from "./varCompletion";

  /** Project root for the active tab, or empty if none. */
  $: projectPath = $activeProject?.path ?? "";

  /**
   * CodeMirror extensions threaded into the URL `MiniCodeInput`. We
   * mount the same `varCompletion()` source the body editor used to
   * use, plus a programmatic `startCompletion` trigger for `{{` —
   * CodeMirror's default `activateOnTyping` only fires on word chars,
   * so two `{` in a row never auto-pop the popover. The watcher
   * checks the two characters immediately behind the cursor and kicks
   * the completion open via `queueMicrotask` so the doc-change has
   * fully applied before the source reads state.
   */
  const urlExtensions = [
    autocompletion({
      override: [
        varCompletion(
          () => get(activeProject)?.path ?? "",
          () => get(currentEnv),
        ),
      ],
      activateOnTyping: true,
    }),
    EditorView.updateListener.of((u) => {
      if (!u.docChanged) return;
      const head = u.state.selection.main.head;
      const before = u.state.doc.sliceString(Math.max(0, head - 2), head);
      if (before === "{{") {
        queueMicrotask(() => startCompletion(u.view));
      }
    }),
  ];

  /**
   * Local mirror of `currentRequest.url` that `MiniCodeInput` binds
   * to. We pull the URL out of the store snapshot whenever the store
   * changes, and push edits back via `handleUrlChange` (called from
   * a reactive statement that fires only on user-driven mutations of
   * `urlValue`). The single-source-of-truth is the store; `urlValue`
   * is just the bind target.
   *
   * `lastSeenStoreUrl` lets us distinguish a store-driven update of
   * `urlValue` from a user-driven one: when the store changes, we
   * adopt the new value and bump `lastSeenStoreUrl` in the same
   * tick so the user-edit reactive statement sees them equal and
   * doesn't fire a redundant `currentRequest.set()`.
   */
  let urlValue = "";
  let lastSeenStoreUrl = "";
  $: {
    const next = $currentRequest?.url ?? "";
    if (next !== lastSeenStoreUrl) {
      urlValue = next;
      lastSeenStoreUrl = next;
    }
  }
  $: if ($currentRequest && urlValue !== lastSeenStoreUrl) {
    // User-driven edit: push it back into the store and record the
    // value we just wrote so the store-driven reactive above doesn't
    // bounce it back at us.
    lastSeenStoreUrl = urlValue;
    currentRequest.set({ ...$currentRequest, url: urlValue });
  }

  /** Methods available in the dropdown. Custom methods can be added later. */
  const methods = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

  /**
   * Frontend-generated id for the in-flight run, used to address the
   * backend `CancellationToken` from the Cancel button. Cleared on
   * resolve (success / error / canceled) so a stale id can never be
   * reused by a subsequent click.
   */
  let activeTicketId: string | null = null;

  /**
   * Persist the in-memory request to disk (so the backend parser sees
   * the latest edit), then run it against the current environment and
   * push the response into the shared store.
   */
  async function send() {
    if (!$currentSource || !$currentRequest) return;
    runState.set("running");
    lastResponseError.set(null);
    const ticketId = crypto.randomUUID();
    activeTicketId = ticketId;
    try {
      const content = requestDocToHttp($currentRequest);
      await invoke("requests_save", {
        sourceKind: $currentSource.kind,
        sourcePath: $currentSource.path,
        projectPath: projectPath || null,
        content,
      });

      const r = await invoke<{
        history_id: number;
        status: number;
        headers: [string, string][];
        body_base64: string;
        truncated: boolean;
        duration_ms: number;
      }>("requests_run", {
        args: {
          source_kind: $currentSource.kind,
          source_path: $currentSource.path,
          project_path: projectPath || null,
          env_name: $currentEnv,
          overrides: {},
          ticket_id: ticketId,
        },
      });
      lastResponse.set({
        status: r.status,
        headers: r.headers,
        body: Uint8Array.from(atob(r.body_base64), (c) => c.charCodeAt(0)),
        truncated: r.truncated,
        durationMs: r.duration_ms,
      });
      runState.set("done");
    } catch (e) {
      // The backend surfaces a canceled run as the literal string
      // "canceled" (the Display impl of RequestsError::Canceled).
      // Distinguish that from real errors so the UI can show a neutral
      // "canceled" state instead of an error banner.
      const msg = String(e);
      if (msg === "canceled" || msg.includes("canceled")) {
        runState.set("canceled");
      } else {
        lastResponseError.set(msg);
        runState.set("error");
      }
    } finally {
      activeTicketId = null;
    }
  }

  /**
   * Cancel the in-flight run. Flips the UI to `"canceled"` immediately
   * (so the button swaps back even if the IPC is slow) and fires the
   * backend `requests_cancel` to actually abort the network call.
   * Errors from the cancel IPC are swallowed — at worst the run
   * completes naturally and the UI stays in `"canceled"`.
   */
  async function cancel() {
    runState.set("canceled");
    const id = activeTicketId;
    if (id) {
      try {
        await invoke("requests_cancel", { ticketId: id });
      } catch (_) {
        // Best-effort — see fn-doc above.
      }
    }
  }
</script>

<div class="url-bar">
  {#if $currentRequest}
    <select class="bg-select method" bind:value={$currentRequest.method}>
      {#each methods as m}
        <option value={m}>{m}</option>
      {/each}
    </select>
    <MiniCodeInput
      class="url"
      placeholder="https://api.example.com/..."
      extraExtensions={urlExtensions}
      bind:value={urlValue}
    />
  {/if}
  {#if $currentEnv}
    <span class="env-pill" title={`Variables resolve from env "${$currentEnv}"`}>
      env: {$currentEnv}
    </span>
  {/if}
  {#if $runState === "running"}
    <Button variant="danger" size="md" onclick={cancel}>Cancel</Button>
  {:else}
    <Button
      variant="primary"
      size="md"
      icon={""}
      disabled={!$currentRequest}
      onclick={send}
    >
      Send
    </Button>
  {/if}
  <CopyAsMenu />
</div>

<style>
  .url-bar {
    display: flex;
    gap: 6px;
    padding: 8px;
    border-bottom: 1px solid var(--border);
    align-items: center;
  }

  /*
   * Normalize the native select to the same intrinsic height as the
   * `MiniCodeInput` host so they sit on a shared baseline.
   * `appearance: none` strips the OS caret (which adds vertical
   * padding); we paint our own SVG caret as a background image and
   * reserve right-padding for it. The URL field is now a CodeMirror
   * host (`MiniCodeInput`) which carries its own copy of the same
   * recipe — the legacy `.bg-input` rules used to live here too.
   */
  .bg-select {
    height: 30px;
    line-height: 28px; /* 30px - 2px borders */
    padding: 0 10px;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-family: var(--font-mono);
    font-size: 12px;
    outline: none;
    box-sizing: border-box;
    appearance: none;
    -webkit-appearance: none;
    -moz-appearance: none;
    cursor: pointer;
    padding-right: 26px;
    background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='10' height='6' viewBox='0 0 10 6'><path fill='none' stroke='%23888' stroke-width='1.4' stroke-linecap='round' stroke-linejoin='round' d='M1 1l4 4 4-4'/></svg>");
    background-repeat: no-repeat;
    background-position: right 10px center;
  }

  .bg-select:focus {
    border-color: var(--accent-primary);
  }

  .method {
    font-weight: 600;
    min-width: 96px;
  }

  /*
   * The URL input is now a `MiniCodeInput` (scoped CodeMirror host
   * div), no longer a native `<input>`. Target its host via the
   * passthrough `class="url"` we feed the component so it stretches
   * to fill the row alongside the method dropdown.
   */
  .url-bar :global(.url) {
    flex: 1;
  }

  /*
   * Tonal pill that mirrors the response status pills used in the
   * response area. Communicates which env (if any) the resolver will
   * use for {{vars}} / {{$secrets}} when the user hits Send.
   */
  .env-pill {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 8px;
    border-radius: 11px;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 500;
    color: var(--accent-primary);
    background: var(--overlay-accent-blue);
    white-space: nowrap;
  }
</style>

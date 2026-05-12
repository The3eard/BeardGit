<!--
  Xrefs — render text with clickable #N / URL / vX.Y.Z cross-references.

  Strategy:
  - If `render` is provided, it's applied to plain-text segments only
    (e.g. markdown renderer). Cross-ref segments always render as anchor
    elements with click handlers.
  - No renderer → plain text is emitted verbatim, preserving newlines.
-->
<script lang="ts">
  import { parseXrefs, type XrefContext } from "../../utils/xrefs";
  import { mrPrList } from "../../stores/mr-pr";
  import { issueByNumber, loadIssueDetail } from "../../stores/issues";
  import { loadMrPrDetail } from "../../stores/mr-pr";
  import { releaseTagSet, selectRelease } from "../../stores/releases";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { activeViewStore } from "../../stores/navigation";

  interface Props {
    /** Text to render. */
    text: string;
    /** Optional markdown/HTML renderer applied to plain-text segments. */
    render?: (t: string) => string;
  }
  let { text, render }: Props = $props();

  let ctx = $derived<XrefContext>({
    mrPrCache: new Map($mrPrList.map((p) => [p.number, p])),
    issueCache: $issueByNumber,
    releaseTagCache: $releaseTagSet,
    onOpenMrPr: (n) => {
      activeViewStore.set("merge-requests");
      void loadMrPrDetail(n);
    },
    onOpenIssue: (n) => {
      activeViewStore.set("issues");
      void loadIssueDetail(n);
    },
    onOpenRelease: (tag) => {
      activeViewStore.set("releases");
      selectRelease(tag);
    },
    onOpenExternal: (url) => {
      void openUrl(url);
    },
  });

  let segments = $derived(parseXrefs(text, ctx));
</script>

{#each segments as seg, i (i)}
  {#if seg.type === "text"}
    {#if render}
      <!-- eslint-disable-next-line svelte/no-at-html-tags -->
      {@html render(seg.value)}
    {:else}
      {seg.value}
    {/if}
  {:else if seg.type === "mr_pr"}
    <button
      type="button"
      class="xref xref-mr"
      onclick={() => ctx.onOpenMrPr(seg.number)}
    >{seg.display}</button>
  {:else if seg.type === "issue"}
    <button
      type="button"
      class="xref xref-issue"
      onclick={() => ctx.onOpenIssue(seg.number)}
    >{seg.display}</button>
  {:else if seg.type === "release"}
    <button
      type="button"
      class="xref xref-release"
      onclick={() => ctx.onOpenRelease(seg.tag)}
    >{seg.display}</button>
  {:else if seg.type === "external"}
    <a
      class="xref xref-external"
      href={seg.url}
      onclick={(e) => { e.preventDefault(); ctx.onOpenExternal(seg.url); }}
    >{seg.display}</a>
  {/if}
{/each}

<style>
  .xref {
    color: var(--accent-primary);
    text-decoration: none;
    cursor: pointer;
    background: none;
    border: none;
    padding: 0;
    font: inherit;
    display: inline;
  }
  .xref:hover {
    text-decoration: underline;
  }
  .xref-release {
    color: var(--accent-purple);
  }
</style>

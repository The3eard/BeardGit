<!--
  ActiveRow — single row in the "Active terminals" section of the AI
  Sessions view.

  Thin wrapper over `SessionRow`. One `ActiveTerminal` in, one trimmed
  row out. Row shows ONLY provider icon, title, and (for bg runs only)
  relative start-time. Tab/segment rows get an em-dash date because they
  have no single "moment of truth" timestamp worth surfacing in the list.

  All former inline affordances (provider-name span, status badge, cwd
  line, Focus button) moved to `AiSessionDetail.svelte` via the new
  `selectedActiveTerminal` store — see the "active tab/segment branch"
  in that file.
-->
<script lang="ts">
  import type { ActiveTerminal } from "$lib/stores/aiActiveTerminals";
  import {
    selectAiSessionRow,
    selectedActiveTerminal,
  } from "$lib/stores/aiActiveTerminals";
  import { providerName } from "$lib/data/ai-providers";
  import { formatRelativeTimeUnix } from "$lib/utils/time";
  import ProviderIcon from "./ProviderIcon.svelte";
  import SessionRow from "./SessionRow.svelte";

  interface Props {
    active: ActiveTerminal;
  }

  let { active }: Props = $props();

  /** Last path segment for compact display. */
  function shortCwd(fullPath: string): string {
    const parts = fullPath.replace(/\\/g, "/").split("/").filter(Boolean);
    return parts[parts.length - 1] ?? fullPath;
  }

  let provider = $derived(
    active.kind === "bg" ? active.session.provider : active.info.provider!,
  );

  let title = $derived.by(() => {
    if (active.kind === "tab") return `Terminal ${active.tabIndex + 1}`;
    if (active.kind === "segment")
      return `Terminal in ${shortCwd(active.info.cwd)}`;
    return providerName(active.session.provider);
  });

  let date = $derived(
    active.kind === "bg"
      ? formatRelativeTimeUnix(active.session.started_at)
      : null,
  );

  let selected = $derived.by(() => {
    const sel = $selectedActiveTerminal;
    if (!sel) return false;
    if (sel.kind !== active.kind) return false;
    if (active.kind === "bg" && sel.kind === "bg") {
      return sel.session.id === active.session.id;
    }
    if (active.kind === "tab" && sel.kind === "tab") {
      return sel.tabIndex === active.tabIndex;
    }
    if (active.kind === "segment" && sel.kind === "segment") {
      return (
        sel.tabIndex === active.tabIndex &&
        sel.segmentIndex === active.segmentIndex
      );
    }
    return false;
  });

  function onSelect() {
    selectAiSessionRow({ kind: "active", active });
  }
</script>

<div
  data-testid="ai-active-row"
  data-kind={active.kind}
  onclick={onSelect}
  role="presentation"
>
  <SessionRow {title} {date} {selected} {onSelect}>
    {#snippet icon()}
      <ProviderIcon {provider} size={20} />
    {/snippet}
  </SessionRow>
</div>

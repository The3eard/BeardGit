<!--
  ConversationRow — single row in the "Conversations" section of the AI
  Sessions view.

  Thin wrapper over `SessionRow`. Row shows ONLY provider icon, title,
  and relative last-activity. Everything else (provider name, cwd,
  forked-from, Resume action) lives on the detail pane, which picks up
  the click via the `selectedConversationId` store.

  Selection is coordinated with `selectedBackgroundSessionId` and
  `selectedActiveTerminal` via `selectAiSessionRow` — all three are
  mutually exclusive.
-->
<script lang="ts">
  import type { AiConversation } from "$lib/types";
  import { selectedConversationId } from "$lib/stores/aiConversations";
  import { selectAiSessionRow } from "$lib/stores/aiActiveTerminals";
  import { formatRelativeTimeMs } from "$lib/utils/time";
  import * as m from "$lib/paraglide/messages";
  import ProviderIcon from "./ProviderIcon.svelte";
  import SessionRow from "./SessionRow.svelte";

  interface Props {
    conversation: AiConversation;
  }

  let { conversation }: Props = $props();

  let title = $derived(
    conversation.title && conversation.title.trim().length > 0
      ? conversation.title
      : m.ai_sessions_no_title(),
  );

  let date = $derived(formatRelativeTimeMs(conversation.last_activity_at));

  let selected = $derived($selectedConversationId === conversation.id);

  function onSelect() {
    selectAiSessionRow({ kind: "conversation", id: conversation.id });
  }
</script>

<div
  data-testid="ai-conversation-row"
  data-conversation-id={conversation.id}
  onclick={onSelect}
  role="presentation"
>
  <SessionRow {title} {date} {selected} {onSelect}>
    {#snippet icon()}
      <ProviderIcon provider={conversation.provider} size={20} />
    {/snippet}
  </SessionRow>
</div>

<!--
  Test host for SessionRow — wraps the component with a static icon
  snippet so unit tests don't need to import ProviderIcon.
-->
<script lang="ts">
  import SessionRow from "../SessionRow.svelte";

  interface Props {
    title: string;
    date: string | null;
    selected: boolean;
    onSelect: () => void;
  }

  let { title, date, selected, onSelect }: Props = $props();
</script>

<SessionRow {title} {date} {selected} {onSelect}>
  {#snippet icon()}
    <span data-testid="row-icon-stub">ICON</span>
  {/snippet}
</SessionRow>

<script lang="ts">
  import SplitView from "../common/SplitView.svelte";
  import MrPrList from "./MrPrList.svelte";
  import MrPrDetail from "./MrPrDetail.svelte";
  import { refreshMrPrList } from "../../stores/mr-pr";

  interface Props {
    /** Forwarded to MrPrDetail so file-row clicks bubble up to the page. */
    onFileClick?: (path: string) => void;
  }
  let { onFileClick }: Props = $props();
</script>

<SplitView refreshFn={refreshMrPrList}>
  {#snippet left()}<MrPrList />{/snippet}
  {#snippet right()}<MrPrDetail {onFileClick} />{/snippet}
</SplitView>

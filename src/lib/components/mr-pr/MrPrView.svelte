<script lang="ts">
  import SplitView from "../common/SplitView.svelte";
  import MrPrList from "./MrPrList.svelte";
  import MrPrDetail from "./MrPrDetail.svelte";
  import { refreshMrPrList } from "../../stores/mr-pr";
</script>

<SplitView refreshFn={refreshMrPrList}>
  {#snippet left()}<MrPrList />{/snippet}
  {#snippet right()}<MrPrDetail />{/snippet}
</SplitView>

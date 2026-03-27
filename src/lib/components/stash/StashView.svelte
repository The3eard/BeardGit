<script lang="ts">
  import SplitView from "../common/SplitView.svelte";
  import StashList from "./StashList.svelte";
  import StashDetail from "./StashDetail.svelte";
  import { loadStashes } from "../../stores/stashes";
</script>

<SplitView refreshFn={loadStashes}>
  {#snippet left()}<StashList />{/snippet}
  {#snippet right()}<StashDetail />{/snippet}
</SplitView>

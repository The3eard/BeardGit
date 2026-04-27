<script lang="ts">
  import SplitView from "../common/SplitView.svelte";
  import BranchList from "./BranchList.svelte";
  import BranchDetail from "./BranchDetail.svelte";
  import { refreshBranches } from "../../stores/branches";
</script>

<div class="branch-view" data-testid="branch-view">
  <SplitView refreshFn={refreshBranches}>
    {#snippet left()}<BranchList />{/snippet}
    {#snippet right()}<BranchDetail />{/snippet}
  </SplitView>
</div>

<style>
  .branch-view {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
</style>

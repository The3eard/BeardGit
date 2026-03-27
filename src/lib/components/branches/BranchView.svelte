<script lang="ts">
  import SplitView from "../common/SplitView.svelte";
  import BranchList from "./BranchList.svelte";
  import BranchDetail from "./BranchDetail.svelte";
  import { refreshBranches } from "../../stores/branches";
</script>

<SplitView refreshFn={refreshBranches}>
  {#snippet left()}<BranchList />{/snippet}
  {#snippet right()}<BranchDetail />{/snippet}
</SplitView>

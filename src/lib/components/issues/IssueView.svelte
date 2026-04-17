<!--
  IssueView — split container for the issues vertical. Mirrors MrPrView.
-->
<script lang="ts">
  import SplitView from "../common/SplitView.svelte";
  import IssueList from "./IssueList.svelte";
  import IssueDetail from "./IssueDetail.svelte";
  import { refreshIssueList } from "../../stores/issues";
</script>

<SplitView refreshFn={refreshIssueList}>
  {#snippet left()}<IssueList />{/snippet}
  {#snippet right()}<IssueDetail />{/snippet}
</SplitView>

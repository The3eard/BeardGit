<!--
  ReleaseView — split container for the Releases vertical. Left: list of
  releases for the current repo. Right: detail of the selected release.
  Mirrors IssueView / MrPrView pattern.
-->
<script lang="ts">
  import SplitView from "../common/SplitView.svelte";
  import ReleaseList from "./ReleaseList.svelte";
  import ReleaseDetail from "./ReleaseDetail.svelte";
  import { refreshReleases } from "../../stores/releases";
</script>

<SplitView refreshFn={refreshReleases}>
  {#snippet left()}<ReleaseList />{/snippet}
  {#snippet right()}<ReleaseDetail />{/snippet}
</SplitView>

<script lang="ts">
  import SplitView from "../common/SplitView.svelte";
  import TagList from "./TagList.svelte";
  import TagDetail from "./TagDetail.svelte";
  import { refreshTags } from "../../stores/tags";
</script>

<SplitView refreshFn={refreshTags}>
  {#snippet left()}<TagList />{/snippet}
  {#snippet right()}<TagDetail />{/snippet}
</SplitView>

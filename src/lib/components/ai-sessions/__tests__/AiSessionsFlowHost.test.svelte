<!--
  Test host that renders the real `AiSessionList` and `AiSessionDetail`
  side-by-side so integration tests can click a row in one and observe
  the other update through the shared store layer.
-->
<script lang="ts">
  import AiSessionList from "../AiSessionList.svelte";
  import AiSessionDetail from "../AiSessionDetail.svelte";
</script>

<div style="display:flex; gap:8px;">
  <div style="width:240px;">
    <AiSessionList />
  </div>
  <div style="flex:1;">
    <AiSessionDetail />
  </div>
</div>

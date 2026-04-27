<script lang="ts">
  import { toasts } from "../../stores/toast";
  import Toast from "./Toast.svelte";
</script>

<div class="toast-container">
  {#each $toasts as toast (toast.id)}
    <Toast {toast} />
  {/each}
</div>

<style>
  .toast-container {
    position: fixed;
    bottom: 16px;
    right: 16px;
    display: flex;
    flex-direction: column-reverse;
    gap: 8px;
    z-index: 9999;
    pointer-events: none;
  }
  .toast-container :global(.toast) {
    pointer-events: auto;
  }
</style>

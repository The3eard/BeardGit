<!--
  AssetUploadProgress — shows an indeterminate spinner + filename for a
  single in-flight upload task. Subscribes to the tasks store, tracks the
  referenced task's status, and calls onComplete when the task terminates
  (success, failure, or cancellation).
-->
<script lang="ts">
  import { taskById } from "../../stores/taskPanel";
  import type { TaskId } from "../../types";

  interface Props {
    taskId: TaskId;
    fileName: string;
    onComplete?: (ok: boolean) => void;
  }
  let { taskId, fileName, onComplete }: Props = $props();

  let task = $derived($taskById.get(taskId));
  let state = $derived(task?.status.state ?? "running");
  let errorMsg = $derived(
    task?.status.state === "failed"
      ? ((task?.status as { error?: string }).error ?? "")
      : "",
  );

  let notified = false;
  $effect(() => {
    if (
      !notified &&
      (state === "completed" || state === "failed" || state === "cancelled")
    ) {
      notified = true;
      onComplete?.(state === "completed");
    }
  });
</script>

<div class="upload-row" class:failed={state === "failed"}>
  {#if state === "running" || state === "queued"}
    <div class="spinner"></div>
  {:else if state === "completed"}
    <span class="icon nf ok">{"\uF00C"}</span>
  {:else}
    <span class="icon nf err">{"\uF00D"}</span>
  {/if}
  <span class="file">{fileName}</span>
  {#if state === "failed" && errorMsg}
    <span class="err-msg">{errorMsg}</span>
  {/if}
</div>

<style>
  .upload-row {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 4px 8px;
    font-size: 12px;
    border-bottom: 1px solid var(--border);
  }
  .file {
    font-family: var(--font-mono);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .spinner {
    width: 12px;
    height: 12px;
    border: 2px solid var(--border);
    border-top-color: var(--accent-blue);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .icon {
    font-family: var(--font-icons);
    font-size: 12px;
  }
  .ok {
    color: var(--accent-green);
  }
  .err {
    color: var(--accent-red);
  }
  .err-msg {
    color: var(--accent-red);
    font-size: 11px;
  }
</style>

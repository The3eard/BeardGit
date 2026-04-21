<!--
  Test harness for `TasksPopover.integration.test.ts`.

  Mimics how `+page.svelte` wires up the slot and the popover:

    - `TasksSlot` lives in the statusbar and toggles the store on click.
    - `TasksPopover` is mounted unconditionally; its `open` prop is bound
      to `$tasksPopoverOpen` and `onClose` clears the store.

  Having a real Svelte component host both pieces lets us exercise the
  click-bubble path — the regression this test is here to catch.
-->
<script lang="ts">
  import TasksSlot from "$lib/components/layout/statusbar/TasksSlot.svelte";
  import TasksPopover from "$lib/components/tasks/TasksPopover.svelte";
  import {
    tasksPopoverOpen,
    toggleTasksPopover,
    closeTasksPopover,
  } from "$lib/stores/tasksPopover";
</script>

<TasksSlot onOpen={toggleTasksPopover} />
<TasksPopover open={$tasksPopoverOpen} onClose={closeTasksPopover} />

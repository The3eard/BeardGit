/**
 * Clone orchestration.
 *
 * `clone_repo` validates and then hands the clone to the Rust
 * `TaskManager`, returning immediately with a task id. That is what keeps
 * the window responsive during a large clone — but it also means the
 * dialog is gone by the time the repo is ready, so something has to watch
 * the task and open the tab when it lands.
 *
 * That watcher lives here rather than in the dialog component: the dialog
 * unmounts as soon as validation passes, and a subscription owned by an
 * unmounted component is exactly the kind of thing that either leaks or
 * gets torn down at the wrong moment.
 */

import { tasksStore } from "./tasks";
import { openProjectTab } from "./projects";
import { addToast } from "./toast";
import * as m from "$lib/paraglide/messages";
import type { TaskEntry } from "$lib/types/tasks";

/**
 * Watch a `git clone` task to its terminal state, then open the tab (on
 * success) or report the failure (on error).
 *
 * Cancellation is deliberately silent: the user cancelled it, so a toast
 * would only tell them what they just did. Note that a cancelled clone
 * leaves a partial checkout at `path` — `git clone` cleans up after its
 * own failures but not after being killed. Retrying then trips the
 * `destination_exists` check, whose message already tells the user to
 * remove it or pick another folder. We do not delete it for them.
 */
export function watchCloneTask(taskId: number, path: string, name: string): void {
  const id = String(taskId);

  // `subscribe` fires synchronously with the current value, so the task may
  // already be terminal before `unsubscribe` has been assigned. The flag
  // lets that first call hand the teardown back to the tail of this
  // function instead of touching an uninitialised binding.
  let settled = false;
  let unsubscribe: (() => void) | null = null;

  const onTasks = (tasks: TaskEntry[]) => {
    if (settled) return;
    const entry = tasks.find((t) => t.id === id);
    // No entry yet: the `task://update` event has not arrived. Not terminal.
    if (!entry || entry.status === "running") return;

    settled = true;
    unsubscribe?.();

    if (entry.status === "success") {
      addToast({
        type: "success",
        message: m.clone_dialog_success_toast({ name, path }),
      });
      void openProjectTab(path);
    } else if (entry.status === "error") {
      addToast({
        type: "error",
        message: m.clone_dialog_error_clone({
          message: entry.errorMessage ?? "",
        }),
      });
    }
  };

  unsubscribe = tasksStore.subscribe(onTasks);
  if (settled) unsubscribe();
}

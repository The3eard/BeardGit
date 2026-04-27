/**
 * Releases store — list fetching, selection with detail load, CRUD mutations,
 * asset upload tracking via TaskManager, and cache for xrefs cross-linking.
 *
 * Asset uploads are fire-and-forget: `doUploadAsset` returns a TaskId. The
 * caller should subscribe to tasks.ts events for progress; on task completion
 * the detail is refreshed automatically via `refreshSelectedDetail()`.
 *
 * Mirrors the pattern used by `issues.ts` and `mr-pr.ts`.
 */

import { writable, derived, get } from "svelte/store";
import type {
  Release,
  ReleaseDetail,
  TaskId,
  CreateReleaseInput,
  EditReleasePatch,
} from "../types";
import {
  listReleases as apiList,
  getReleaseDetail as apiDetail,
  createRelease as apiCreate,
  editRelease as apiEdit,
  deleteRelease as apiDelete,
  publishRelease as apiPublish,
  uploadReleaseAsset as apiUpload,
  deleteReleaseAsset as apiDeleteAsset,
  createTagAndRelease as apiCreateTagAndRelease,
} from "../api/tauri";
import { runMutation } from "../api/runMutation";
import { fetchListIntoStore } from "../utils/store-helpers";
import { withTimeout } from "../utils/withTimeout";
import { addToast } from "./toast";
import * as m from "$lib/paraglide/messages";

/**
 * Timeout for the release detail fetch. Mirrors the MR/PR detail
 * timeout so slow `gh api releases/tags/...` calls can't strand the
 * detail pane in a permanent loading state.
 */
const RELEASE_DETAIL_TIMEOUT_MS = 15_000;

/** Current list of releases for the active repository. */
export const releases = writable<Release[]>([]);
/** Whether the list is currently loading. */
export const releasesLoading = writable(false);
/** Currently selected release tag (null = nothing selected). */
export const selectedReleaseTag = writable<string | null>(null);
/** Full detail of the selected release. */
export const releaseDetail = writable<ReleaseDetail | null>(null);
/** Whether the detail view is loading. */
export const releaseDetailLoading = writable(false);
/**
 * Last error raised while loading the selected release detail. Null
 * on success or before any load is attempted. `ReleaseDetail.svelte`
 * reads this store via `ForgeDetailShell` to render an inline banner
 * with a Retry affordance.
 */
export const releaseDetailError = writable<string | null>(null);

/** Set of release tags (for `xrefs.ts` cross-linking of `vX.Y.Z` strings). */
export const releaseTagSet = derived(
  releases,
  ($releases) => new Set($releases.map((r) => r.tag)),
);

/** Active upload task IDs keyed by release tag (for UI progress rows). */
export const activeUploads = writable<Map<string, Set<TaskId>>>(new Map());

/** Fetch the releases list (newest 30). Replaces current list. */
export async function refreshReleases(): Promise<void> {
  await fetchListIntoStore(
    releases,
    releasesLoading,
    selectedReleaseTag,
    () => apiList(30),
    [],
    (r) => r.tag,
  );
  if (get(selectedReleaseTag) === null) {
    releaseDetail.set(null);
  }
}

/**
 * Select a release and load its detail.
 *
 * The fetch is raced against a {@link RELEASE_DETAIL_TIMEOUT_MS}
 * timer so a hung IPC call can't leave the detail pane stuck on a
 * spinner. On any failure (including timeout) the error is written
 * to `releaseDetailError` and surfaced via a toast. Every async
 * side-effect is gated on the selected tag still matching the one
 * this call was started for, so rapid-fire clicks don't race.
 */
export function selectRelease(tag: string): void {
  selectedReleaseTag.set(tag);
  releaseDetail.set(null);
  releaseDetailError.set(null);
  releaseDetailLoading.set(true);
  const expected = tag;
  withTimeout(apiDetail(tag), RELEASE_DETAIL_TIMEOUT_MS)
    .then((d) => {
      if (get(selectedReleaseTag) !== expected) return;
      releaseDetail.set(d);
    })
    .catch((err) => {
      if (get(selectedReleaseTag) !== expected) return;
      const msg = err instanceof Error ? err.message : String(err);
      releaseDetail.set(null);
      releaseDetailError.set(msg);
      addToast({
        message: m.release_load_failed({ tag, error: msg }),
        type: "error",
      });
    })
    .finally(() => {
      if (get(selectedReleaseTag) !== expected) return;
      releaseDetailLoading.set(false);
    });
}

/**
 * Force-reload the currently selected detail (called after edits and
 * asset uploads finish). Uses the same 15 s timeout as
 * {@link selectRelease}, but does **not** fire a toast on failure —
 * this is a background refresh, not a user-initiated load, so the
 * error is only written to `releaseDetailError` for the banner to
 * pick up. The user can still manually retry.
 */
export async function refreshSelectedDetail(): Promise<void> {
  const tag = get(selectedReleaseTag);
  if (!tag) return;
  try {
    const d = await withTimeout(apiDetail(tag), RELEASE_DETAIL_TIMEOUT_MS);
    if (get(selectedReleaseTag) === tag) {
      releaseDetail.set(d);
      releaseDetailError.set(null);
    }
  } catch (err) {
    if (get(selectedReleaseTag) !== tag) return;
    const msg = err instanceof Error ? err.message : String(err);
    releaseDetailError.set(msg);
    /* no toast — this is an auto-refresh path. */
  }
}

/** Create a release via the provider CLI and refresh the list. */
export async function doCreateRelease(
  input: CreateReleaseInput,
): Promise<Release> {
  const created = await runMutation({
    kind: "release_create",
    invoke: () => apiCreate(input),
    successToast: (r) => `Created release ${r.tag}`,
    failureToastPrefix: "Release create failed",
  });
  await refreshReleases();
  return created;
}

/** Atomic create-tag + push + create-release. Returns the TaskId. */
export async function doCreateTagAndRelease(
  tag: string,
  sourceRef: string,
  remote: string,
  input: CreateReleaseInput,
): Promise<TaskId> {
  // Long-running task — progress + completion are reported by the
  // Rust-side TaskManager, which already fires its own task entries.
  // Toast policy still runs through runMutation so a provider-CLI
  // failure (e.g. `gh` not installed) surfaces a sticky error.
  return runMutation({
    kind: "release_create_tag_and_release",
    invoke: () => apiCreateTagAndRelease(tag, sourceRef, remote, input),
    failureToastPrefix: "Release create failed",
  });
}

/** Edit a release, then refresh list + detail. */
export async function doEditRelease(
  tag: string,
  patch: EditReleasePatch,
): Promise<void> {
  await runMutation({
    kind: "release_edit",
    invoke: () => apiEdit(tag, patch),
    successToast: () => `Updated release ${tag}`,
    failureToastPrefix: "Release edit failed",
  });
  await refreshReleases();
  await refreshSelectedDetail();
}

/** Delete a release and refresh. Clears selection if it was the deleted tag. */
export async function doDeleteRelease(tag: string): Promise<void> {
  await runMutation({
    kind: "release_delete",
    invoke: () => apiDelete(tag),
    successToast: () => `Deleted release ${tag}`,
    failureToastPrefix: "Release delete failed",
  });
  if (get(selectedReleaseTag) === tag) {
    selectedReleaseTag.set(null);
    releaseDetail.set(null);
  }
  await refreshReleases();
}

/** Publish a draft release (GitHub only), then refresh. */
export async function doPublishRelease(tag: string): Promise<void> {
  await runMutation({
    kind: "release_publish",
    invoke: () => apiPublish(tag),
    successToast: () => `Published ${tag}`,
    failureToastPrefix: "Release publish failed",
    trackAsTask: true,
  });
  await refreshReleases();
  await refreshSelectedDetail();
}

/**
 * Upload an asset to a release. Returns the TaskId immediately; the caller
 * should show progress via `<AssetUploadProgress>` and call `completeUpload`
 * when the task finishes to clean up the active-uploads map.
 */
export async function doUploadAsset(
  tag: string,
  assetPath: string,
  label?: string,
): Promise<TaskId> {
  const id = await apiUpload(tag, assetPath, label);
  activeUploads.update((map) => {
    const next = new Map(map);
    const set = new Set(next.get(tag) ?? []);
    set.add(id);
    next.set(tag, set);
    return next;
  });
  return id;
}

/** Remove an upload task from the active-uploads map. */
export function completeUpload(tag: string, taskId: TaskId): void {
  activeUploads.update((map) => {
    const next = new Map(map);
    const set = new Set(next.get(tag) ?? []);
    set.delete(taskId);
    if (set.size === 0) {
      next.delete(tag);
    } else {
      next.set(tag, set);
    }
    return next;
  });
}

/** Delete a release asset and refresh the detail view. */
export async function doDeleteAsset(
  tag: string,
  assetId: number,
): Promise<void> {
  await runMutation({
    kind: "release_asset_delete",
    invoke: () => apiDeleteAsset(tag, assetId),
    successToast: () => "Asset deleted",
    failureToastPrefix: "Asset delete failed",
  });
  await refreshSelectedDetail();
}

/** Reset all release state (on project switch). */
export function clearReleaseState(): void {
  releases.set([]);
  selectedReleaseTag.set(null);
  releaseDetail.set(null);
  releaseDetailLoading.set(false);
  releaseDetailError.set(null);
  activeUploads.set(new Map());
}

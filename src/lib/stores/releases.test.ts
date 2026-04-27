/**
 * Release store unit tests. Mocks `api/tauri.ts` so no IPC is required.
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";

vi.mock("../api/tauri", () => ({
  listReleases: vi.fn(),
  getReleaseDetail: vi.fn(),
  createRelease: vi.fn(),
  editRelease: vi.fn(),
  deleteRelease: vi.fn(),
  publishRelease: vi.fn(),
  uploadReleaseAsset: vi.fn(),
  deleteReleaseAsset: vi.fn(),
  createTagAndRelease: vi.fn(),
}));

import * as api from "../api/tauri";
import {
  releases,
  refreshReleases,
  releaseTagSet,
  selectedReleaseTag,
  doUploadAsset,
  activeUploads,
  completeUpload,
  doDeleteRelease,
  releaseDetail,
  clearReleaseState,
} from "./releases";

function mkRelease(tag: string) {
  return {
    tag,
    name: tag,
    state: "published" as const,
    author: "a",
    created_at: "",
    published_at: null,
    asset_count: 0,
    url: "",
  };
}

describe("releases store", () => {
  beforeEach(() => {
    releases.set([]);
    selectedReleaseTag.set(null);
    releaseDetail.set(null);
    activeUploads.set(new Map());
    vi.clearAllMocks();
  });

  it("refreshReleases populates the list and drives releaseTagSet", async () => {
    (api.listReleases as ReturnType<typeof vi.fn>).mockResolvedValue([
      mkRelease("v1.0.0"),
      mkRelease("v0.9.0"),
    ]);
    await refreshReleases();
    expect(get(releases)).toHaveLength(2);
    expect(get(releaseTagSet).has("v1.0.0")).toBe(true);
    expect(get(releaseTagSet).has("v0.9.0")).toBe(true);
  });

  it("doUploadAsset records an active upload keyed by tag", async () => {
    (api.uploadReleaseAsset as ReturnType<typeof vi.fn>).mockResolvedValue(42);
    await doUploadAsset("v1.0.0", "/tmp/a.dmg");
    const uploads = get(activeUploads);
    expect(uploads.get("v1.0.0")?.has(42)).toBe(true);
  });

  it("completeUpload removes the task and drops the tag when set is empty", async () => {
    (api.uploadReleaseAsset as ReturnType<typeof vi.fn>).mockResolvedValue(7);
    await doUploadAsset("v1.0.0", "/tmp/a.dmg");
    completeUpload("v1.0.0", 7);
    const uploads = get(activeUploads);
    expect(uploads.has("v1.0.0")).toBe(false);
  });

  it("doDeleteRelease clears selection if it was the deleted tag", async () => {
    releases.set([mkRelease("v1.0.0"), mkRelease("v0.9.0")]);
    selectedReleaseTag.set("v1.0.0");
    releaseDetail.set({
      summary: mkRelease("v1.0.0"),
      body: "",
      assets: [],
    });
    (api.deleteRelease as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);
    (api.listReleases as ReturnType<typeof vi.fn>).mockResolvedValue([
      mkRelease("v0.9.0"),
    ]);
    await doDeleteRelease("v1.0.0");
    expect(get(selectedReleaseTag)).toBeNull();
    expect(get(releaseDetail)).toBeNull();
  });

  it("clearReleaseState resets everything", () => {
    releases.set([mkRelease("v1.0.0")]);
    selectedReleaseTag.set("v1.0.0");
    activeUploads.set(new Map([["v1.0.0", new Set([1, 2])]]));
    clearReleaseState();
    expect(get(releases)).toHaveLength(0);
    expect(get(selectedReleaseTag)).toBeNull();
    expect(get(activeUploads).size).toBe(0);
  });
});

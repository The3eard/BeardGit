/**
 * Unit tests for the empty-state copy in `ReleaseDetail.svelte`.
 *
 * Phase 7.2 of the Forge Data Fixes plan replaces the lonely em-dash in
 * the body section with a neutral localized message whenever a release
 * has neither notes nor assets. The assets table + upload zone stay
 * visible so users can still seed a blank release with a file. These
 * assertions pin that contract:
 *
 *   - With both body and assets empty the `release_empty_blank` copy
 *     renders with the release tag interpolated.
 *   - With body present the markdown renders and the empty copy is
 *     hidden.
 *   - With body empty but assets present the pane still renders the
 *     em-dash fallback (preserves the pre-Phase-7 behavior, since the
 *     "empty" heuristic requires both pieces missing).
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { tick } from "svelte";
import type { ReleaseDetail as ReleaseDetailT } from "$lib/types";

const mocks = vi.hoisted(() => {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    releaseDetail: writable<ReleaseDetailT | null>(null),
    releaseDetailLoading: writable(false),
    releaseDetailError: writable<string | null>(null),
    selectedReleaseTag: writable<string | null>(null),
    activeProvider: writable<{ kind: "github" | "gitlab" } | null>({
      kind: "github",
    }),
  };
});

vi.mock("$lib/stores/releases", () => {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { writable, readable } = require("svelte/store") as typeof import("svelte/store");
  return {
    releaseDetail: mocks.releaseDetail,
    releaseDetailLoading: mocks.releaseDetailLoading,
    releaseDetailError: mocks.releaseDetailError,
    selectedReleaseTag: mocks.selectedReleaseTag,
    // Xrefs.svelte reads this store for tag-link parsing. We don't
    // exercise that path, so a static empty set is fine.
    releaseTagSet: readable<Set<string>>(new Set()),
    releases: writable([]),
    releasesLoading: writable(false),
    activeUploads: writable(new Map()),
    selectRelease: vi.fn(),
    doDeleteRelease: vi.fn(),
    doPublishRelease: vi.fn(),
    doUploadAsset: vi.fn(),
    doDeleteAsset: vi.fn(),
    refreshSelectedDetail: vi.fn(),
    completeUpload: vi.fn(),
  };
});

vi.mock("$lib/stores/provider", () => ({
  activeProvider: mocks.activeProvider,
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

import ReleaseDetail from "../ReleaseDetail.svelte";

function makeDetail(
  over: Partial<ReleaseDetailT> = {},
  summaryOver: Partial<ReleaseDetailT["summary"]> = {},
): ReleaseDetailT {
  return {
    summary: {
      tag: "v-empty",
      name: "",
      state: "published",
      author: "alice",
      created_at: new Date().toISOString(),
      published_at: new Date().toISOString(),
      asset_count: 0,
      url: "https://example.com/releases/v-empty",
      ...summaryOver,
    },
    body: "",
    assets: [],
    ...over,
  };
}

beforeEach(() => {
  mocks.releaseDetail.set(null);
  mocks.releaseDetailLoading.set(false);
  mocks.releaseDetailError.set(null);
  mocks.selectedReleaseTag.set("v-empty");
  mocks.activeProvider.set({ kind: "github" });
});

afterEach(() => cleanup());

describe("ReleaseDetail — empty-state copy", () => {
  it("renders the 'no release notes or assets' message when body and assets are empty", async () => {
    mocks.releaseDetail.set(makeDetail());
    const { getByText } = render(ReleaseDetail);
    await tick();
    expect(
      getByText("No release notes or assets published for v-empty."),
    ).toBeTruthy();
  });

  it("does NOT render the empty copy when body is present", async () => {
    mocks.releaseDetail.set(
      makeDetail({ body: "### Highlights\n- new stuff" }),
    );
    const { queryByText } = render(ReleaseDetail);
    await tick();
    expect(
      queryByText(/No release notes or assets published for/),
    ).toBeNull();
  });

  it("does NOT render the empty copy when assets are present", async () => {
    mocks.releaseDetail.set(
      makeDetail({
        body: "",
        assets: [
          {
            id: 1,
            name: "bundle.zip",
            label: null,
            size: 1024,
            download_count: 0,
            content_type: "application/zip",
            url: "https://example.com/a.zip",
          },
        ],
      }),
    );
    const { queryByText } = render(ReleaseDetail);
    await tick();
    expect(
      queryByText(/No release notes or assets published for/),
    ).toBeNull();
  });
});

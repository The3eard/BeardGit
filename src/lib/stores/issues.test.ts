import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";
import * as api from "../api/tauri";
import {
  issueList,
  issueDetail,
  selectedIssueNumber,
  issueStateFilter,
  refreshIssueList,
  loadIssueDetail,
  clearIssueState,
  issueByNumber,
} from "./issues";

vi.mock("../api/tauri");

beforeEach(() => {
  clearIssueState();
  vi.resetAllMocks();
});

function mkIssue(n: number, partial: Partial<any> = {}): any {
  return {
    number: n,
    title: `issue ${n}`,
    state: "open",
    author: "alice",
    labels: [],
    assignees: [],
    milestone: null,
    comments_count: 0,
    created_at: "",
    updated_at: "",
    url: "",
    ...partial,
  };
}

describe("issues store", () => {
  it("refreshIssueList populates issueList", async () => {
    (api.listIssues as any).mockResolvedValue([mkIssue(1)]);
    await refreshIssueList();
    expect(get(issueList)).toHaveLength(1);
    expect(get(issueList)[0].number).toBe(1);
  });

  it("loadIssueDetail sets selected number and detail", async () => {
    (api.getIssue as any).mockResolvedValue({
      summary: mkIssue(7),
      body: "B",
      comments: [],
    });
    await loadIssueDetail(7);
    expect(get(selectedIssueNumber)).toBe(7);
    expect(get(issueDetail)?.body).toBe("B");
  });

  it("loadIssueDetail clears detail on error", async () => {
    (api.getIssue as any).mockRejectedValue(new Error("boom"));
    await loadIssueDetail(9);
    expect(get(issueDetail)).toBeNull();
  });

  it("clearIssueState resets everything", () => {
    issueList.set([mkIssue(1)]);
    selectedIssueNumber.set(5);
    clearIssueState();
    expect(get(issueList)).toEqual([]);
    expect(get(selectedIssueNumber)).toBeNull();
    expect(get(issueStateFilter)).toBe("open");
  });

  it("refreshIssueList honors issueStateFilter", async () => {
    (api.listIssues as any).mockResolvedValue([]);
    issueStateFilter.set("closed");
    await refreshIssueList();
    expect(api.listIssues).toHaveBeenCalledWith(
      "closed",
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      50,
    );
  });

  it("refreshIssueList passes undefined state when filter is 'all'", async () => {
    (api.listIssues as any).mockResolvedValue([]);
    issueStateFilter.set("all");
    await refreshIssueList();
    expect(api.listIssues).toHaveBeenCalledWith(
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      50,
    );
  });

  it("refreshIssueList clears selection when selected issue vanishes", async () => {
    selectedIssueNumber.set(3);
    (api.listIssues as any).mockResolvedValue([mkIssue(1)]);
    await refreshIssueList();
    expect(get(selectedIssueNumber)).toBeNull();
  });

  it("issueByNumber derived store maps numbers to issues", async () => {
    (api.listIssues as any).mockResolvedValue([mkIssue(1), mkIssue(2)]);
    await refreshIssueList();
    const map = get(issueByNumber);
    expect(map.get(1)?.number).toBe(1);
    expect(map.get(2)?.number).toBe(2);
    expect(map.get(99)).toBeUndefined();
  });
});

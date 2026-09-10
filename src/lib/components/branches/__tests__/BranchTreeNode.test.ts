/**
 * The star toggle on a branch row.
 *
 * Two things could break invisibly here. The star must report the branch's
 * favorite key rather than its ref, or a local branch and its remote stop
 * sharing one star. And its click must not reach the row, or starring a
 * branch also selects it — which loads 30 commits and swaps the detail pane
 * for a branch the user never asked to open.
 */
import { describe, it, expect, vi, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import BranchTreeNode from "../BranchTreeNode.svelte";
import type { BranchTreeNode as TreeNode } from "../branch-tree";

function leaf(over: Partial<TreeNode> = {}): TreeNode {
  return {
    name: "develop",
    fullPath: "develop",
    isFolder: false,
    isHead: false,
    isRemote: false,
    oid: "abc1234",
    ahead: 0,
    behind: 0,
    upstreamGone: false,
    isFavorite: false,
    favoriteKey: "develop",
    children: [],
    ...over,
  };
}

function mount(node: TreeNode) {
  const onSelect = vi.fn();
  const onToggleFavorite = vi.fn();
  const view = render(BranchTreeNode, {
    props: { node, depth: 0, selected: null, onSelect, onContext: vi.fn(), onToggleFavorite },
  });
  return { ...view, onSelect, onToggleFavorite };
}

afterEach(() => cleanup());

describe("star toggle", () => {
  it("reports the node and does not select the branch", async () => {
    const node = leaf();
    const { getByTestId, onSelect, onToggleFavorite } = mount(node);

    await fireEvent.click(getByTestId("branch-fav-develop"));

    expect(onToggleFavorite).toHaveBeenCalledWith(node);
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("carries the branch key for a remote row, not the ref", async () => {
    const node = leaf({
      name: "develop",
      fullPath: "origin/develop",
      isRemote: true,
      favoriteKey: "develop",
    });
    const { getByTestId, onToggleFavorite } = mount(node);

    await fireEvent.click(getByTestId("branch-fav-origin-develop"));

    expect(onToggleFavorite.mock.calls[0][0].favoriteKey).toBe("develop");
  });

  it("labels itself by what the click will do", () => {
    expect(mount(leaf()).getByTestId("branch-fav-develop").getAttribute("aria-label")).toBe(
      "Add to favorites",
    );
    cleanup();
    expect(
      mount(leaf({ isFavorite: true })).getByTestId("branch-fav-develop").getAttribute("aria-label"),
    ).toBe("Remove from favorites");
  });

  it("marks a starred row as pressed so the state is not colour-only", () => {
    const { getByTestId } = mount(leaf({ isFavorite: true }));

    const star = getByTestId("branch-fav-develop");
    expect(star.getAttribute("aria-pressed")).toBe("true");
    expect(star.classList.contains("is-favorite")).toBe(true);
  });

  it("still selects the branch when the row itself is clicked", async () => {
    const { getByTestId, onSelect } = mount(leaf());

    await fireEvent.click(getByTestId("branch-row-develop"));

    expect(onSelect).toHaveBeenCalledWith("develop");
  });
});

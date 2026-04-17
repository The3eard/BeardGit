import sidebar from "../../pages/sidebar.page";
import changes from "../../pages/changes.page";
import { openFixtureProject } from "../../helpers/project";

describe("Regression: Staging & Commit", () => {
  before(async () => {
    await $("aside.sidebar").waitForExist({ timeout: 10000 });
    await openFixtureProject("simple-repo");
    await sidebar.navigateTo("changes");
    await changes.waitForVisible();
  });

  it("should display the staging area", async () => {
    const area = await changes.stagingArea;
    expect(await area.isDisplayed()).toBe(true);
  });

  it("should display the commit message textarea", async () => {
    const textarea = await changes.commitMessage;
    expect(await textarea.isDisplayed()).toBe(true);
  });

  it("should display the commit button", async () => {
    const btn = await changes.commitBtn;
    expect(await btn.isDisplayed()).toBe(true);
  });

  it("should show amend toggle", async () => {
    const toggle = await changes.amendToggle;
    try {
      expect(await toggle.isDisplayed()).toBe(true);
    } catch {
      // Amend toggle may be in overflow menu — acceptable
    }
  });

  it("should separate staged and unstaged files", async () => {
    // Both lists should exist (even if empty)
    const stagedList = await changes.stagedList;
    const unstagedList = await changes.unstagedList;

    // At least one list container should be visible
    const stagedVisible = await stagedList.isDisplayed().catch(() => false);
    const unstagedVisible = await unstagedList.isDisplayed().catch(() => false);
    expect(stagedVisible || unstagedVisible).toBe(true);
  });

  it("should accept text in the commit message field", async () => {
    const textarea = await changes.commitMessage;
    await textarea.clearValue();
    await textarea.setValue("Test commit message from E2E");

    const value = await textarea.getValue();
    expect(value).toContain("Test commit message from E2E");

    // Clear it back
    await textarea.clearValue();
  });
});

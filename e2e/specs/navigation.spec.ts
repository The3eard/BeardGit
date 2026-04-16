import sidebar from "../pages/sidebar.page";

describe("Navigation", () => {
  it("should navigate to branches view", async () => {
    await sidebar.navigateTo("branches");
    const branchList = await $(".branch-list");
    expect(await branchList.isDisplayed()).toBe(true);
  });

  it("should navigate to settings view", async () => {
    await sidebar.navigateTo("settings");
    const settings = await $(".settings-page");
    expect(await settings.isDisplayed()).toBe(true);
  });
});

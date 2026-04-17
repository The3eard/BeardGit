import sidebar from "../pages/sidebar.page";
import settings from "../pages/settings.page";

describe("Navigation", () => {
  before(async () => {
    await $("aside.sidebar").waitForExist({ timeout: 10000 });
  });

  it("should navigate to branches view", async () => {
    await sidebar.navigateTo("branches");
    const activeView = await sidebar.getActiveView();
    expect(activeView).toBe("branches");
  });

  it("should navigate to changes view", async () => {
    await sidebar.navigateTo("changes");
    const activeView = await sidebar.getActiveView();
    expect(activeView).toBe("changes");
  });

  it("should navigate to settings view", async () => {
    await sidebar.navigateTo("settings");
    expect(await settings.isVisible()).toBe(true);
  });

  it("should navigate to bisect view", async () => {
    await sidebar.navigateTo("bisect");
    const activeView = await sidebar.getActiveView();
    expect(activeView).toBe("bisect");
  });

  it("should navigate back to graph view", async () => {
    await sidebar.navigateTo("graph");
    const activeView = await sidebar.getActiveView();
    expect(activeView).toBe("graph");
  });
});

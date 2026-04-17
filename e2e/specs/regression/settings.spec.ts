import sidebar from "../../pages/sidebar.page";
import settings from "../../pages/settings.page";

describe("Regression: Settings", () => {
  before(async () => {
    // Settings page doesn't require a repo open — sidebar is enough.
    await $("aside.sidebar").waitForExist({ timeout: 10000 });
    await sidebar.navigateTo("settings");
    await settings.waitForVisible();
  });

  it("should display the settings page", async () => {
    expect(await settings.isVisible()).toBe(true);
  });

  it("should have content loaded", async () => {
    expect(await settings.hasContent()).toBe(true);
  });

  it("should default to 'connection' section", async () => {
    const active = await settings.getActiveSection();
    expect(active).toBe("connection");
  });

  it("should navigate to 'appearance' section", async () => {
    await settings.navigateToSection("appearance");
    const active = await settings.getActiveSection();
    expect(active).toBe("appearance");
    expect(await settings.hasContent()).toBe(true);
  });

  it("should navigate to 'git-config' section", async () => {
    await settings.navigateToSection("git-config");
    const active = await settings.getActiveSection();
    expect(active).toBe("git-config");
    expect(await settings.hasContent()).toBe(true);
  });

  it("should navigate to 'ai' section", async () => {
    await settings.navigateToSection("ai");
    const active = await settings.getActiveSection();
    expect(active).toBe("ai");
    expect(await settings.hasContent()).toBe(true);
  });

  it("should navigate back to 'connection' section", async () => {
    await settings.navigateToSection("connection");
    const active = await settings.getActiveSection();
    expect(active).toBe("connection");
  });
});

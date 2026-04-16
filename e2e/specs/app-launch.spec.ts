describe("App Launch", () => {
  it("should display the sidebar", async () => {
    const sidebar = await $("aside.sidebar");
    expect(await sidebar.isDisplayed()).toBe(true);
  });

  it("should show navigation items", async () => {
    const graphNav = await $('[data-testid="nav-graph"]');
    expect(await graphNav.isDisplayed()).toBe(true);
  });
});

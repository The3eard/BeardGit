import sidebar from "../pages/sidebar.page";
import graph from "../pages/graph.page";

describe("App Launch", () => {
  it("should display the sidebar", async () => {
    expect(await sidebar.isVisible()).toBe(true);
  });

  it("should show all navigation items", async () => {
    const graphNav = await $('[data-testid="nav-graph"]');
    expect(await graphNav.isDisplayed()).toBe(true);

    const changesNav = await $('[data-testid="nav-changes"]');
    expect(await changesNav.isDisplayed()).toBe(true);

    const branchesNav = await $('[data-testid="nav-branches"]');
    expect(await branchesNav.isDisplayed()).toBe(true);
  });

  it("should default to graph view", async () => {
    const activeView = await sidebar.getActiveView();
    expect(activeView).toBe("graph");
  });

  it("should render the graph canvas on launch", async () => {
    expect(await graph.isVisible()).toBe(true);
  });
});

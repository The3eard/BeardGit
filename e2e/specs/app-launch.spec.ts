import sidebar from "../pages/sidebar.page";
import graph from "../pages/graph.page";
import { openFixtureProject } from "../helpers/project";

describe("App Launch", () => {
  before(async () => {
    // Wait for the Svelte first paint. Assertions that run too early
    // see an empty body and report "not displayed" even though the UI
    // arrives ~500ms later. 2s is generous but keeps CI deterministic.
    await $("aside.sidebar").waitForExist({ timeout: 10000 });
  });

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

  it("should render the graph canvas once a repo is opened", async () => {
    // App launches to a welcome screen with no repo loaded — the graph
    // canvas only renders after a project is opened.
    await openFixtureProject("simple-repo");
    await graph.waitForRender(10000);
    expect(await graph.isVisible()).toBe(true);
  });
});

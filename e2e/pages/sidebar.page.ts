class SidebarPage {
  get graphNav() { return $('[data-testid="nav-graph"]'); }
  get changesNav() { return $('[data-testid="nav-changes"]'); }
  get branchesNav() { return $('[data-testid="nav-branches"]'); }
  get tagsNav() { return $('[data-testid="nav-tags"]'); }
  get stashesNav() { return $('[data-testid="nav-stashes"]'); }
  get worktreesNav() { return $('[data-testid="nav-worktrees"]'); }
  get reflogNav() { return $('[data-testid="nav-reflog"]'); }
  get bisectNav() { return $('[data-testid="nav-bisect"]'); }
  get submodulesNav() { return $('[data-testid="nav-submodules"]'); }
  get aiConfigNav() { return $('[data-testid="nav-ai-config"]'); }
  get aiSessionsNav() { return $('[data-testid="nav-ai-sessions"]'); }
  get settingsNav() { return $('[data-testid="nav-settings"]'); }

  /** Navigate to a view by its sidebar id (e.g., "graph", "changes", "branches"). */
  async navigateTo(view: string) {
    const nav = await $(`[data-testid="nav-${view}"]`);
    await nav.waitForDisplayed({ timeout: 5000 });
    await nav.click();
    // Wait for the target view to render
    await browser.pause(300);
  }

  /** Get the currently active view by checking which nav-item has .active class. */
  async getActiveView(): Promise<string | null> {
    const items = await $$("button.nav-item.active").getElements();
    if (items.length === 0) return null;
    const testId = await items[0].getAttribute("data-testid");
    // data-testid="nav-graph" -> "graph"
    return testId ? testId.replace("nav-", "") : null;
  }

  /** Check that the sidebar is visible. */
  async isVisible(): Promise<boolean> {
    const sidebar = await $("aside.sidebar");
    return sidebar.isDisplayed();
  }
}

export default new SidebarPage();

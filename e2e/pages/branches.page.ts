class BranchesPage {
  get container() { return $('[data-testid="branch-view"]'); }
  get branchList() { return $('[data-testid="branch-list"]'); }
  get createBtn() { return $('[data-testid="branch-create-btn"]'); }
  get filterInput() { return $('[data-testid="branch-filter"]'); }

  /** Wait for the branches view to be visible. */
  async waitForVisible(timeout = 5000): Promise<void> {
    const container = await this.container;
    await container.waitForDisplayed({ timeout });
  }

  /** Get all visible branch names from the list. */
  async getBranches(): Promise<string[]> {
    const rows = await $$('[data-testid^="branch-row-"]');
    const names: string[] = [];
    for (const row of rows) {
      const testId = await row.getAttribute("data-testid");
      if (testId) {
        // data-testid="branch-row-feature-auth" -> "feature/auth"
        names.push(testId.replace("branch-row-", "").replace(/-/g, "/"));
      }
    }
    return names;
  }

  /** Click the create branch button. Caller must handle the dialog that appears. */
  async clickCreate(): Promise<void> {
    const btn = await this.createBtn;
    await btn.waitForClickable({ timeout: 3000 });
    await btn.click();
  }

  /** Checkout a branch by clicking its row. */
  async checkout(branchName: string): Promise<void> {
    const sanitized = branchName.replace(/\//g, "-");
    const row = await $(`[data-testid="branch-row-${sanitized}"]`);
    await row.waitForDisplayed({ timeout: 5000 });
    await row.doubleClick();
    // Wait for checkout operation
    await browser.pause(1000);
  }

  /** Right-click a branch to open context menu. */
  async openContextMenu(branchName: string): Promise<void> {
    const sanitized = branchName.replace(/\//g, "-");
    const row = await $(`[data-testid="branch-row-${sanitized}"]`);
    await row.waitForDisplayed({ timeout: 5000 });
    await row.click({ button: "right" });
  }

  /** Filter branches by typing into the filter input. */
  async filter(query: string): Promise<void> {
    const input = await this.filterInput;
    await input.waitForDisplayed({ timeout: 3000 });
    await input.clearValue();
    await input.setValue(query);
    // Debounce
    await browser.pause(200);
  }
}

export default new BranchesPage();

class SettingsPage {
  get container() { return $('[data-testid="settings-page"]'); }
  get content() { return $('[data-testid="settings-content"]'); }

  /** Wait for the settings page to be visible. */
  async waitForVisible(timeout = 5000): Promise<void> {
    const container = await this.container;
    await container.waitForDisplayed({ timeout });
  }

  /** Check if the settings page is displayed. */
  async isVisible(): Promise<boolean> {
    try {
      const container = await this.container;
      return await container.isDisplayed();
    } catch {
      return false;
    }
  }

  /** Navigate to a specific settings section by clicking its nav item. */
  async navigateToSection(sectionId: string): Promise<void> {
    const navItem = await $(`[data-testid="settings-nav-${sectionId}"]`);
    await navItem.waitForClickable({ timeout: 3000 });
    await navItem.click();
    await browser.pause(300);
  }

  /** Get the currently active settings section by checking active nav class. */
  async getActiveSection(): Promise<string | null> {
    const items = await $$("button.settings-nav-item.active").getElements();
    if (items.length === 0) return null;
    const testId = await items[0].getAttribute("data-testid");
    return testId ? testId.replace("settings-nav-", "") : null;
  }

  /** Check that the settings content area has rendered child content. */
  async hasContent(): Promise<boolean> {
    const content = await this.content;
    const children = await content.$$("*").getElements();
    return children.length > 0;
  }
}

export default new SettingsPage();

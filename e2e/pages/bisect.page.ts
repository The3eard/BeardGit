class BisectPage {
  get container() { return $('[data-testid="bisect-view"]'); }
  get badInput() { return $('[data-testid="bisect-bad-input"]'); }
  get goodInput() { return $('[data-testid="bisect-good-input"]'); }
  get startBtn() { return $('[data-testid="bisect-start-btn"]'); }
  get goodBtn() { return $('[data-testid="bisect-good-btn"]'); }
  get badBtn() { return $('[data-testid="bisect-bad-btn"]'); }
  get resetBtn() { return $('[data-testid="bisect-reset-btn"]'); }

  /** Wait for the bisect view to be visible. */
  async waitForVisible(timeout = 5000): Promise<void> {
    const container = await this.container;
    await container.waitForDisplayed({ timeout });
  }

  /** Start a bisect session with optional bad/good commits. */
  async startBisect(badCommit?: string, goodCommit?: string): Promise<void> {
    if (badCommit) {
      const badInput = await this.badInput;
      await badInput.waitForDisplayed({ timeout: 3000 });
      await badInput.clearValue();
      await badInput.setValue(badCommit);
    }

    if (goodCommit) {
      const goodInput = await this.goodInput;
      await goodInput.waitForDisplayed({ timeout: 3000 });
      await goodInput.clearValue();
      await goodInput.setValue(goodCommit);
    }

    const btn = await this.startBtn;
    await btn.waitForClickable({ timeout: 3000 });
    await btn.click();
    // Wait for bisect to initialize
    await browser.pause(1000);
  }

  /** Mark the current commit as good. */
  async markGood(): Promise<void> {
    const btn = await this.goodBtn;
    await btn.waitForClickable({ timeout: 3000 });
    await btn.click();
    await browser.pause(500);
  }

  /** Mark the current commit as bad. */
  async markBad(): Promise<void> {
    const btn = await this.badBtn;
    await btn.waitForClickable({ timeout: 3000 });
    await btn.click();
    await browser.pause(500);
  }

  /** Reset the bisect session. */
  async resetBisect(): Promise<void> {
    const btn = await this.resetBtn;
    await btn.waitForClickable({ timeout: 3000 });
    await btn.click();
    await browser.pause(500);
  }

  /** Check if the bisect view is displayed. */
  async isVisible(): Promise<boolean> {
    try {
      const container = await this.container;
      return await container.isDisplayed();
    } catch {
      return false;
    }
  }
}

export default new BisectPage();

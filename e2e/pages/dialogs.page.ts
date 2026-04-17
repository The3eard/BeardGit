class DialogsPage {
  get confirmDialog() { return $('[data-testid="dialog-confirm"]'); }
  get errorDialog() { return $('[data-testid="dialog-error"]'); }
  get dialogTitle() { return $('[data-testid="dialog-title"]'); }
  get confirmBtn() { return $('[data-testid="dialog-confirm-btn"]'); }
  get cancelBtn() { return $('[data-testid="dialog-cancel-btn"]'); }
  get dismissBtn() { return $('[data-testid="dialog-dismiss-btn"]'); }

  /** Check if any dialog is currently visible. */
  async isVisible(): Promise<boolean> {
    try {
      const confirm = await this.confirmDialog;
      if (await confirm.isDisplayed()) return true;
    } catch { /* not found */ }

    try {
      const error = await this.errorDialog;
      if (await error.isDisplayed()) return true;
    } catch { /* not found */ }

    return false;
  }

  /** Get the title text of the currently visible dialog. */
  async getDialogTitle(): Promise<string> {
    const title = await this.dialogTitle;
    await title.waitForDisplayed({ timeout: 3000 });
    return title.getText();
  }

  /** Click the confirm/OK button in a confirmation dialog. */
  async confirm(): Promise<void> {
    const btn = await this.confirmBtn;
    await btn.waitForClickable({ timeout: 3000 });
    await btn.click();
    await browser.pause(300);
  }

  /** Click the cancel button in a confirmation dialog. */
  async cancel(): Promise<void> {
    const btn = await this.cancelBtn;
    await btn.waitForClickable({ timeout: 3000 });
    await btn.click();
    await browser.pause(300);
  }

  /** Dismiss an error dialog. */
  async dismiss(): Promise<void> {
    const btn = await this.dismissBtn;
    await btn.waitForClickable({ timeout: 3000 });
    await btn.click();
    await browser.pause(300);
  }

  /** Wait for a dialog to appear. */
  async waitForDialog(timeout = 5000): Promise<void> {
    await browser.waitUntil(
      async () => this.isVisible(),
      { timeout, timeoutMsg: "Dialog did not appear within timeout" }
    );
  }

  /** Wait for all dialogs to disappear. */
  async waitForDismissed(timeout = 5000): Promise<void> {
    await browser.waitUntil(
      async () => !(await this.isVisible()),
      { timeout, timeoutMsg: "Dialog did not disappear within timeout" }
    );
  }
}

export default new DialogsPage();

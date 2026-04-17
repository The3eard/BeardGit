class ChangesPage {
  get stagingArea() { return $('[data-testid="staging-area"]'); }
  get commitMessage() { return $('[data-testid="commit-message"]'); }
  get commitBtn() { return $('[data-testid="commit-btn"]'); }
  get amendToggle() { return $('[data-testid="amend-toggle"]'); }
  get stagedList() { return $('[data-testid="changes-list-staged"]'); }
  get unstagedList() { return $('[data-testid="changes-list-unstaged"]'); }

  /** Wait for the changes/staging view to be visible. */
  async waitForVisible(timeout = 5000): Promise<void> {
    const area = await this.stagingArea;
    await area.waitForDisplayed({ timeout });
  }

  /** Get file paths listed in the staged section. */
  async getStagedFiles(): Promise<string[]> {
    const rows = await $$('[data-testid="changes-list-staged"] [data-testid^="file-row-"]');
    return this.extractFilePaths(rows as unknown as WebdriverIO.Element[]);
  }

  /** Get file paths listed in the unstaged section. */
  async getUnstagedFiles(): Promise<string[]> {
    const rows = await $$('[data-testid="changes-list-unstaged"] [data-testid^="file-row-"]');
    return this.extractFilePaths(rows as unknown as WebdriverIO.Element[]);
  }

  /** Stage a file by clicking its stage button/checkbox. */
  async stageFile(filePath: string): Promise<void> {
    const sanitized = filePath.replace(/\//g, "-");
    const row = await $(`[data-testid="changes-list-unstaged"] [data-testid="file-row-${sanitized}"]`);
    await row.waitForDisplayed({ timeout: 5000 });
    // Double-click or find the stage action within the row
    await row.click();
    await browser.pause(500);
  }

  /** Unstage a file by clicking its unstage button/checkbox. */
  async unstageFile(filePath: string): Promise<void> {
    const sanitized = filePath.replace(/\//g, "-");
    const row = await $(`[data-testid="changes-list-staged"] [data-testid="file-row-${sanitized}"]`);
    await row.waitForDisplayed({ timeout: 5000 });
    await row.click();
    await browser.pause(500);
  }

  /** Write a commit message and click commit. */
  async commitWithMessage(message: string): Promise<void> {
    const textarea = await this.commitMessage;
    await textarea.waitForDisplayed({ timeout: 5000 });
    await textarea.clearValue();
    await textarea.setValue(message);

    const btn = await this.commitBtn;
    await btn.waitForClickable({ timeout: 3000 });
    await btn.click();
    // Wait for commit operation
    await browser.pause(1000);
  }

  /** Toggle the amend checkbox. */
  async toggleAmend(): Promise<void> {
    const toggle = await this.amendToggle;
    await toggle.waitForClickable({ timeout: 3000 });
    await toggle.click();
  }

  private async extractFilePaths(rows: WebdriverIO.Element[]): Promise<string[]> {
    const paths: string[] = [];
    for (const row of rows) {
      const testId = await row.getAttribute("data-testid");
      if (testId) {
        // data-testid="file-row-src-main.ts" -> "src/main.ts"
        paths.push(testId.replace("file-row-", "").replace(/-/g, "/"));
      }
    }
    return paths;
  }
}

export default new ChangesPage();

class GraphPage {
  get container() { return $('[data-testid="graph-container"]'); }
  get canvas() { return $('[data-testid="graph-canvas"]'); }
  get searchInput() { return $('[data-testid="graph-search"]'); }

  /** Wait for the graph canvas to be visible and rendered (non-zero dimensions). */
  async waitForRender(timeout = 10000): Promise<void> {
    const canvas = await this.canvas;
    await canvas.waitForDisplayed({ timeout });
    // Wait for the canvas to have non-zero width (indicates rendering)
    await browser.waitUntil(
      async () => {
        const width = await canvas.getAttribute("width");
        return width !== null && parseInt(width, 10) > 0;
      },
      { timeout, timeoutMsg: "Graph canvas did not render within timeout" }
    );
  }

  /** Click on a commit row at a given y-offset index (0-based, row height = 28px). */
  async selectCommit(index: number): Promise<void> {
    const canvas = await this.canvas;
    const rowHeight = 28;
    const y = rowHeight * index + rowHeight / 2;
    // Click in the middle of the row, offset from left by 200px (message column)
    await canvas.click({ x: 200, y });
  }

  /** Open context menu on a commit row at a given index. */
  async openContextMenu(index: number): Promise<void> {
    const canvas = await this.canvas;
    const rowHeight = 28;
    const y = rowHeight * index + rowHeight / 2;
    // Right-click
    await canvas.click({ button: "right", x: 200, y });
  }

  /** Type a search query into the graph search input. */
  async search(query: string): Promise<void> {
    const input = await this.searchInput;
    await input.waitForDisplayed({ timeout: 5000 });
    await input.clearValue();
    await input.setValue(query);
  }

  /** Check if the graph view is currently visible. */
  async isVisible(): Promise<boolean> {
    try {
      const canvas = await this.canvas;
      return await canvas.isDisplayed();
    } catch {
      return false;
    }
  }
}

export default new GraphPage();

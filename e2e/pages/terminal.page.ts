class TerminalPage {
  get terminalView() { return $('[data-testid="terminal-view"]'); }
  get xtermContainer() { return $(".xterm-screen"); }

  /** Wait for the terminal view to be visible. */
  async waitForVisible(timeout = 10000): Promise<void> {
    const view = await this.terminalView;
    await view.waitForDisplayed({ timeout });
  }

  /** Check if a terminal is currently rendered. */
  async isVisible(): Promise<boolean> {
    try {
      const view = await this.terminalView;
      return await view.isDisplayed();
    } catch {
      return false;
    }
  }

  /**
   * Type a command into the terminal.
   * Uses keyboard actions since xterm.js intercepts input directly.
   */
  async typeCommand(cmd: string): Promise<void> {
    // Click the terminal to focus it
    const container = await this.xtermContainer;
    await container.click();
    await browser.pause(200);

    // Type the command followed by Enter
    await browser.keys(cmd.split(""));
    await browser.keys(["Enter"]);
    // Wait for command execution
    await browser.pause(1000);
  }

  /**
   * Get the visible terminal output text.
   * Reads from xterm.js DOM rows.
   */
  async getOutput(): Promise<string> {
    const rows = await $$(".xterm-rows > div");
    const lines: string[] = [];
    for (const row of rows) {
      const text = await row.getText();
      if (text.trim()) {
        lines.push(text);
      }
    }
    return lines.join("\n");
  }

  /** Open a new terminal tab via keyboard shortcut Cmd+T / Ctrl+T. */
  async openTerminal(): Promise<void> {
    const modifier = process.platform === "darwin" ? "Meta" : "Control";
    await browser.keys([modifier, "t"]);
    await browser.pause(1500); // Wait for terminal to spawn and connect
  }

  /** Close the active terminal tab via keyboard shortcut Cmd+W / Ctrl+W. */
  async closeTerminal(): Promise<void> {
    const modifier = process.platform === "darwin" ? "Meta" : "Control";
    await browser.keys([modifier, "w"]);
    await browser.pause(500);
  }
}

export default new TerminalPage();

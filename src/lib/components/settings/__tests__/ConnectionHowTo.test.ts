/**
 * Unit tests for `ConnectionHowTo.svelte` — Phase 4 of the
 * "PAT unifies CLI auth" plan.
 *
 * Coverage:
 *  - Default mode is `pat` (the recommended, token-unifies-CLI path).
 *  - The fine-grained PAT warning is rendered in the PAT body so users
 *    don't accidentally pick a token shape that breaks `gh`/`glab`.
 *  - The manual-login `<details>` block holds both the insecure
 *    (`echo`) and secure (`read -rs`) templates.
 *  - The "Copy template" button places the literal template on the
 *    clipboard — including the `<YOUR_PAT>` placeholder, never a
 *    real substituted token.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, fireEvent } from "@testing-library/svelte";
import { tick } from "svelte";
import * as m from "$lib/paraglide/messages";

import ConnectionHowTo from "../ConnectionHowTo.svelte";

const writeText = vi.fn();

beforeEach(() => {
  writeText.mockReset();
  // jsdom's navigator.clipboard isn't writable by default; replace it
  // with a spyable stub for the duration of each test.
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText },
  });
});

afterEach(() => {
  cleanup();
});

describe("ConnectionHowTo — default mode + PAT body", () => {
  it("defaults the <select> to `pat`", async () => {
    const { container } = render(ConnectionHowTo);
    // Open the body by clicking the disclosure toggle so we can assert
    // the PAT-specific copy is rendered.
    const toggle = container.querySelector(".toggle") as HTMLButtonElement;
    await fireEvent.click(toggle);
    await tick();

    const select = container.querySelector(
      "#howto-mode",
    ) as HTMLSelectElement;
    expect(select).not.toBeNull();
    expect(select.value).toBe("pat");

    // PAT intro paragraph appears inside the body.
    const intro = container.querySelector('[data-testid="pat-intro"]');
    expect(intro, "PAT intro paragraph should render").not.toBeNull();
  });

  it("renders the Classic/Legacy vs fine-grained token-type warning", async () => {
    const { container, getByText } = render(ConnectionHowTo);
    const toggle = container.querySelector(".toggle") as HTMLButtonElement;
    await fireEvent.click(toggle);
    await tick();

    const callout = container.querySelector(
      '[data-testid="pat-token-type-callout"]',
    );
    expect(callout, "token-type callout should render").not.toBeNull();
    // Spot-check the "fine-grained" phrase — resilient to the exact
    // bold markup a renderer chooses.
    expect(callout!.textContent).toMatch(/fine-grained/i);
    // Link label from the i18n key.
    expect(
      getByText(m.connection_howto_pat_token_type_link_label()),
    ).toBeTruthy();
  });
});

describe("ConnectionHowTo — manual-login templates", () => {
  it("renders a <details> block with both templates inside", async () => {
    const { container, getByText } = render(ConnectionHowTo);
    const toggle = container.querySelector(".toggle") as HTMLButtonElement;
    await fireEvent.click(toggle);
    await tick();

    // The <details> summary uses the manual-title i18n key.
    expect(getByText(m.connection_howto_pat_manual_title())).toBeTruthy();

    const insecure = container.querySelector(
      '[data-testid="manual-insecure-code"]',
    );
    const secure = container.querySelector(
      '[data-testid="manual-secure-code"]',
    );
    expect(insecure, "insecure template code block").not.toBeNull();
    expect(secure, "secure template code block").not.toBeNull();

    // Literal shape checks — placeholder stays `<YOUR_PAT>`, not a
    // substituted token.
    expect(insecure!.textContent).toContain('echo "<YOUR_PAT>"');
    expect(insecure!.textContent).toContain("gh auth login --with-token");
    expect(secure!.textContent).toContain("read -rs -p");
    expect(secure!.textContent).toContain("gh auth login --with-token");
  });

  it("copies the insecure template to the clipboard verbatim", async () => {
    const { container } = render(ConnectionHowTo);
    const toggle = container.querySelector(".toggle") as HTMLButtonElement;
    await fireEvent.click(toggle);
    await tick();

    const button = container.querySelector(
      '[data-testid="copy-template-insecure"]',
    ) as HTMLButtonElement;
    expect(button, "copy-template-insecure button").not.toBeNull();

    await fireEvent.click(button);
    await tick();

    expect(writeText).toHaveBeenCalledTimes(1);
    const payload = writeText.mock.calls[0][0] as string;
    // Literal placeholder survives — no real PAT substitution.
    expect(payload).toContain('echo "<YOUR_PAT>"');
    expect(payload).toContain("gh auth login --with-token --hostname github.com");
    expect(payload).not.toMatch(/ghp_[A-Za-z0-9]+/);
  });

  it("copies the secure template to the clipboard verbatim", async () => {
    const { container } = render(ConnectionHowTo);
    const toggle = container.querySelector(".toggle") as HTMLButtonElement;
    await fireEvent.click(toggle);
    await tick();

    const button = container.querySelector(
      '[data-testid="copy-template-secure"]',
    ) as HTMLButtonElement;
    expect(button, "copy-template-secure button").not.toBeNull();

    await fireEvent.click(button);
    await tick();

    expect(writeText).toHaveBeenCalledTimes(1);
    const payload = writeText.mock.calls[0][0] as string;
    expect(payload).toContain("read -rs -p");
    expect(payload).toContain("gh auth login --with-token --hostname github.com");
  });
});

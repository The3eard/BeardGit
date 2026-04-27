/**
 * Unit tests for `InitRepoDialog.svelte`.
 *
 * Verifies the user-visible contract of the init-repo dialog without
 * touching the Tauri backend:
 *
 * - Renders nothing when there is no open request.
 * - "Create remote" is disabled when no providers are connected.
 * - The provider dropdown only appears when more than one provider is
 *   connected.
 * - The primary button label flips to "Initialize, commit & push" when
 *   both checkboxes (create remote + initial commit) are on.
 * - Submit calls `initRepo` with the camelCase options shape the
 *   `$lib/api/tauri` binding expects (path, initialBranch, initialCommit,
 *   remote.{kind, providerIndex, name, private, pushAfter}).
 * - On a typed `create_remote` failure the dialog stays open and renders
 *   the localised banner message containing the original failure detail.
 *
 * `addToast`, `openProjectTab`, `initRepo`, and `countFolderContents`
 * are all mocked so the tests run hermetically under jsdom.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import { tick } from "svelte";

vi.mock("$lib/api/tauri", () => ({
  initRepo: vi.fn().mockResolvedValue({ web_url: "https://github.com/me/foo" }),
  countFolderContents: vi
    .fn()
    .mockResolvedValue({ files: 3, bytes: 1024, truncated: false }),
}));

vi.mock("$lib/stores/projects", () => ({
  openProjectTab: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("$lib/stores/toast", () => ({
  addToast: vi.fn(),
}));

import { providerStatus } from "$lib/stores/provider";
import {
  requestOpenInitRepoDialog,
  closeInitRepoDialog,
} from "$lib/stores/initRepoDialog";
import InitRepoDialog from "../InitRepoDialog.svelte";
import { initRepo } from "$lib/api/tauri";

afterEach(() => cleanup());

describe("InitRepoDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    closeInitRepoDialog();
    providerStatus.set({ providers: [], active_index: null });
    // Re-arm the default success response after `vi.clearAllMocks()` strips it.
    (initRepo as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      web_url: "https://github.com/me/foo",
    });
  });

  it("renders nothing when no request", () => {
    const { queryByRole } = render(InitRepoDialog);
    expect(queryByRole("dialog")).toBeNull();
  });

  it("disables 'create remote' when zero providers connected", async () => {
    requestOpenInitRepoDialog("/tmp/foo");
    const { findByRole, getByRole } = render(InitRepoDialog);
    await tick();
    // With 0 providers, createRemote defaults to false. Tick the checkbox to
    // reveal the radios, then verify the "Create new on …" radio is disabled.
    const cb = (await findByRole("checkbox", {
      name: /add remote/i,
    })) as HTMLInputElement;
    await fireEvent.click(cb);
    await tick();
    const createRadio = getByRole("radio", {
      name: /create new on/i,
    }) as HTMLInputElement;
    expect(createRadio.disabled).toBe(true);
  });

  it("renders provider dropdown only with >1 providers", async () => {
    providerStatus.set({
      providers: [
        // The component only reads `kind` off providers; cast keeps the
        // fixture small and avoids importing the full `ConnectedProvider`
        // shape.
        { kind: "github" } as never,
        { kind: "gitlab" } as never,
      ],
      active_index: 0,
    });
    requestOpenInitRepoDialog("/tmp/foo");
    const { findByLabelText } = render(InitRepoDialog);
    expect(await findByLabelText(/provider/i)).toBeTruthy();
  });

  it("primary button label flips for both checkboxes on", async () => {
    providerStatus.set({
      providers: [{ kind: "github" } as never],
      active_index: 0,
    });
    requestOpenInitRepoDialog("/tmp/foo");
    const { findByText } = render(InitRepoDialog);
    // Button.description renders as both `title` and the fallback
    // `aria-label`, so the accessible name is the tooltip text. Match on
    // visible text instead.
    expect(
      await findByText(/initialize, commit & push$/i),
    ).toBeTruthy();
  });

  it("submit calls initRepo with correct options", async () => {
    providerStatus.set({
      providers: [{ kind: "github" } as never],
      active_index: 0,
    });
    requestOpenInitRepoDialog("/tmp/foo");
    const { findByText, container } = render(InitRepoDialog);
    // Wait for the visible label to render, then click the primary button.
    await findByText(/initialize, commit & push$/i);
    const btn = container.querySelector<HTMLButtonElement>(
      'button[data-variant="primary"]',
    );
    expect(btn).toBeTruthy();
    await fireEvent.click(btn!);
    expect(initRepo).toHaveBeenCalledWith(
      expect.objectContaining({
        path: "/tmp/foo",
        initialBranch: "main",
        initialCommit: true,
        remote: expect.objectContaining({
          kind: "create",
          providerIndex: 0,
          name: "foo",
          private: true,
          pushAfter: true,
        }),
      }),
    );
  });

  it("keeps dialog open and shows banner on CreateRemote failure", async () => {
    (initRepo as unknown as ReturnType<typeof vi.fn>).mockRejectedValueOnce({
      step: "create_remote",
      provider: "GitHub",
      message: "name already taken",
    });
    providerStatus.set({
      providers: [{ kind: "github" } as never],
      active_index: 0,
    });
    requestOpenInitRepoDialog("/tmp/foo");
    const { findByText, container } = render(InitRepoDialog);
    await findByText(/initialize, commit & push$/i);
    const btn = container.querySelector<HTMLButtonElement>(
      'button[data-variant="primary"]',
    );
    expect(btn).toBeTruthy();
    await fireEvent.click(btn!);
    expect(await findByText(/already taken/i)).toBeTruthy();
  });

  it("defaults to 'create' mode when ≥1 provider connected", async () => {
    providerStatus.set({
      providers: [
        {
          kind: "github",
          username: "x",
          auth_status: { kind: "Authenticated" },
        } as never,
      ],
      active_index: 0,
    });
    requestOpenInitRepoDialog("/tmp/foo");
    const { findByRole } = render(InitRepoDialog);
    const createRadio = (await findByRole("radio", {
      name: /create new on github/i,
    })) as HTMLInputElement;
    expect(createRadio.checked).toBe(true);
  });

  it("defaults to 'existing' mode when 0 providers connected and disables 'create' radio", async () => {
    requestOpenInitRepoDialog("/tmp/foo");
    const { findByRole, getByRole } = render(InitRepoDialog);
    // Default behavior with 0 providers: createRemote = false. Tick the
    // checkbox to reveal the radios.
    const cb = (await findByRole("checkbox", {
      name: /add remote/i,
    })) as HTMLInputElement;
    await fireEvent.click(cb);
    await tick();
    const createRadio = getByRole("radio", {
      name: /create new on/i,
    }) as HTMLInputElement;
    expect(createRadio.disabled).toBe(true);
    const existingRadio = getByRole("radio", {
      name: /use existing remote url/i,
    }) as HTMLInputElement;
    expect(existingRadio.checked).toBe(true);
  });

  it("submit with 'existing' mode passes kind: 'use_existing' and trimmed URL", async () => {
    providerStatus.set({
      providers: [
        {
          kind: "github",
          username: "x",
          auth_status: { kind: "Authenticated" },
        } as never,
      ],
      active_index: 0,
    });
    requestOpenInitRepoDialog("/tmp/foo");
    const { findByRole, container, findByText } = render(InitRepoDialog);
    await tick();
    // Switch radio to "existing"
    const existingRadio = (await findByRole("radio", {
      name: /use existing remote url/i,
    })) as HTMLInputElement;
    await fireEvent.click(existingRadio);
    await tick();
    // Type a URL with surrounding whitespace. The URL <input> sits inside a
    // <label class="field"> with a title attribute that also contains "URL",
    // so `findByLabelText` matches multiple nodes — use the placeholder
    // selector instead.
    const urlInput = container.querySelector<HTMLInputElement>(
      'input[type="text"][placeholder*="github.com"]',
    );
    expect(urlInput).toBeTruthy();
    await fireEvent.input(urlInput!, {
      target: { value: "  https://example.test/me/repo.git  " },
    });
    await tick();
    await findByText(/initialize, commit & push to existing remote/i);
    const btn = container.querySelector<HTMLButtonElement>(
      'button[data-variant="primary"]',
    );
    expect(btn).toBeTruthy();
    await fireEvent.click(btn!);
    expect(initRepo).toHaveBeenCalledWith(
      expect.objectContaining({
        remote: expect.objectContaining({
          kind: "use_existing",
          url: "https://example.test/me/repo.git",
          pushAfter: true,
        }),
      }),
    );
  });

  it("primary button label flips to 'Initialize, commit & push to existing remote' in existing mode", async () => {
    providerStatus.set({
      providers: [
        {
          kind: "github",
          username: "x",
          auth_status: { kind: "Authenticated" },
        } as never,
      ],
      active_index: 0,
    });
    requestOpenInitRepoDialog("/tmp/foo");
    const { findByRole, container, findByText } = render(InitRepoDialog);
    await tick();
    const existingRadio = (await findByRole("radio", {
      name: /use existing remote url/i,
    })) as HTMLInputElement;
    await fireEvent.click(existingRadio);
    const urlInput = container.querySelector<HTMLInputElement>(
      'input[type="text"][placeholder*="github.com"]',
    );
    expect(urlInput).toBeTruthy();
    await fireEvent.input(urlInput!, {
      target: { value: "https://example.test/me/repo.git" },
    });
    await tick();
    expect(
      await findByText(/initialize, commit & push to existing remote/i),
    ).toBeTruthy();
  });

  it("every interactive element exposes a non-empty title attribute", async () => {
    providerStatus.set({
      providers: [
        {
          kind: "github",
          username: "x",
          auth_status: { kind: "Authenticated" },
        } as never,
      ],
      active_index: 0,
    });
    requestOpenInitRepoDialog("/tmp/foo");
    const { container } = render(InitRepoDialog);
    await tick();
    // Every label[title] should have a non-empty value
    const labels = Array.from(
      container.querySelectorAll<HTMLLabelElement>("label[title]"),
    );
    expect(labels.length).toBeGreaterThan(0);
    for (const lbl of labels) {
      expect(lbl.title.trim().length).toBeGreaterThan(0);
    }
    // The primary button gets its tooltip via the Button.description prop,
    // which renders as title=. Confirm it has one.
    const primaryBtn = container.querySelector<HTMLButtonElement>(
      'button[data-variant="primary"]',
    );
    expect(primaryBtn).toBeTruthy();
    expect(primaryBtn!.getAttribute("title")?.length ?? 0).toBeGreaterThan(0);
  });
});

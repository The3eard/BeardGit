/**
 * Behaviours covered:
 *
 *   1. Lines that look like JSON objects/arrays render pretty-printed
 *      across multiple visible rows (Claude Code's `--output-format
 *      stream-json` is the main producer).
 *   2. Plain-text lines (Codex / OpenCode console output, errors)
 *      pass through unchanged.
 *   3. Bare scalars that happen to be valid JSON (a quoted string, a
 *      number) are NOT reformatted — the user sees them as-is rather
 *      than rewrapped in surplus quotes.
 *   4. Copy concatenates the rendered (pretty-printed) text rather
 *      than the original input, so the clipboard matches what's on
 *      screen.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, fireEvent } from "@testing-library/svelte";
import { tick } from "svelte";
import BackgroundRunTranscript from "../BackgroundRunTranscript.svelte";

afterEach(() => cleanup());

describe("BackgroundRunTranscript — JSON pretty-print", () => {
  it("expands a stream-json object onto multiple rendered rows", async () => {
    const line = `{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"hi"}]}}`;
    const { container } = render(BackgroundRunTranscript, {
      props: { lines: [line] },
    });
    await tick();
    // The rendered `<div class="line">` should contain `\n` because
    // JSON.stringify(_, null, 2) inserts them.
    const rendered = container.querySelector(".line")?.textContent ?? "";
    expect(rendered).toContain("\n");
    expect(rendered).toMatch(/^\{\n {2}"type": "assistant"/);
  });

  it("leaves plain-text lines untouched", async () => {
    const line = `error: build failed at step 3`;
    const { container } = render(BackgroundRunTranscript, {
      props: { lines: [line] },
    });
    await tick();
    expect(container.querySelector(".line")?.textContent).toBe(line);
  });

  it("does not reformat bare scalars that happen to parse as JSON", async () => {
    const line = `"hello world"`;
    const { container } = render(BackgroundRunTranscript, {
      props: { lines: [line] },
    });
    await tick();
    // Should pass through verbatim — only object/array lines get
    // expanded by the pretty-printer.
    expect(container.querySelector(".line")?.textContent).toBe(line);
  });

  it("copy puts the pretty-printed text on the clipboard, not the raw line", async () => {
    const writeText = vi.fn((_text: string) => Promise.resolve());
    Object.assign(navigator, { clipboard: { writeText } });

    const json = `{"type":"system","subtype":"hook_started"}`;
    const { container } = render(BackgroundRunTranscript, {
      props: { lines: [json] },
    });
    await tick();
    const btn = container.querySelector(".btn-copy") as HTMLButtonElement;
    await fireEvent.click(btn);
    await tick();

    expect(writeText).toHaveBeenCalledTimes(1);
    const written = writeText.mock.calls[0][0];
    // Pretty-printed output contains a newline; the raw single-line
    // input doesn't.
    expect(written).toContain("\n");
    expect(written).toMatch(/"type": "system"/);
  });
});

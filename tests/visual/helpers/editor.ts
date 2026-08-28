/**
 * Wait for CodeMirror's lazily-loaded grammar to have tokenised the document.
 *
 * `loadLanguageExtension()` fetches the grammar as a separate chunk, on
 * purpose — it is what keeps the initial JS bundle small. So an editor that is
 * *visible* is not yet an editor that is *highlighted*: the document renders in
 * plain text first, and the token spans appear a frame or two later, when the
 * chunk lands and the parser runs.
 *
 * Screenshots taken in that gap are a race, and it loses. Measured on the
 * changes-view diff: zero `tok-` spans immediately after
 * `.staging-diff-editor` became visible, and eight distinct token classes
 * 300 ms later. It normally wins, which is worse than always failing — the
 * `dark-populated-diff` baseline failed once mid-session for no reason
 * connected to the change under test, after a paraglide rebuild invalidated
 * Vite's cache and made the chunk take longer than usual to serve.
 *
 * The positive signal is a token span. CodeMirror's highlight style emits
 * `tok-keyword`, `tok-typeName`, `tok-string` and friends inside
 * `.cm-content`; before the grammar arrives there are none. This asks for the
 * one thing that means "the parser has run", rather than sleeping and hoping.
 *
 * Only use it where the document actually has syntax to highlight. A plain-text
 * file, or one whose extension has no grammar, produces no token spans and this
 * would (correctly) time out.
 */

import { type Page } from "@playwright/test";

/**
 * Resolve once at least one syntax token has been emitted inside `container`.
 *
 * @param container CSS selector for the editor wrapper. Defaults to the
 *   staging diff editor, which is the one the changes-view specs shoot.
 */
export async function waitForSyntaxHighlighted(
  page: Page,
  container = ".staging-diff-editor",
): Promise<void> {
  await page
    .locator(`${container} [class*="tok-"]`)
    .first()
    .waitFor({ state: "attached", timeout: 10_000 });
}

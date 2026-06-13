/**
 * GitHub-Flavored Markdown renderer + allow-list sanitiser used by the
 * release / issue / merge-request detail surfaces.
 *
 * Rendering is delegated to {@link https://marked.js.org/ | marked} v13+
 * in synchronous mode, with `marked-gfm-heading-id` plugged in for stable
 * anchor ids on headings. `marked` handles GFM tables, fenced code blocks,
 * task lists, strikethrough, and autolinks out of the box — features
 * the previous minimal renderer either dropped or garbled.
 *
 * Sanitisation is a small allow-list pass:
 *   1. Event-handler attributes are stripped from every tag.
 *   2. Tags outside {@link ALLOWED_TAGS} are removed (keeping inner text).
 *      `<input>` is gated on `type="checkbox"` — the only variant that
 *      GFM task lists emit.
 *   3. `href` values starting with `javascript:` are wiped.
 *   4. `<a>` tags gain `target="_blank" rel="noopener noreferrer"` so
 *      links open in the system browser rather than hijacking the Tauri
 *      webview.
 *
 * The function is intentionally sync — consumers interpolate the result
 * directly inside `{@html}` blocks, and an async signature would be a
 * breaking change for them.
 */
import { marked } from "marked";
import { gfmHeadingId } from "marked-gfm-heading-id";

// One-time module load: wire the heading-id extension and lock marked to
// sync-GFM mode. `breaks: false` matches GitHub's rendering of release
// notes, where single newlines fold into paragraphs rather than emitting
// a <br>.
marked.use(gfmHeadingId());
marked.setOptions({
  gfm: true,
  breaks: false,
  async: false,
});

/** URL schemes permitted on `href` / `src`. Everything else is neutralised. */
const SAFE_SCHEMES = new Set(["http", "https", "mailto"]);

/**
 * Return the lowercased URL scheme of `raw` (the token before the first `:`),
 * or `""` when the value is scheme-relative, a fragment, or a relative path.
 *
 * Browsers decode HTML entities and ignore embedded control/whitespace chars
 * BEFORE evaluating a URL scheme, so a deny-list on the literal `javascript:`
 * string is bypassable (`java&#115;cript:`, `java&Tab;script:`,
 * `java\tscript:`). We replicate that decode here so the scheme allow-list
 * sees what the browser would.
 */
function urlScheme(raw: string): string {
  const decoded = raw
    .replace(/&#x([0-9a-f]+);?/gi, (_, h: string) =>
      String.fromCodePoint(parseInt(h, 16)),
    )
    .replace(/&#(\d+);?/g, (_, d: string) => String.fromCodePoint(parseInt(d, 10)))
    .replace(/&colon;/gi, ":")
    .replace(/&Tab;|&NewLine;/gi, "")
    // Drop whitespace (tab/newline/CR/space) — the URL spec strips these
    // before scheme evaluation, so `java\tscript:` would otherwise slip past.
    .replace(/\s+/g, "");
  const match = /^([a-z][a-z0-9+.-]*):/i.exec(decoded);
  return match ? match[1].toLowerCase() : "";
}

/** Allowed HTML tags for markdown rendering. Everything else is stripped. */
const ALLOWED_TAGS = new Set([
  "a", "b", "blockquote", "br", "code", "del", "em", "h1", "h2", "h3",
  "h4", "h5", "h6", "hr", "i", "img", "input", "li", "ol", "p", "pre", "s",
  "strong", "sub", "sup", "table", "tbody", "td", "th", "thead", "tr", "ul",
]);

/**
 * Strip disallowed tags + event-handler attributes, gate non-checkbox
 * `<input>`, neutralise `javascript:` URLs, and force links to open in
 * the external browser.
 */
function sanitize(html: string): string {
  // 1. Strip event-handler attributes on every tag (any quote style).
  let clean = html.replace(
    /\s+on\w+\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]+)/gi,
    "",
  );
  // 2. Strip disallowed tags. `<input>` is allowed only for checkboxes.
  clean = clean.replace(
    /<\/?([a-z][a-z0-9]*)\b([^>]*)\/?>/gi,
    (match, tag: string, attrs: string) => {
      const t = tag.toLowerCase();
      if (!ALLOWED_TAGS.has(t)) return "";
      if (t === "input" && !/type\s*=\s*["']?checkbox["']?/i.test(attrs)) {
        return "";
      }
      return match;
    },
  );
  // 3. Allow-list URL schemes on href/src. A deny-list on the literal
  //    `javascript:` string is bypassable via entity-/whitespace-obfuscation
  //    (`java&#115;cript:`, `java&Tab;script:`) that a browser decodes before
  //    evaluating the scheme. We decode (urlScheme) and permit only
  //    http/https/mailto; scheme-relative, fragment, and relative URLs (no
  //    scheme) pass through. Anything else (javascript:, data:, vbscript:, …)
  //    is neutralised to "#".
  clean = clean.replace(
    /\s(href|src)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))/gi,
    (match, attr: string, dq?: string, sq?: string, uq?: string) => {
      const value = dq ?? sq ?? uq ?? "";
      const scheme = urlScheme(value);
      return scheme && !SAFE_SCHEMES.has(scheme) ? ` ${attr}="#"` : match;
    },
  );
  // 4. Force external links to open in the system browser.
  clean = clean.replace(/<a\s/gi, '<a target="_blank" rel="noopener noreferrer" ');
  // 5. Re-skin task-list checkboxes. After step 2 every surviving
  //    `<input>` is a GFM task-list checkbox; native inputs can't be
  //    styled with pseudo-elements, so swap them for a token-themed
  //    display-only span (styles in `lib/styles/markdown.css`) that
  //    mirrors the ui/Checkbox primitive. ARIA keeps the semantics.
  clean = clean.replace(/<input\b([^>]*?)\/?>/gi, (_match, attrs: string) => {
    const checked = /\bchecked\b/i.test(attrs);
    return (
      `<span class="md-task-checkbox${checked ? " md-task-checkbox--checked" : ""}"` +
      ` role="checkbox" aria-checked="${checked}" aria-disabled="true"></span>`
    );
  });
  return clean;
}

/**
 * Parse GitHub-flavored markdown to HTML with allow-list sanitisation.
 * Only allowed tags pass through; event handlers are stripped; links
 * open in the external browser.
 *
 * Returns `""` for empty/falsy input so callers don't need to guard.
 */
export function renderMarkdown(text: string): string {
  if (!text) return "";
  const html = marked.parse(text) as string;
  return sanitize(html);
}

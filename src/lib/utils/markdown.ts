import snarkdown from "snarkdown";

/** Allowed HTML tags for markdown rendering. Everything else is stripped. */
const ALLOWED_TAGS = new Set([
  "a", "b", "blockquote", "br", "code", "del", "em", "h1", "h2", "h3",
  "h4", "h5", "h6", "hr", "i", "img", "li", "ol", "p", "pre", "s",
  "strong", "sub", "sup", "table", "tbody", "td", "th", "thead", "tr", "ul",
]);

/** Strip all tags not in the allowlist and remove event handler attributes. */
function sanitize(html: string): string {
  // Remove event handler attributes (any quote style or unquoted)
  let clean = html.replace(/\s+on\w+\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]+)/gi, "");
  // Strip disallowed tags (keep content, remove the tag)
  clean = clean.replace(/<\/?([a-z][a-z0-9]*)\b[^>]*\/?>/gi, (match, tag: string) => {
    return ALLOWED_TAGS.has(tag.toLowerCase()) ? match : "";
  });
  // Force all links to open externally
  clean = clean.replace(/<a\s/gi, '<a target="_blank" rel="noopener noreferrer" ');
  return clean;
}

/**
 * Parse markdown to HTML with allowlist-based sanitization.
 * Only allowed tags pass through. Event handlers are stripped.
 * Links open in external browser (target="_blank").
 */
export function renderMarkdown(text: string): string {
  if (!text) return "";
  // snarkdown doesn't handle multi-paragraph well — split on double newlines
  const html = text
    .split(/\n{2,}/)
    .map((block) => snarkdown(block.trim()))
    .filter(Boolean)
    .join("\n");
  return sanitize(html);
}

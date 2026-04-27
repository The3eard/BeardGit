/**
 * Forbid raw hex / rgb / rgba / hsl color literals inside Svelte inline
 * `style="..."` attributes and `style:<prop>={...}` bindings. The sweep
 * in docs/superpowers/plans/2026-04-23-theme-color-audit.md routes all
 * UI colors through theme tokens (var(--token)) or the BRAND_COLORS
 * allowlist; this rule blocks regressions.
 *
 * Escape hatch: place a comment containing the magic string on the line(s)
 * immediately above the element:
 *   // beardgit:allow-hex: <short justification>   (inside <script> blocks)
 *   <!-- beardgit:allow-hex: <reason> -->            (in Svelte template markup)
 *
 * Covers:
 *   - <div style="color: #f85149">                          (literal string attr)
 *   - <div style={`color: ${foo ? "#f85149" : "red"}`}>     (template literal inside attr)
 *   - <div style:color={"#f85149"}>                         (style:<prop> directive)
 *   - <div style:background-color="#f85149">                (style:<prop> with string literal value)
 */

"use strict";

const HEX = /#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})\b/;
const FUNCTIONAL = /\b(?:rgb|rgba|hsl|hsla)\s*\(/i;
const ALLOW_MARKER = /beardgit:allow-hex:/;

/**
 * Check whether the source line(s) immediately before `node` contain the
 * escape-hatch marker. Handles both JS-style comments (`//`) and Svelte HTML
 * comments (`<!-- -->`), because style: directives live in the template part
 * of a .svelte file where getCommentsBefore returns nothing.
 */
function hasAllowComment(sourceCode, node) {
  // Fast path: AST comments registered before this node (works for JS contexts).
  const fn = sourceCode.getCommentsBefore?.bind(sourceCode);
  const astComments = fn ? fn(node) : [];
  if (astComments.some((c) => ALLOW_MARKER.test(c.value))) return true;

  // Fallback: scan the raw source lines above the node. We look up to 8 lines
  // to cover cases where the comment precedes the opening element tag and the
  // directive itself is indented several lines into the element's attribute list.
  const startLine = node.loc?.start?.line;
  if (!startLine || startLine <= 1) return false;
  const lines = sourceCode.lines ?? sourceCode.getLines?.() ?? [];
  // Scan upward: stop as soon as we find the marker OR we pass the opening
  // element tag without finding the marker one line above it.
  let passedElementOpen = false;
  for (let l = startLine - 2; l >= Math.max(0, startLine - 9); l--) {
    const line = lines[l] ?? "";
    if (ALLOW_MARKER.test(line)) return true;
    if (passedElementOpen) break; // marker must appear on the line directly above the element
    if (/^\s*<[^/!]/.test(line)) passedElementOpen = true;
  }
  return false;
}

function check(context, node, value) {
  if (typeof value !== "string") return;
  if (!HEX.test(value) && !FUNCTIONAL.test(value)) return;
  // ESLint ≥ 9: context.sourceCode replaces the deprecated context.getSourceCode().
  const src = context.sourceCode ?? context.getSourceCode?.();
  if (src && hasAllowComment(src, node)) return;
  context.report({
    node,
    message:
      "Hardcoded color literal in inline style. Use a theme token (var(--token)) or BRAND_COLORS. " +
      "Add `// beardgit:allow-hex: <reason>` above this line to override.",
  });
}

module.exports = {
  meta: {
    type: "problem",
    docs: { description: "Disallow hardcoded hex/rgb/hsl colors in Svelte inline styles" },
    schema: [],
    messages: {},
  },
  create(context) {
    return {
      // Inline attribute: <el style="...">
      "SvelteAttribute[key.name='style']"(node) {
        for (const v of node.value || []) {
          if (v.type === "SvelteLiteral") check(context, v, v.value);
          else if (v.type === "SvelteMustacheTag" && v.expression.type === "Literal") {
            check(context, v.expression, String(v.expression.value));
          } else if (v.type === "SvelteMustacheTag" && v.expression.type === "TemplateLiteral") {
            for (const q of v.expression.quasis) check(context, q, q.value.cooked);
          }
        }
      },
      // Directive: <el style:color={...} or style:color="#hex">
      "SvelteStyleDirective"(node) {
        for (const v of node.value || []) {
          if (v.type === "SvelteLiteral") check(context, v, v.value);
          else if (v.type === "SvelteMustacheTag" && v.expression.type === "Literal") {
            check(context, v.expression, String(v.expression.value));
          } else if (v.type === "SvelteMustacheTag" && v.expression.type === "TemplateLiteral") {
            for (const q of v.expression.quasis) check(context, q, q.value.cooked);
          }
        }
      },
    };
  },
};

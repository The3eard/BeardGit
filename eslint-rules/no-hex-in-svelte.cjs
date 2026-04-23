/**
 * Forbid raw hex / rgb / rgba / hsl color literals inside Svelte inline
 * `style="..."` attributes and `style:<prop>={...}` bindings. The sweep
 * in docs/superpowers/plans/2026-04-23-theme-color-audit.md routes all
 * UI colors through theme tokens (var(--token)) or the BRAND_COLORS
 * allowlist; this rule blocks regressions.
 *
 * Escape hatch: place a comment on the line above the hit:
 *   // beardgit:allow-hex: <short justification>
 *
 * Covers:
 *   - <div style="color: #f85149">                          (AttributeShorthand / literal)
 *   - <div style={`color: ${foo ? "#f85149" : "red"}`}>     (template literal inside attr)
 *   - <div style:color={"#f85149"}>                         (style:<prop> directive)
 *   - <div style:background-color="#f85149">                (style:<prop> with string literal value)
 */

"use strict";

const HEX = /#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})\b/;
const FUNCTIONAL = /\b(?:rgb|rgba|hsl|hsla)\s*\(/i;

function hasAllowComment(sourceCode, node) {
  const comments = sourceCode.getCommentsBefore ? sourceCode.getCommentsBefore(node) : [];
  return comments.some((c) => /beardgit:allow-hex:/.test(c.value));
}

function check(context, node, value) {
  if (typeof value !== "string") return;
  if (!HEX.test(value) && !FUNCTIONAL.test(value)) return;
  if (hasAllowComment(context.getSourceCode(), node)) return;
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

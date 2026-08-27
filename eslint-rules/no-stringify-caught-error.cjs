/**
 * Forbid `String(err)` — and the `err instanceof Error ? err.message :
 * String(err)` long form — on a value caught from a `catch` clause.
 *
 * Tauri rejects with one of two shapes: a plain string (the legacy
 * `Result<_, String>` commands) or the structured `{ code, message }`
 * envelope that `IpcError` serialises to. `String(x)` reads correctly on
 * the first and produces the literal text `"[object Object]"` on the
 * second — in a toast, in a stored error field, and in three places that
 * compared the result against known text to decide what to do next.
 *
 * That is exactly what happened when the last 250 commands moved to
 * `IpcError`: 97 callsites across 38 files kept compiling, kept passing
 * every test, and started showing `[object Object]`. Nothing in the gate
 * could see it, because both expressions are perfectly valid TypeScript
 * either way.
 *
 * `getErrorMessage` from `$lib/api/errors` handles both shapes (and
 * `Error` instances), so it is always the right call. `firstErrorLine`
 * wraps it for single-line toasts.
 *
 * Escape hatch, for the rare case where the value genuinely is not a
 * rejection — a caught `SyntaxError` from `JSON.parse` of local input,
 * say — put the marker on the line above:
 *
 *   // beardgit:allow-string-error: <short justification>
 */

"use strict";

const ALLOW_MARKER = /beardgit:allow-string-error:/;

/** Does a comment on the preceding lines grant the escape hatch? */
function hasAllowMarker(context, node) {
  const source = context.sourceCode ?? context.getSourceCode();
  const line = node.loc.start.line;
  for (let n = line - 1; n >= Math.max(1, line - 2); n--) {
    const text = source.lines[n - 1] ?? "";
    if (ALLOW_MARKER.test(text)) return true;
  }
  return false;
}

/**
 * Names bound by an enclosing `catch (x)`, plus the conventional error
 * parameter names of a `.catch(x => …)` callback. Scoped deliberately: a
 * `String(count)` elsewhere is none of this rule's business.
 */
function caughtNames(context, node) {
  const source = context.sourceCode ?? context.getSourceCode();
  const names = new Set();
  let current = source.getAncestors ? node : node;
  const ancestors = source.getAncestors
    ? source.getAncestors(node)
    : context.getAncestors();
  for (const a of ancestors) {
    if (a.type === "CatchClause" && a.param && a.param.type === "Identifier") {
      names.add(a.param.name);
    }
    if (
      (a.type === "ArrowFunctionExpression" || a.type === "FunctionExpression") &&
      a.params.length === 1 &&
      a.params[0].type === "Identifier" &&
      /^(e|err|error|ex)$/.test(a.params[0].name)
    ) {
      names.add(a.params[0].name);
    }
  }
  void current;
  return names;
}

module.exports = {
  meta: {
    type: "problem",
    docs: {
      description:
        "use getErrorMessage() on caught errors — String() renders an IpcError as [object Object]",
    },
    schema: [],
    messages: {
      stringified:
        "`String({{name}})` renders a structured IpcError as \"[object Object]\". " +
        "Use `getErrorMessage({{name}})` from $lib/api/errors (or `firstErrorLine` for a toast).",
    },
  },

  create(context) {
    return {
      CallExpression(node) {
        if (node.callee.type !== "Identifier" || node.callee.name !== "String") return;
        if (node.arguments.length !== 1) return;
        const arg = node.arguments[0];
        if (arg.type !== "Identifier") return;
        if (!caughtNames(context, node).has(arg.name)) return;
        if (hasAllowMarker(context, node)) return;
        context.report({ node, messageId: "stringified", data: { name: arg.name } });
      },
    };
  },
};

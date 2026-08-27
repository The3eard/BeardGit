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
 * Three spellings, because a first version of this rule only knew one and
 * the sweep that went with it had the same blind spot:
 *
 *   String(err)              — the obvious one
 *   String(outcome.err)      — a member expression rooted on the binding
 *   `failed: ${err}`         — template interpolation, same coercion
 *
 * The second is not academic: `taskRunner.complete()` took `outcome.err`
 * and was reported as fixed while it was not, which put "[object Object]"
 * in the Tasks popover for every tracked mutation — push, pull, fetch,
 * merge, rebase, clone. The third accounted for six more sites, one of
 * them the only command in the app with bespoke error codes.
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
 * Names bound by an enclosing `catch (x)`, plus the parameter of a
 * `.catch(x => …)` callback.
 *
 * Scoped deliberately: `String(count)` elsewhere is none of this rule's
 * business, and neither is `items.map((e) => String(e))` — an earlier
 * version matched any single-parameter arrow whose parameter happened to
 * be named `e`, which would have reported that.
 */
function caughtNames(context, node) {
  const source = context.sourceCode ?? context.getSourceCode();
  const names = new Set();
  const ancestors = source.getAncestors
    ? source.getAncestors(node)
    : context.getAncestors();
  for (let i = 0; i < ancestors.length; i++) {
    const a = ancestors[i];
    if (a.type === "CatchClause" && a.param && a.param.type === "Identifier") {
      names.add(a.param.name);
      continue;
    }
    // A `.catch(cb)` callback: the function must be the sole argument of a
    // call whose callee is a `.catch` member access.
    if (a.type !== "ArrowFunctionExpression" && a.type !== "FunctionExpression") continue;
    if (a.params.length !== 1 || a.params[0].type !== "Identifier") continue;
    const parent = ancestors[i - 1];
    if (
      parent &&
      parent.type === "CallExpression" &&
      parent.arguments.length === 1 &&
      parent.arguments[0] === a &&
      parent.callee.type === "MemberExpression" &&
      parent.callee.property.type === "Identifier" &&
      parent.callee.property.name === "catch"
    ) {
      names.add(a.params[0].name);
    }
  }
  return names;
}

/**
 * The identifier an expression is rooted on, so `outcome.err` reports
 * under `outcome` and `err.cause.message` under `err`. Anything that is
 * not a plain identifier or member chain yields `null`.
 */
function rootName(node) {
  let cur = node;
  while (cur.type === "MemberExpression") cur = cur.object;
  return cur.type === "Identifier" ? cur.name : null;
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
        "Stringifying `{{name}}` renders a structured IpcError as \"[object Object]\". " +
        "Use `getErrorMessage(...)` from $lib/api/errors (or `firstErrorLine` for a toast).",
    },
  },

  create(context) {
    /** Report `expr` if it is rooted on a caught binding. */
    function check(node, expr) {
      const root = rootName(expr);
      if (!root) return;
      if (!caughtNames(context, node).has(root)) return;
      if (hasAllowMarker(context, node)) return;
      const source = context.sourceCode ?? context.getSourceCode();
      context.report({
        node,
        messageId: "stringified",
        data: { name: source.getText(expr) },
      });
    }

    return {
      CallExpression(node) {
        if (node.callee.type !== "Identifier" || node.callee.name !== "String") return;
        if (node.arguments.length !== 1) return;
        check(node, node.arguments[0]);
      },
      // `` `failed: ${err}` `` coerces exactly the same way.
      TemplateLiteral(node) {
        for (const expr of node.expressions) check(node, expr);
      },
    };
  },
};

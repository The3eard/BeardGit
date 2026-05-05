/**
 * Lightweight JSON linter for the in-app file editor.
 *
 * Built on `@codemirror/lint`'s `linter()` chassis. For any `*.json`
 * buffer we parse with the platform `JSON.parse` and convert errors to
 * `Diagnostic` entries. A few hand-curated schema rules apply to known
 * files (`package.json`, `tsconfig.json`, `.beardgit/requests/_env/*.json`)
 * so users get a hint when an obvious required field is missing.
 *
 * No AJV / no JSON Schema runtime — the rules are conservative, pure
 * functions, and live entirely in this file.
 */

import { linter, type Diagnostic } from "@codemirror/lint";
import type { EditorView } from "@codemirror/view";
import type { Extension } from "@codemirror/state";

/** Suffix used to detect a Requests panel env file in `lintBuffer`. */
const REQUESTS_ENV_FRAGMENT = "/.beardgit/requests/_env/";

/**
 * Convert a `JSON.parse` error message of the form
 * `"... at position N (line L column C)"` into a `[from, to]` slice
 * inside `doc`. Falls back to `[0, 1]` for unparseable messages so the
 * diagnostic still renders.
 */
function locateParseError(doc: string, message: string): [number, number] {
  const positionMatch = /position\s+(\d+)/i.exec(message);
  if (positionMatch) {
    const pos = Number.parseInt(positionMatch[1], 10);
    if (Number.isFinite(pos)) {
      const from = Math.min(Math.max(pos, 0), doc.length);
      const to = Math.min(from + 1, doc.length);
      return [from, Math.max(to, from)];
    }
  }
  return [0, Math.min(1, doc.length)];
}

/**
 * Run the JSON parse + schema rules on the given source text. Exported
 * separately from jsonLinter so unit tests can drive the linter
 * without standing up an EditorView.
 *
 * `filename` (when provided) is the workdir-relative path used to pick
 * which schema rules apply.
 */
export function lintBuffer(doc: string, filename: string | null): Diagnostic[] {
  const diagnostics: Diagnostic[] = [];
  // Empty / whitespace-only buffers don't parse but aren't useful to
  // flag — let the user start typing in peace.
  if (doc.trim().length === 0) return diagnostics;

  let parsed: unknown;
  try {
    parsed = JSON.parse(doc);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    const [from, to] = locateParseError(doc, message);
    diagnostics.push({
      from,
      to,
      severity: "error",
      message,
      source: "json",
    });
    return diagnostics;
  }

  // Only schema-check object roots — arrays / scalars don't have the
  // fields we're looking for.
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    return diagnostics;
  }
  const root = parsed as Record<string, unknown>;
  const basename = filename ? filename.split(/[\\/]/).pop() ?? "" : "";

  if (basename === "package.json") {
    if (typeof root.name !== "string" || root.name.trim().length === 0) {
      diagnostics.push({
        from: 0,
        to: Math.min(1, doc.length),
        severity: "warning",
        message: 'package.json should declare a "name" string',
        source: "json-schema",
      });
    }
    if (typeof root.version !== "string" || root.version.trim().length === 0) {
      diagnostics.push({
        from: 0,
        to: Math.min(1, doc.length),
        severity: "warning",
        message: 'package.json should declare a "version" string',
        source: "json-schema",
      });
    }
  }

  if (basename === "tsconfig.json") {
    if (
      "compilerOptions" in root &&
      (root.compilerOptions === null ||
        typeof root.compilerOptions !== "object" ||
        Array.isArray(root.compilerOptions))
    ) {
      diagnostics.push({
        from: 0,
        to: Math.min(1, doc.length),
        severity: "error",
        message: 'tsconfig.json "compilerOptions" must be an object',
        source: "json-schema",
      });
    }
  }

  if (filename && filename.includes(REQUESTS_ENV_FRAGMENT) && filename.endsWith(".json")) {
    // Requests env files are flat string->string maps. Flag any top-level
    // value that isn't a string, number, or boolean — keeps the schema
    // tolerant while still catching nested objects.
    for (const [key, value] of Object.entries(root)) {
      const t = typeof value;
      if (t !== "string" && t !== "number" && t !== "boolean") {
        diagnostics.push({
          from: 0,
          to: Math.min(1, doc.length),
          severity: "warning",
          message: `Requests env values should be string / number / boolean (key "${key}")`,
          source: "json-schema",
        });
      }
    }
  }

  return diagnostics;
}

/**
 * CodeMirror extension that runs lintBuffer on every `*.json` buffer.
 * `filename` is captured at construction time — `EditorPane` rebuilds
 * the extension array per tab, so the closure is fresh each mount. For
 * non-JSON buffers we still return an extension (a no-op linter) so the
 * caller can compose unconditionally.
 */
export function jsonLinter(filename: string | null): Extension {
  const isJson = !!filename && filename.toLowerCase().endsWith(".json");
  if (!isJson) {
    return linter(() => []);
  }
  return linter((view: EditorView) => lintBuffer(view.state.doc.toString(), filename));
}

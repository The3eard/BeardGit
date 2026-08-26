#!/usr/bin/env node

/**
 * Guard against sensitive arguments leaking into `#[instrument]` spans.
 *
 * ## Why this exists
 *
 * `#[instrument]` records every function argument that isn't skipped, and
 * the `fmt` layer re-renders a span's fields on **every event inside that
 * span**. So `#[instrument(skip(state, app))]` on a function taking
 * `message: String` writes the commit message to the log once per git
 * command the commit runs — even though the argv logging carefully elides
 * it.
 *
 * The trap is that `skip(...)` is an *allowlist of things to hide*. Add a
 * parameter later and it is recorded by default, silently. This class of
 * leak was fixed by inspection twice in one changeset and reappeared both
 * times, which is why it is enforced here instead.
 *
 * ## The rule
 *
 * If a function's parameter name matches {@link SENSITIVE_PARAMS}, its
 * `#[instrument]` attribute must use `skip_all`. `skip_all` is a denylist
 * of everything, with `fields(...)` as the explicit opt-in — the only form
 * that stays correct when the signature changes.
 *
 * Struct-typed parameters named `options` / `opts` / `input` / `payload`
 * count too: `CloneRepoOptions` holds a clone URL and `InitRepoOptions`
 * holds a `.gitignore` body, so a rule that only looked at leaf parameter
 * names missed both.
 *
 * ## Failing loudly
 *
 * Any attribute whose signature cannot be parsed is reported as an error,
 * not skipped. A parser gap must surface as a visible failure rather than
 * a silent pass — that is the difference between a guard and decoration.
 *
 * Run: `npm run check:instrument`
 */

import { readFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { join, relative } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname;
const SCAN_DIRS = [join(ROOT, "crates"), join(ROOT, "src-tauri", "src")];

/**
 * Parameter names whose values must never be recorded as span fields.
 *
 * Deliberately not "anything that looks like a path": repo paths and file
 * paths ARE wanted in the log — they're how you tell which repository and
 * which file a report is about. What's excluded is content the user
 * authored and identifiers that name their private work.
 *
 * Note `source` is absent on purpose: it is a branch name in
 * `list_ci_runs` and `create_mr_pr`, not a content field.
 */
const SENSITIVE_PARAMS = [
  // User-authored prose.
  "message",
  "body",
  "title",
  "description",
  "content",
  "contents",
  "text",
  "prompt",
  "comment",
  "note",
  // File and diff content.
  "diff",
  "patch",
  "hunks",
  "blob",
  "buffer",
  "data",
  "value",
  "values",
  // Credentials and anything that can embed them.
  //
  // `url` / `remote_url` are clone URLs: they can carry a PAT in the
  // userinfo and they name a possibly-private repository. Note that
  // `instance_url` is deliberately NOT here — a base host like
  // `https://gitlab.example.com` carries no credential and no repo name,
  // and it's the first thing you need to know when a provider connection
  // fails. The `token` at that callsite is skipped separately.
  "token",
  "password",
  "passphrase",
  "secret",
  "key",
  "api_key",
  "credentials",
  "url",
  "remote_url",
  // Process and shell input. This is the exact class that leaked through
  // task-runner's joined argv.
  "command",
  "test_command",
  "args",
  "argv",
  "env",
  "env_vars",
  // Search input — the CQL query language will cross IPC eventually.
  "query",
  "search",
  "pattern",
  "term",
  // Names that identify people or a private workflow.
  "email",
  "author",
  "committer",
  "username",
  "labels",
  "reviewers",
  "assignees",
  // Option structs — the payload hides one level down.
  "options",
  "opts",
  "input",
  "payload",
];

/** Recursively collect `.rs` files under `dir`. */
function rustFiles(dir) {
  if (!existsSync(dir)) return [];
  const out = [];
  for (const entry of readdirSync(dir)) {
    if (entry === "target") continue;
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) out.push(...rustFiles(full));
    else if (entry.endsWith(".rs")) out.push(full);
  }
  return out;
}

/**
 * Blank out comments and string literals, preserving length and newlines.
 *
 * Without this, a doc comment that *mentions* `#[instrument(skip(state))]`
 * is matched as a real attribute and paired with whatever function follows
 * it — a false positive on code that has no span at all. Two such comments
 * exist in this repo today.
 *
 * Length is preserved so byte offsets and line numbers still refer to the
 * original source.
 */
function blankCommentsAndStrings(source) {
  const out = source.split("");
  let i = 0;
  const blank = (from, to) => {
    for (let k = from; k < to && k < out.length; k++) {
      if (out[k] !== "\n") out[k] = " ";
    }
  };

  while (i < source.length) {
    const two = source.slice(i, i + 2);
    if (two === "//") {
      const end = source.indexOf("\n", i);
      const stop = end === -1 ? source.length : end;
      blank(i, stop);
      i = stop;
    } else if (two === "/*") {
      const end = source.indexOf("*/", i + 2);
      const stop = end === -1 ? source.length : end + 2;
      blank(i, stop);
      i = stop;
    } else if (source[i] === '"') {
      // Raw strings (r"…", r#"…"#) and escapes both matter here.
      let j = i + 1;
      while (j < source.length) {
        if (source[j] === "\\") j += 2;
        else if (source[j] === '"') break;
        else j++;
      }
      blank(i + 1, j);
      i = j + 1;
    } else {
      i++;
    }
  }
  return out.join("");
}

/**
 * Return `[start, end]` of the balanced `(…)` group starting at or after
 * `from`, or `null` if it can't be resolved.
 *
 * Only parens and square brackets are tracked. Angle brackets are
 * deliberately ignored: `->` in `impl Fn(&str) -> bool` would otherwise
 * drive the depth negative and silently abandon the scan.
 */
function balancedParens(source, from) {
  const open = source.indexOf("(", from);
  if (open === -1) return null;
  let depth = 0;
  for (let i = open; i < source.length; i++) {
    const ch = source[i];
    if (ch === "(" || ch === "[") depth++;
    else if (ch === ")" || ch === "]") {
      depth--;
      if (depth === 0) return [open, i];
      if (depth < 0) return null;
    }
  }
  return null;
}

/**
 * Extract parameter names from the signature of the `fn` that follows
 * `from`.
 *
 * Anchored on the `fn` keyword, not "the next open paren": attributes
 * stack, and `#[allow(clippy::too_many_arguments)]` sitting between
 * `#[instrument]` and the function would otherwise be parsed as the
 * parameter list — silently clean.
 *
 * Returns `{ params }` or `{ error }`.
 */
function paramNames(source, from) {
  const fnMatch = /\bfn\s+[A-Za-z_][A-Za-z0-9_]*/.exec(source.slice(from));
  if (!fnMatch) return { error: "no `fn` found after the attribute" };

  const fnEnd = from + fnMatch.index + fnMatch[0].length;
  const span = balancedParens(source, fnEnd);
  if (!span) return { error: "could not find a balanced parameter list" };

  const inner = source.slice(span[0] + 1, span[1]);

  // Split on top-level commas. Angle depth is tracked here (a comma inside
  // `State<'_, AppState>` must not split), but `>` preceded by `-` is the
  // arrow of a closure return type, not a closing bracket.
  const params = [];
  let depth = 0;
  let current = "";
  for (let i = 0; i < inner.length; i++) {
    const ch = inner[i];
    const prev = inner[i - 1];
    if (ch === "(" || ch === "[") depth++;
    else if (ch === ")" || ch === "]") depth--;
    else if (ch === "<") depth++;
    else if (ch === ">" && prev !== "-") depth--;

    if (ch === "," && depth === 0) {
      params.push(current);
      current = "";
    } else {
      current += ch;
    }
  }
  params.push(current);

  const names = params
    .map((p) => p.trim())
    .filter((p) => p && !p.startsWith("&") && p !== "self")
    .map((p) => p.split(":")[0].trim())
    .map((p) => p.replace(/^mut\s+/, ""))
    .filter(Boolean);

  return { params: names };
}

const violations = [];
const parseErrors = [];

for (const dir of SCAN_DIRS) {
  for (const file of rustFiles(dir)) {
    const raw = readFileSync(file, "utf8");
    const source = blankCommentsAndStrings(raw);
    const rel = relative(ROOT, file);

    const attrRe = /#\[(?:tracing::)?instrument\b/g;
    let match;
    while ((match = attrRe.exec(source)) !== null) {
      const line = source.slice(0, match.index).split("\n").length;

      // The attribute may be bare (`#[instrument]`) or parenthesised.
      let args = "";
      let afterAttr = match.index + match[0].length;
      if (source[afterAttr] === "(") {
        const span = balancedParens(source, afterAttr);
        if (!span) {
          parseErrors.push({
            file: rel,
            line,
            reason: "unbalanced `#[instrument(...)]` argument list",
          });
          continue;
        }
        args = source.slice(span[0], span[1] + 1);
        afterAttr = span[1] + 1;
      }

      // Word-boundary match: `fields(skip_all_hunks = 1)` must not count.
      if (/\bskip_all\b/.test(args)) continue;

      const { params, error } = paramNames(source, afterAttr);
      if (error) {
        parseErrors.push({ file: rel, line, reason: error });
        continue;
      }

      const skipped = new Set(
        [...args.matchAll(/\bskip\(([^)]*)\)/g)]
          .flatMap((m) => m[1].split(","))
          .map((s) => s.trim()),
      );

      const leaked = params.filter(
        (p) => SENSITIVE_PARAMS.includes(p) && !skipped.has(p),
      );
      if (leaked.length > 0) {
        violations.push({ file: rel, line, params: leaked });
      }
    }
  }
}

if (parseErrors.length > 0) {
  console.error(
    `\n✗ ${parseErrors.length} #[instrument] attribute(s) could not be analysed.\n`,
  );
  for (const e of parseErrors) {
    console.error(`  ${e.file}:${e.line}  ${e.reason}`);
  }
  console.error(
    `
An unparseable signature is reported rather than skipped: a silent pass
here is how a leak gets through. Either simplify the signature or extend
the parser in scripts/check-instrument-fields.mjs.
`,
  );
}

if (violations.length > 0) {
  console.error(
    `\n✗ ${violations.length} #[instrument] attribute(s) record sensitive arguments as span fields.\n`,
  );
  for (const v of violations) {
    console.error(`  ${v.file}:${v.line}  records: ${v.params.join(", ")}`);
  }
  console.error(
    `
Span fields are re-rendered on every event inside the span, so these values
reach the log file even when the event itself never mentions them.

Fix: use \`skip_all\` and opt back in explicitly.

    #[instrument(skip_all, fields(repo = %path), name = "cmd::x::y")]

\`skip(...)\` is not enough — it silently stops covering any argument added
to the signature later.
`,
  );
}

if (violations.length > 0 || parseErrors.length > 0) process.exit(1);

console.log("✓ No #[instrument] attribute records a sensitive argument.");

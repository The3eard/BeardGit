/**
 * CodeMirror autocomplete source for `{{var}}` placeholders inside the
 * Requests body editor.
 *
 * The completion fires when the cursor sits inside an unclosed `{{ ... `
 * mustache pair. Suggestions cover:
 *
 * 1. Plaintext vars from the active env file (`requests_load_env`).
 * 2. Secret names from the active env file (the *names* — values stay
 *    encrypted).
 *
 * Picking a suggestion inserts the closing `}}` and places the cursor
 * after the braces, so users never have to type the closer manually.
 *
 * The factory takes lazy getters for the project path and active env
 * name so consumers can wire it to live Svelte stores without
 * imperatively re-binding the extension on every keystroke. The
 * suggestion list is fetched on every completion call (the IPC is one
 * small JSON read for the env file; the latency is unnoticeable in
 * practice and keeps the panel honest after env edits).
 */

import { requestsLoadEnv } from "$lib/api/tauri";
import type { RequestEnvFile } from "$lib/types/requests";
import {
  type CompletionContext,
  type CompletionResult,
  type Completion,
} from "@codemirror/autocomplete";

/**
 * Build a CodeMirror autocomplete source that fires inside `{{ }}`
 * mustaches and yields env vars and secret names.
 */
export function varCompletion(
  getProjectPath: () => string,
  getEnvName: () => string | null,
) {
  return async (
    context: CompletionContext,
  ): Promise<CompletionResult | null> => {
    // Look back from the cursor for the nearest "{{". If we find a
    // closing "}}" before "{{" (or hit a newline first), we're not
    // inside a mustache — bail out.
    const text = context.state.doc.toString();
    const pos = context.pos;
    let openAt = -1;
    for (let i = pos - 1; i >= 0; i--) {
      const c = text[i];
      if (c === "\n") break;
      if (c === "}" && i > 0 && text[i - 1] === "}") break;
      if (c === "{" && i > 0 && text[i - 1] === "{") {
        openAt = i + 1; // first char inside the mustache
        break;
      }
    }
    if (openAt < 0) {
      // Not inside `{{ ... `; only fire when the user explicitly invokes
      // completion (Ctrl+Space) so we don't pollute every keystroke.
      if (!context.explicit) return null;
    }

    // Token start = the position right after `{{` (or the prefix start
    // for explicit invocations outside a mustache).
    const from = openAt >= 0 ? openAt : pos;
    const prefix = text.slice(from, pos);

    // Reject prefixes containing characters that can't be part of a
    // variable identifier — keeps the popover from showing on `{{x.y` etc.
    if (!/^[A-Za-z0-9_]*$/.test(prefix)) return null;

    const names = await loadCandidates(getProjectPath(), getEnvName());

    // Build Completion entries. Each entry inserts the bare name plus
    // the closing `}}` so the user never has to type the closer.
    const options: Completion[] = names.map((entry) => ({
      label: entry.label,
      detail: entry.detail,
      type: entry.type,
      apply: (view, _completion, fromPos, toPos) => {
        // Replace the prefix with `name}}` and move the cursor past it.
        const insert = `${entry.label}}}`;
        view.dispatch({
          changes: { from: fromPos, to: toPos, insert },
          selection: { anchor: fromPos + insert.length },
        });
      },
    }));

    if (options.length === 0) return null;

    return {
      from,
      to: pos,
      options,
      validFor: /^[A-Za-z0-9_]*$/,
    };
  };
}

interface Candidate {
  label: string;
  detail: string;
  type: "variable" | "constant";
}

/**
 * Fetch the active env's vars + secret names. Returns a deduplicated,
 * alphabetised list. A failed env load simply yields an empty list.
 */
async function loadCandidates(
  projectPath: string,
  envName: string | null,
): Promise<Candidate[]> {
  const out = new Map<string, Candidate>();

  const env: RequestEnvFile | null =
    projectPath && envName
      ? await requestsLoadEnv(projectPath, envName).catch(() => null)
      : null;

  if (env) {
    const vars = env.vars ?? {};
    for (const [key, value] of Object.entries(vars)) {
      if (!key) continue;
      out.set(key, {
        label: key,
        detail: value ? `= ${value}` : "env var",
        type: "variable",
      });
    }
    const secrets = env.secrets ?? [];
    for (const name of secrets) {
      if (!name) continue;
      out.set(name, {
        label: name,
        detail: "secret",
        type: "variable",
      });
    }
  }

  return Array.from(out.values()).sort((a, b) => a.label.localeCompare(b.label));
}

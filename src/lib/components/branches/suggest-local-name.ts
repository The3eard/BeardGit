/**
 * Source descriptor passed to `CreateBranchDialog` and the name
 * suggestion helper. Kept as a discriminated union so each branch of
 * the dialog's logic (HEAD / branch tip / arbitrary commit) can be
 * pattern-matched exhaustively.
 */
export type InitialSource =
  | { kind: "head" }
  | { kind: "ref"; name: string; oid: string }
  | { kind: "commit"; oid: string };

/**
 * Suggest a sensible default local branch name given a source and the
 * list of configured remotes.
 *
 * Behaviour:
 * - HEAD or commit sources → "" (user types from scratch).
 * - A `ref` source whose name starts with any `"<remote>/"` prefix →
 *   the name with that prefix stripped (keeps nested path segments).
 * - A `ref` source that is a local branch → "".
 *
 * Pure function — easy to unit-test and safe to call inside a Svelte
 * `$effect`.
 */
export function suggestLocalName(source: InitialSource, remotes: string[]): string {
  if (source.kind !== "ref") return "";
  const name = source.name;
  for (const r of remotes) {
    const prefix = `${r}/`;
    if (name.startsWith(prefix)) return name.slice(prefix.length);
  }
  return "";
}

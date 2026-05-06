/**
 * Split a remote-branch display name (e.g. `origin/feature/foo`) into
 * its remote and branch parts.
 *
 * BeardGit lists remote branches with the remote name as the first path
 * segment — that's the form libgit2 returns and what the BranchList
 * tree renders. The "Delete on remote" action needs the parts split so
 * the backend can run `git push <remote> --delete <branch>` and the
 * confirmation modal can show "delete X on Y".
 *
 * Returns `null` when:
 * - the value has no `/` (single segment isn't a valid remote ref);
 * - the slash is at position 0 (`/foo`) or the last char (`origin/`);
 * - the branch part is the symbolic `HEAD` ref (`origin/HEAD`), which
 *   tracks the default branch and should never be deleted directly.
 */
export function parseRemoteBranch(
  fullPath: string,
): { remote: string; branch: string } | null {
  const slash = fullPath.indexOf("/");
  if (slash <= 0 || slash === fullPath.length - 1) return null;
  const remote = fullPath.slice(0, slash);
  const branch = fullPath.slice(slash + 1);
  if (branch === "HEAD") return null;
  return { remote, branch };
}

//! Submodule management — list, init, update, and deinit git submodules.
//!
//! Uses libgit2's Submodule API for read operations (listing, status) and
//! the git CLI for write operations (init, update, deinit) since those
//! may involve network fetches and complex state changes.

use serde::Serialize;
use tracing::instrument;

use crate::error::GitError;
use crate::repository::Repository;

/// Status of a submodule relative to the superproject.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmoduleStatus {
    /// Registered in `.gitmodules` but not yet initialized (`git submodule init` needed).
    Uninitialized,
    /// Checked out at the exact commit the superproject expects.
    Clean,
    /// Checked out but at a different commit than the superproject expects.
    Outdated,
    /// Has local modifications in its working tree.
    Dirty,
}

/// Information about a single submodule.
#[derive(Debug, Clone, Serialize)]
pub struct SubmoduleInfo {
    /// Submodule logical name, as recorded in the `.gitmodules` of the
    /// superproject it belongs to — so a nested submodule's name is local to
    /// its parent, not a path from the repository root.
    pub name: String,
    /// Path relative to the **repository root**, so it addresses the working
    /// tree of a nested submodule too (`libs/sub/inner`).
    pub path: String,
    /// Remote URL configured for this submodule.
    pub url: String,
    /// Current HEAD OID of the submodule working tree, or `None` if uninitialized.
    pub oid: Option<String>,
    /// The OID the superproject expects (recorded in the index/tree).
    pub registered_oid: String,
    /// Computed status of the submodule.
    pub status: SubmoduleStatus,
    /// Nesting level: `0` for a submodule of the repository itself, `1` for a
    /// submodule of that submodule, and so on. The panel indents by it.
    pub depth: usize,
    /// Path (from the repository root) of the superproject this submodule is
    /// registered in, or `None` at depth 0. Every write operation runs inside
    /// this directory — see [`Repository::init_submodule`].
    pub parent: Option<String>,
}

/// How deep [`Repository::list_submodules`] recurses.
///
/// Nesting past a couple of levels is vanishingly rare, and each level costs
/// an extra repository open plus a status walk per submodule. The cap also
/// makes the recursion terminate unconditionally, which a symlinked working
/// tree pointing back up the chain would otherwise threaten.
const MAX_SUBMODULE_DEPTH: usize = 5;

impl Repository {
    /// List every submodule in the repository, including submodules of
    /// submodules down to [`MAX_SUBMODULE_DEPTH`].
    ///
    /// Uses libgit2's `Submodule` API for fast, no-fork reads. Entries come
    /// out depth-first, each parent immediately followed by its children, so
    /// the frontend can render the list as-is and indent by `depth`.
    ///
    /// A submodule that isn't checked out has nothing to recurse into, and a
    /// checked-out one that can't be opened (a broken `.git` link, say) is
    /// listed without its children rather than failing the whole listing.
    pub fn list_submodules(&self) -> Result<Vec<SubmoduleInfo>, GitError> {
        let mut submodules = Vec::new();
        collect_submodules(self.inner(), None, 0, &mut submodules)?;
        Ok(submodules)
    }

    /// Initialize a submodule (registers it and clones the repo).
    ///
    /// Equivalent to `git submodule init <path>`. `parent` is the
    /// [`SubmoduleInfo::parent`] of the submodule: a nested submodule is
    /// registered in *its parent's* `.gitmodules`, so the operation has to run
    /// there — from the root, git rejects the path as unknown.
    #[instrument(skip(self), fields(path = %path))]
    pub fn init_submodule(&self, path: &str, parent: Option<&str>) -> Result<(), GitError> {
        let (mut args, target) = submodule_argv(parent, path, &["submodule", "init", "--"]);
        args.push(target);
        self.expect_success(&args)
    }

    /// Deinitialize a submodule (removes its working tree and config).
    ///
    /// Equivalent to `git submodule deinit [-f] <path>`. See
    /// [`Self::init_submodule`] for `parent`.
    #[instrument(skip(self), fields(path = %path))]
    pub fn deinit_submodule(
        &self,
        path: &str,
        parent: Option<&str>,
        force: bool,
    ) -> Result<(), GitError> {
        let (mut args, target) = submodule_argv(parent, path, &["submodule", "deinit"]);
        if force {
            args.push("--force");
        }
        args.push("--");
        args.push(target);
        self.expect_success(&args)
    }

    /// Remove a submodule completely (deinit, remove from index, delete directory).
    ///
    /// Equivalent to `git submodule deinit -f <path> && git rm -f <path>`. See
    /// [`Self::init_submodule`] for `parent`.
    #[instrument(skip(self), fields(path = %path))]
    pub fn remove_submodule(&self, path: &str, parent: Option<&str>) -> Result<(), GitError> {
        let (mut deinit, target) =
            submodule_argv(parent, path, &["submodule", "deinit", "--force", "--"]);
        deinit.push(target);
        self.expect_success(&deinit)?;

        // Removing from the index has to happen in the same repository the
        // submodule is registered in, for the same reason as the deinit.
        let (mut rm, target) = submodule_argv(parent, path, &["rm", "-f", "--"]);
        rm.push(target);
        self.expect_success(&rm)
    }

    /// Run `git` with `args` and turn a non-zero exit into [`GitError::CliError`].
    fn expect_success(&self, args: &[&str]) -> Result<(), GitError> {
        let result = self.git_cmd(args)?;
        if result.success {
            Ok(())
        } else {
            Err(GitError::CliError(result.stderr))
        }
    }

    /// Return the absolute path to a submodule's working directory.
    ///
    /// Resolves `<repo_root>/<submodule_path>` and refuses a result that falls
    /// outside the repository. The value is handed straight to `open_project`,
    /// so a `..` in the argument would open a project tab on an arbitrary
    /// directory.
    pub fn submodule_abs_path(&self, submodule_path: &str) -> Result<String, GitError> {
        let joined = self.path().join(submodule_path);
        let abs = joined.canonicalize().map_err(|_| {
            GitError::RepoNotFound(format!(
                "Submodule path does not exist: {}",
                joined.display()
            ))
        })?;
        // Both sides canonicalised: on macOS the repo root itself is often
        // reached through a symlink (`/var` → `/private/var`), and a resolved
        // path never starts with an unresolved root.
        let root = self.path().canonicalize().map_err(GitError::Io)?;
        if !abs.starts_with(&root) {
            return Err(GitError::RepoNotFound(format!(
                "Submodule path escapes the repository: {}",
                abs.display()
            )));
        }
        Ok(abs.to_string_lossy().to_string())
    }
}

/// Append `repo`'s submodules — and theirs, recursively — to `out`.
///
/// `parent` is the path of `repo` itself relative to the repository root
/// (`None` for the root), and prefixes the paths reported for its submodules
/// so every entry addresses the working tree from the root.
fn collect_submodules(
    repo: &git2::Repository,
    parent: Option<&str>,
    depth: usize,
    out: &mut Vec<SubmoduleInfo>,
) -> Result<(), GitError> {
    let sm_list = repo.submodules()?;

    for sm in &sm_list {
        let name = sm.name().unwrap_or("").to_string();
        let local_path = sm.path().to_string_lossy().to_string();
        let path = match parent {
            Some(prefix) => format!("{prefix}/{local_path}"),
            None => local_path,
        };
        let url = sm.url().unwrap_or("").to_string();
        let registered_oid = sm.index_id().map(|id| id.to_string()).unwrap_or_default();

        // Status is a property of the owning repository, so it is queried on
        // `repo` with the name local to it.
        let status_flags = repo.submodule_status(&name, git2::SubmoduleIgnore::Unspecified)?;

        let oid;
        let status;

        if status_flags.contains(git2::SubmoduleStatus::WD_UNINITIALIZED) {
            oid = None;
            status = SubmoduleStatus::Uninitialized;
        } else {
            let workdir_oid = sm.workdir_id().map(|id| id.to_string());
            oid = workdir_oid.clone();

            // The three `WD_*` flags mean different things, and conflating
            // them made `Outdated` unreachable: libgit2 raises `WD_MODIFIED`
            // for "the submodule's HEAD is not the commit the superproject
            // records", which is precisely outdated — so a moved HEAD was
            // reported as dirty and the OID comparison below never ran. Dirty
            // is about the submodule's own index and working tree, and stays
            // first: uncommitted work is the more urgent thing to surface
            // when both are true.
            if status_flags.intersects(
                git2::SubmoduleStatus::WD_INDEX_MODIFIED | git2::SubmoduleStatus::WD_WD_MODIFIED,
            ) {
                status = SubmoduleStatus::Dirty;
            } else if status_flags.contains(git2::SubmoduleStatus::WD_MODIFIED)
                || workdir_oid.as_deref() != Some(&registered_oid)
            {
                status = SubmoduleStatus::Outdated;
            } else {
                status = SubmoduleStatus::Clean;
            }
        }

        let checked_out = status != SubmoduleStatus::Uninitialized;

        out.push(SubmoduleInfo {
            name,
            path: path.clone(),
            url,
            oid,
            registered_oid,
            status,
            depth,
            parent: parent.map(str::to_string),
        });

        if checked_out && depth + 1 < MAX_SUBMODULE_DEPTH {
            // A checked-out submodule whose repository won't open (broken
            // `.git` file, deleted gitdir) is listed without its children
            // rather than taking the whole listing down.
            if let Ok(nested) = sm.open() {
                collect_submodules(&nested, Some(&path), depth + 1, out)?;
            }
        }
    }

    Ok(())
}

/// Split a submodule addressed from the repository root into the directory
/// the operation has to run in and the path to hand git once there.
///
/// A nested submodule is registered in *its parent's* `.gitmodules`, so
/// `git submodule <op> libs/sub/inner` from the root is refused: the operation
/// belongs in `libs/sub`, addressed as `inner`. `parent` is
/// [`SubmoduleInfo::parent`]; at depth 0 this returns `(None, path)`.
///
/// Public because `app-core` builds its own argv for the update operations —
/// they run through the task runner rather than [`Repository::git_cmd`], and
/// point the task's working directory at the returned subdirectory.
pub fn submodule_operation_target<'a>(
    parent: Option<&'a str>,
    path: &'a str,
) -> (Option<&'a str>, &'a str) {
    match parent {
        Some(prefix) => (
            Some(prefix),
            path.strip_prefix(prefix)
                .and_then(|rest| rest.strip_prefix('/'))
                .unwrap_or(path),
        ),
        None => (None, path),
    }
}

/// Build the argv for a submodule write operation, plus the path to hand it.
///
/// At depth 0 that is `op` and the path unchanged; for a nested submodule it
/// prefixes `-C <parent>` per [`submodule_operation_target`].
fn submodule_argv<'a>(
    parent: Option<&'a str>,
    path: &'a str,
    op: &[&'a str],
) -> (Vec<&'a str>, &'a str) {
    let (dir, target) = submodule_operation_target(parent, path);
    let mut args: Vec<&'a str> = Vec::with_capacity(op.len() + 3);
    if let Some(dir) = dir {
        args.push("-C");
        args.push(dir);
    }
    args.extend_from_slice(op);
    (args, target)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Path of the one submodule every fixture below registers.
    const SUB_PATH: &str = "libs/sub";

    /// Run `git` in `dir` and assert it succeeded.
    ///
    /// `protocol.file.allow=always` is not optional: cloning a submodule from
    /// a local path uses the `file` transport, which git refuses for
    /// submodules by default (CVE-2022-39253). Every fixture invocation that
    /// can reach a clone needs it.
    ///
    /// `-c` identity rather than the ambient config so the fixture doesn't
    /// depend on the developer's `~/.gitconfig`.
    fn git(dir: &std::path::Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .args([
                "-c",
                "protocol.file.allow=always",
                "-c",
                "user.name=Test User",
                "-c",
                "user.email=test@example.com",
                "-c",
                "core.autocrlf=false",
            ])
            .args(args)
            .current_dir(dir)
            .output()
            .expect("spawn git");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// Commit `content` to `file` in the repo at `dir`.
    fn commit_file(dir: &std::path::Path, file: &str, content: &str, message: &str) {
        std::fs::write(dir.join(file), content).unwrap();
        git(dir, &["add", file]);
        git(dir, &["commit", "-m", message]);
    }

    /// A superproject carrying one **real** submodule at [`SUB_PATH`], checked
    /// out at the commit the superproject records.
    ///
    /// Returns `(superproject, submodule source)`. Both `TempDir`s must stay
    /// alive for the duration of a test: the second one is the submodule's
    /// `origin`, and dropping it deletes the directory `.gitmodules` points at.
    fn create_test_repo_with_submodule() -> (tempfile::TempDir, tempfile::TempDir) {
        let sub_remote = tempfile::tempdir().unwrap();
        git(sub_remote.path(), &["init", "-q", "."]);
        commit_file(sub_remote.path(), "sub.txt", "sub content", "init sub");

        let super_dir = tempfile::tempdir().unwrap();
        git(super_dir.path(), &["init", "-q", "."]);
        commit_file(super_dir.path(), "main.txt", "main content", "init super");

        let sub_url = sub_remote.path().to_string_lossy().to_string();
        git(
            super_dir.path(),
            &["submodule", "add", "-q", "--", &sub_url, SUB_PATH],
        );
        git(super_dir.path(), &["commit", "-m", "add submodule"]);

        (super_dir, sub_remote)
    }

    /// The single submodule of a fixture repo.
    fn only_submodule(repo: &Repository) -> SubmoduleInfo {
        let mut subs = repo.list_submodules().expect("list_submodules");
        assert_eq!(subs.len(), 1, "fixture registers exactly one submodule");
        subs.remove(0)
    }

    /// Name of the submodule the nested fixture hangs off [`SUB_PATH`].
    const NESTED_NAME: &str = "nested";

    /// Path of that nested submodule, from the superproject root.
    fn nested_path() -> String {
        format!("{SUB_PATH}/{NESTED_NAME}")
    }

    /// A superproject whose submodule at [`SUB_PATH`] has a submodule of its
    /// own at [`NESTED_NAME`], both checked out.
    ///
    /// Returns the superproject plus the two source repos, which must stay
    /// alive: they are the `origin` of the two submodules.
    fn create_test_repo_with_nested_submodule()
    -> (tempfile::TempDir, tempfile::TempDir, tempfile::TempDir) {
        let inner_remote = tempfile::tempdir().unwrap();
        git(inner_remote.path(), &["init", "-q", "."]);
        commit_file(
            inner_remote.path(),
            "inner.txt",
            "inner content",
            "init inner",
        );

        let mid_remote = tempfile::tempdir().unwrap();
        git(mid_remote.path(), &["init", "-q", "."]);
        commit_file(mid_remote.path(), "sub.txt", "sub content", "init sub");
        let inner_url = inner_remote.path().to_string_lossy().to_string();
        git(
            mid_remote.path(),
            &["submodule", "add", "-q", "--", &inner_url, NESTED_NAME],
        );
        git(mid_remote.path(), &["commit", "-m", "add nested"]);

        let super_dir = tempfile::tempdir().unwrap();
        git(super_dir.path(), &["init", "-q", "."]);
        commit_file(super_dir.path(), "main.txt", "main content", "init super");
        let mid_url = mid_remote.path().to_string_lossy().to_string();
        git(
            super_dir.path(),
            &["submodule", "add", "-q", "--", &mid_url, SUB_PATH],
        );
        git(super_dir.path(), &["commit", "-m", "add submodule"]);
        // `submodule add` clones one level deep; the nested one needs a
        // recursive update before it has a working tree.
        git(
            super_dir.path(),
            &["submodule", "update", "--init", "--recursive", "-q"],
        );

        (super_dir, mid_remote, inner_remote)
    }

    #[test]
    fn test_submodule_status_serialization() {
        let json = serde_json::to_string(&SubmoduleStatus::Clean).unwrap();
        assert_eq!(json, "\"clean\"");
        let json = serde_json::to_string(&SubmoduleStatus::Uninitialized).unwrap();
        assert_eq!(json, "\"uninitialized\"");
        let json = serde_json::to_string(&SubmoduleStatus::Outdated).unwrap();
        assert_eq!(json, "\"outdated\"");
        let json = serde_json::to_string(&SubmoduleStatus::Dirty).unwrap();
        assert_eq!(json, "\"dirty\"");
    }

    #[test]
    fn list_submodules_on_repo_without_submodules_is_empty() {
        let (_tmp, path) = crate::test_support::create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        assert!(repo.list_submodules().unwrap().is_empty());
    }

    #[test]
    fn list_submodules_reports_a_checked_out_submodule_as_clean() {
        let (super_dir, sub_remote) = create_test_repo_with_submodule();
        let repo = Repository::open(super_dir.path()).unwrap();

        let sub = only_submodule(&repo);
        assert_eq!(sub.name, SUB_PATH);
        assert_eq!(sub.path, SUB_PATH);
        assert_eq!(sub.url, sub_remote.path().to_string_lossy());
        assert_eq!(
            sub.oid.as_deref(),
            Some(sub.registered_oid.as_str()),
            "a freshly added submodule sits at the recorded commit"
        );
        assert_eq!(sub.status, SubmoduleStatus::Clean);
    }

    #[test]
    fn list_submodules_reports_a_modified_working_tree_as_dirty() {
        let (super_dir, _sub_remote) = create_test_repo_with_submodule();
        std::fs::write(super_dir.path().join(SUB_PATH).join("sub.txt"), "edited").unwrap();

        let repo = Repository::open(super_dir.path()).unwrap();
        let sub = only_submodule(&repo);

        assert_eq!(sub.status, SubmoduleStatus::Dirty);
        assert_eq!(
            sub.oid.as_deref(),
            Some(sub.registered_oid.as_str()),
            "an uncommitted edit does not move the submodule's HEAD"
        );
    }

    #[test]
    fn list_submodules_reports_a_moved_head_as_outdated() {
        let (super_dir, _sub_remote) = create_test_repo_with_submodule();
        let sub_wd = super_dir.path().join(SUB_PATH);
        // Commit inside the submodule: its HEAD moves, the gitlink the
        // superproject recorded stays behind. This is the "outdated" case —
        // and the one that separates it from "dirty", since the submodule's
        // own working tree is clean afterwards.
        commit_file(&sub_wd, "sub.txt", "second revision", "move sub HEAD");

        let repo = Repository::open(super_dir.path()).unwrap();
        let sub = only_submodule(&repo);

        assert_ne!(
            sub.oid.as_deref(),
            Some(sub.registered_oid.as_str()),
            "the fixture must actually move the submodule's HEAD"
        );
        assert_eq!(sub.status, SubmoduleStatus::Outdated);
    }

    #[test]
    fn list_submodules_reports_a_deinitialized_submodule_as_uninitialized() {
        let (super_dir, _sub_remote) = create_test_repo_with_submodule();
        git(super_dir.path(), &["submodule", "deinit", "-f", SUB_PATH]);

        let repo = Repository::open(super_dir.path()).unwrap();
        let sub = only_submodule(&repo);

        assert_eq!(sub.status, SubmoduleStatus::Uninitialized);
        assert_eq!(sub.oid, None);
        assert!(
            !sub.registered_oid.is_empty(),
            "the superproject still records the commit it expects"
        );
    }

    #[test]
    fn test_submodule_abs_path_not_found() {
        let (super_dir, _sub_remote) = create_test_repo_with_submodule();
        let repo = Repository::open(super_dir.path()).unwrap();
        let result = repo.submodule_abs_path("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn list_submodules_includes_nested_submodules_depth_first() {
        let (super_dir, _mid, _inner) = create_test_repo_with_nested_submodule();
        let repo = Repository::open(super_dir.path()).unwrap();

        let subs = repo.list_submodules().unwrap();

        assert_eq!(
            subs.iter()
                .map(|s| (s.path.clone(), s.depth))
                .collect::<Vec<_>>(),
            vec![(SUB_PATH.to_string(), 0), (nested_path(), 1)],
        );
        assert_eq!(subs[0].parent, None);
        assert_eq!(subs[1].parent.as_deref(), Some(SUB_PATH));
        assert_eq!(
            subs[1].name, NESTED_NAME,
            "a nested submodule's name is local to its parent"
        );
        assert_eq!(subs[1].status, SubmoduleStatus::Clean);
    }

    #[test]
    fn list_submodules_does_not_recurse_into_an_uninitialized_submodule() {
        let (super_dir, _mid, _inner) = create_test_repo_with_nested_submodule();
        git(super_dir.path(), &["submodule", "deinit", "-f", SUB_PATH]);
        let repo = Repository::open(super_dir.path()).unwrap();

        let subs = repo.list_submodules().unwrap();

        assert_eq!(
            subs.len(),
            1,
            "a submodule with no working tree has no children"
        );
        assert_eq!(subs[0].status, SubmoduleStatus::Uninitialized);
    }

    #[test]
    fn deinit_of_a_nested_submodule_runs_inside_its_parent() {
        let (super_dir, _mid, _inner) = create_test_repo_with_nested_submodule();
        let repo = Repository::open(super_dir.path()).unwrap();
        let nested_file = super_dir.path().join(nested_path()).join("inner.txt");
        assert!(
            nested_file.exists(),
            "fixture checks the nested submodule out"
        );

        repo.deinit_submodule(&nested_path(), Some(SUB_PATH), true)
            .expect("deinit nested");

        assert!(!nested_file.exists(), "the nested working tree is emptied");
        assert_eq!(
            repo.list_submodules().unwrap()[1].status,
            SubmoduleStatus::Uninitialized
        );
    }

    #[test]
    fn a_nested_submodule_operation_without_its_parent_is_rejected() {
        let (super_dir, _mid, _inner) = create_test_repo_with_nested_submodule();
        let repo = Repository::open(super_dir.path()).unwrap();

        // The whole reason the `parent` argument exists: from the root, the
        // nested path is not a submodule git knows about.
        let err = repo
            .deinit_submodule(&nested_path(), None, true)
            .expect_err("git must refuse this");

        assert!(matches!(err, GitError::CliError(_)), "unexpected: {err:?}");
    }

    #[test]
    fn submodule_argv_leaves_a_top_level_path_alone() {
        let (args, target) = submodule_argv(None, "libs/sub", &["submodule", "init", "--"]);

        assert_eq!(args, ["submodule", "init", "--"]);
        assert_eq!(target, "libs/sub");
    }

    #[test]
    fn submodule_argv_runs_a_nested_path_in_its_parent() {
        let (args, target) = submodule_argv(
            Some("libs/sub"),
            "libs/sub/nested",
            &["submodule", "init", "--"],
        );

        assert_eq!(args, ["-C", "libs/sub", "submodule", "init", "--"]);
        assert_eq!(target, "nested");
    }

    #[test]
    fn submodule_abs_path_resolves_a_registered_submodule() {
        let (super_dir, _sub_remote) = create_test_repo_with_submodule();
        let repo = Repository::open(super_dir.path()).unwrap();

        let abs = repo.submodule_abs_path(SUB_PATH).unwrap();

        assert!(std::path::Path::new(&abs).join(".git").exists());
    }

    #[test]
    fn submodule_abs_path_rejects_a_path_outside_the_repository() {
        let (super_dir, _sub_remote) = create_test_repo_with_submodule();
        let repo = Repository::open(super_dir.path()).unwrap();

        // The parent of a tempdir exists, so this fails containment rather
        // than existence — the case a bare `exists()` check let through.
        let err = repo.submodule_abs_path("..").expect_err("must be refused");

        assert!(
            matches!(err, GitError::RepoNotFound(ref m) if m.contains("escapes the repository")),
            "unexpected error: {err:?}"
        );
    }
}

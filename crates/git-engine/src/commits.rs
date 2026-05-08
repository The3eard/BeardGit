//! Commit walking and history retrieval.
//!
//! Extends [`Repository`] with methods to walk the full commit graph or to
//! retrieve individual commits by OID. Filtering by branch, author, message,
//! or SHA prefix is provided via [`Repository::walk_commits_filtered`].

use std::collections::HashMap;

use serde::Serialize;

use crate::{error::GitError, repository::Repository};

/// Serialisable snapshot of a single git commit.
#[derive(Debug, Clone, Serialize)]
pub struct CommitInfo {
    /// Full SHA-1 OID as a hex string.
    pub oid: String,
    /// First line of the commit message.
    pub summary: String,
    /// Remainder of the commit message after the summary line.
    pub body: String,
    /// Author display name.
    pub author: String,
    /// Author e-mail address.
    pub email: String,
    /// Unix timestamp (seconds since epoch) of the author date.
    pub timestamp: i64,
    /// OIDs of parent commits (empty for root commits, two entries for merges).
    pub parents: Vec<String>,
    /// Short ref names (branches, tags) that point directly at this commit.
    pub refs: Vec<String>,
}

/// Extract a [`CommitInfo`] from a libgit2 commit and its OID.
fn commit_to_info(
    commit: &git2::Commit,
    oid: git2::Oid,
    ref_map: &std::collections::HashMap<String, Vec<String>>,
) -> CommitInfo {
    let summary = commit.summary().unwrap_or("").to_owned();
    let body = commit.body().unwrap_or("").to_owned();
    let author = commit.author().name().unwrap_or("").to_owned();
    let email = commit.author().email().unwrap_or("").to_owned();
    let timestamp = commit.time().seconds();
    let parents: Vec<String> = commit.parent_ids().map(|id| id.to_string()).collect();
    let refs = ref_map.get(&oid.to_string()).cloned().unwrap_or_default();
    CommitInfo {
        oid: oid.to_string(),
        summary,
        body,
        author,
        email,
        timestamp,
        parents,
        refs,
    }
}

/// Build a map from OID (as string) → list of short ref names pointing to it.
fn build_ref_map(repo: &git2::Repository) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();

    if let Ok(references) = repo.references() {
        for reference in references.flatten() {
            // Resolve symbolic refs to their target
            let target_oid = if let Some(oid) = reference.target() {
                oid
            } else if let Ok(resolved) = reference.resolve() {
                if let Some(oid) = resolved.target() {
                    oid
                } else {
                    continue;
                }
            } else {
                continue;
            };

            let shorthand = reference
                .shorthand()
                .unwrap_or_else(|| reference.name().unwrap_or("unknown"))
                .to_owned();

            map.entry(target_oid.to_string())
                .or_default()
                .push(shorthand);
        }
    }

    map
}

/// Return the short refs that point at `target` without materialising the
/// full repository-wide ref map. O(refs) but with a single pass and no map
/// allocation per OID — used by [`Repository::get_commit`] which only needs
/// the refs of a single commit.
fn refs_for_oid(repo: &git2::Repository, target: git2::Oid) -> Vec<String> {
    let Ok(references) = repo.references() else {
        return Vec::new();
    };

    let mut hits = Vec::new();
    for reference in references.flatten() {
        let oid = match reference.target() {
            Some(oid) => oid,
            None => match reference.resolve().ok().and_then(|r| r.target()) {
                Some(oid) => oid,
                None => continue,
            },
        };
        if oid != target {
            continue;
        }
        hits.push(
            reference
                .shorthand()
                .unwrap_or_else(|| reference.name().unwrap_or("unknown"))
                .to_owned(),
        );
    }
    hits
}

impl Repository {
    /// Walk all commits reachable from any ref, skipping `offset` and returning at most `max_count`.
    ///
    /// Commits are returned in topological + time order (newest first). The first
    /// `offset` commits encountered during the walk are discarded before
    /// collecting up to `max_count` results, enabling pagination/windowing over
    /// the commit graph.
    pub fn walk_commits(
        &self,
        offset: usize,
        max_count: usize,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let repo = self.inner();
        let ref_map = build_ref_map(repo);

        let mut revwalk = repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;

        // Push all heads, remotes, and tags as starting points.
        if let Ok(refs) = repo.references() {
            for reference in refs.flatten() {
                if let Some(oid) = reference.target() {
                    // Only push commit-ish objects; ignore failures silently.
                    let _ = revwalk.push(oid);
                }
            }
        }

        let mut commits = Vec::new();

        for (i, oid_result) in revwalk.enumerate() {
            if i < offset {
                continue;
            }
            if commits.len() >= max_count {
                break;
            }

            let oid = oid_result?;
            let commit = repo.find_commit(oid)?;
            commits.push(commit_to_info(&commit, oid, &ref_map));
        }

        Ok(commits)
    }

    /// Walk commits filtered by criteria, skipping `offset` raw revwalk entries.
    ///
    /// Returns commits matching ALL filters in topological + time order
    /// (newest first). The first `offset` commits produced by the revwalk are
    /// discarded before filtering; up to `max_count` matching commits are
    /// returned afterwards. This enables windowed/paginated scans over filtered
    /// history.
    pub fn walk_commits_filtered(
        &self,
        offset: usize,
        max_count: usize,
        branch_filter: Option<&str>,
        author_filter: Option<&str>,
        message_filter: Option<&str>,
        sha_filter: Option<&str>,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let repo = self.inner();
        let ref_map = build_ref_map(repo);
        let mut revwalk = repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;

        if let Some(branch) = branch_filter {
            // Push only refs matching the branch filter
            let mut found = false;
            for reference in repo.references()? {
                let reference = reference?;
                if let Some(name) = reference.shorthand()
                    && name.to_lowercase().contains(&branch.to_lowercase())
                    && let Some(target) = reference.target()
                {
                    revwalk.push(target)?;
                    found = true;
                }
            }
            if !found {
                return Ok(vec![]); // No matching branches
            }
        } else {
            // Push all refs
            revwalk.push_glob("refs/heads/*")?;
            revwalk.push_glob("refs/remotes/*")?;
            revwalk.push_glob("refs/tags/*")?;
        }

        let mut commits = Vec::with_capacity(max_count);
        for (i, oid_result) in revwalk.enumerate() {
            if i < offset {
                continue;
            }
            if commits.len() >= max_count {
                break;
            }
            let oid = oid_result?;
            let commit = repo.find_commit(oid)?;

            // Apply author filter
            if let Some(author) = author_filter {
                let sig = commit.author();
                let name = sig.name().unwrap_or("");
                if !name.to_lowercase().contains(&author.to_lowercase()) {
                    continue;
                }
            }

            // Apply message filter
            if let Some(msg) = message_filter {
                let summary = commit.summary().unwrap_or("");
                if !summary.to_lowercase().contains(&msg.to_lowercase()) {
                    continue;
                }
            }

            // Apply SHA filter
            if let Some(sha) = sha_filter
                && !oid.to_string().starts_with(&sha.to_lowercase())
            {
                continue;
            }

            commits.push(commit_to_info(&commit, oid, &ref_map));
        }

        Ok(commits)
    }

    /// Walk commits reachable from a specific branch, up to `limit`.
    ///
    /// Resolves the branch name to a ref and walks from its tip.
    /// Works for both local (`main`) and remote (`origin/main`) branches.
    pub fn branch_commits(
        &self,
        branch_name: &str,
        limit: usize,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let repo = self.inner();
        let ref_map = build_ref_map(repo);

        // Try local, then remote ref
        let reference = repo
            .find_reference(&format!("refs/heads/{branch_name}"))
            .or_else(|_| repo.find_reference(&format!("refs/remotes/{branch_name}")))?;

        let oid = reference
            .target()
            .ok_or_else(|| GitError::Git(git2::Error::from_str("Symbolic ref without target")))?;

        let mut revwalk = repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        revwalk.push(oid)?;

        let mut commits = Vec::new();
        for oid_result in revwalk {
            if commits.len() >= limit {
                break;
            }
            let oid = oid_result?;
            let commit = repo.find_commit(oid)?;
            commits.push(commit_to_info(&commit, oid, &ref_map));
        }
        Ok(commits)
    }

    /// Retrieve a single commit by its OID string.
    pub fn get_commit(&self, oid_str: &str) -> Result<CommitInfo, GitError> {
        let repo = self.inner();
        let oid = git2::Oid::from_str(oid_str)?;
        let commit = repo.find_commit(oid)?;
        // Avoid `build_ref_map` here — it walks every reference in the repo
        // even though we only need the refs touching this single OID. On
        // repos with many tags, click-to-load-commit was O(refs).
        let mut ref_map: HashMap<String, Vec<String>> = HashMap::new();
        let refs = refs_for_oid(repo, oid);
        if !refs.is_empty() {
            ref_map.insert(oid.to_string(), refs);
        }
        Ok(commit_to_info(&commit, oid, &ref_map))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::create_repo_with_n_commits;

    #[test]
    fn test_walk_commits_returns_all() {
        let (_dir, path) = create_repo_with_n_commits(5);
        let repo = Repository::open(&path).unwrap();

        let commits = repo.walk_commits(0, 100).unwrap();
        assert_eq!(commits.len(), 5, "should return all 5 commits");
    }

    #[test]
    fn test_walk_commits_respects_offset() {
        let (_dir, path) = create_repo_with_n_commits(10);
        let repo = Repository::open(&path).unwrap();

        let all = repo.walk_commits(0, 100).unwrap();
        let skipped = repo.walk_commits(3, 100).unwrap();
        assert_eq!(all.len(), 10);
        assert_eq!(skipped.len(), 7);
        assert_eq!(
            skipped[0].oid, all[3].oid,
            "offset should skip the first 3 commits"
        );
    }

    #[test]
    fn test_walk_commits_offset_beyond_total_returns_empty() {
        let (_dir, path) = create_repo_with_n_commits(5);
        let repo = Repository::open(&path).unwrap();

        let result = repo.walk_commits(100, 100).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_walk_commits_respects_max_count() {
        let (_dir, path) = create_repo_with_n_commits(10);
        let repo = Repository::open(&path).unwrap();

        let commits = repo.walk_commits(0, 3).unwrap();
        assert_eq!(commits.len(), 3, "should respect max_count of 3");
    }

    #[test]
    fn test_walk_commits_has_parent_info() {
        let (_dir, path) = create_repo_with_n_commits(3);
        let repo = Repository::open(&path).unwrap();

        let commits = repo.walk_commits(0, 100).unwrap();
        assert_eq!(commits.len(), 3);

        // Newest commit (index 0) should have one parent
        assert_eq!(commits[0].parents.len(), 1, "newest commit has 1 parent");
        // Middle commit should also have one parent
        assert_eq!(commits[1].parents.len(), 1, "middle commit has 1 parent");
        // Root commit should have no parents
        assert_eq!(commits[2].parents.len(), 0, "root commit has 0 parents");

        // Parent of commit[0] should be OID of commit[1]
        assert_eq!(commits[0].parents[0], commits[1].oid);
    }

    #[test]
    fn test_get_commit_by_oid() {
        let (_dir, path) = create_repo_with_n_commits(3);
        let repo = Repository::open(&path).unwrap();

        let commits = repo.walk_commits(0, 100).unwrap();
        assert!(!commits.is_empty());

        // Pick the middle commit
        let target = &commits[1];
        let fetched = repo.get_commit(&target.oid).unwrap();

        assert_eq!(fetched.oid, target.oid);
        assert_eq!(fetched.summary, target.summary);
        assert_eq!(fetched.author, target.author);
        assert_eq!(fetched.parents, target.parents);
    }

    #[test]
    fn test_walk_commits_filtered_by_author() {
        let (_dir, path) = create_repo_with_n_commits(3);
        let repo = Repository::open(&path).unwrap();

        // Filter by existing author
        let commits = repo
            .walk_commits_filtered(0, 100, None, Some("Test User"), None, None)
            .unwrap();
        assert_eq!(commits.len(), 3, "all commits are by Test User");

        // Filter by nonexistent author
        let commits = repo
            .walk_commits_filtered(0, 100, None, Some("Nonexistent"), None, None)
            .unwrap();
        assert!(commits.is_empty(), "no commits by Nonexistent");
    }

    #[test]
    fn test_walk_commits_filtered_by_message() {
        let (_dir, path) = create_repo_with_n_commits(5);
        let repo = Repository::open(&path).unwrap();

        // Filter by message substring matching a single commit
        let commits = repo
            .walk_commits_filtered(0, 100, None, None, Some("Commit 3"), None)
            .unwrap();
        assert_eq!(commits.len(), 1, "only one commit matches 'Commit 3'");
        assert_eq!(commits[0].summary, "Commit 3");

        // Filter by common substring matching all commits
        let commits = repo
            .walk_commits_filtered(0, 100, None, None, Some("Commit"), None)
            .unwrap();
        assert_eq!(commits.len(), 5, "all commits match 'Commit'");
    }

    #[test]
    fn test_walk_commits_filtered_by_sha() {
        let (_dir, path) = create_repo_with_n_commits(3);
        let repo = Repository::open(&path).unwrap();

        let all = repo.walk_commits(0, 100).unwrap();
        let target = &all[1];
        let sha_prefix = &target.oid[..8];

        let commits = repo
            .walk_commits_filtered(0, 100, None, None, None, Some(sha_prefix))
            .unwrap();
        assert_eq!(commits.len(), 1, "exactly one commit matches SHA prefix");
        assert_eq!(commits[0].oid, target.oid);
    }

    #[test]
    fn test_walk_commits_filtered_by_branch() {
        let (_dir, path) = create_repo_with_n_commits(2);
        let git_repo = git2::Repository::open(&path).unwrap();

        // Create a "feature" branch from HEAD
        let head_commit = git_repo
            .find_commit(git_repo.head().unwrap().target().unwrap())
            .unwrap();
        git_repo.branch("feature", &head_commit, false).unwrap();

        // Add one more commit on "feature"
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let tree = git_repo
            .find_tree(git_repo.index().unwrap().write_tree().unwrap())
            .unwrap();
        git_repo
            .commit(
                Some("refs/heads/feature"),
                &sig,
                &sig,
                "Feature commit",
                &tree,
                &[&head_commit],
            )
            .unwrap();

        let repo = Repository::open(&path).unwrap();
        let commits = repo
            .walk_commits_filtered(0, 100, Some("feature"), None, None, None)
            .unwrap();

        assert!(
            commits.len() >= 3,
            "feature branch should include its own commit plus inherited ones"
        );
        assert!(
            commits.iter().any(|c| c.summary == "Feature commit"),
            "should contain the feature-only commit"
        );
    }

    #[test]
    fn test_walk_commits_filtered_no_match() {
        let (_dir, path) = create_repo_with_n_commits(3);
        let repo = Repository::open(&path).unwrap();

        let commits = repo
            .walk_commits_filtered(
                0,
                100,
                Some("nonexistent-branch"),
                Some("Nobody"),
                Some("zzz"),
                Some("0000000"),
            )
            .unwrap();
        assert!(
            commits.is_empty(),
            "no commits should match all bad filters"
        );
    }

    #[test]
    fn test_branch_commits() {
        let (_dir, path) = create_repo_with_n_commits(2);
        let git_repo = git2::Repository::open(&path).unwrap();

        // Create a "feature" branch and add a commit to it
        let head_commit = git_repo
            .find_commit(git_repo.head().unwrap().target().unwrap())
            .unwrap();
        git_repo.branch("feature", &head_commit, false).unwrap();

        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let tree = git_repo
            .find_tree(git_repo.index().unwrap().write_tree().unwrap())
            .unwrap();
        git_repo
            .commit(
                Some("refs/heads/feature"),
                &sig,
                &sig,
                "Feature work",
                &tree,
                &[&head_commit],
            )
            .unwrap();

        let repo = Repository::open(&path).unwrap();
        let commits = repo.branch_commits("feature", 100).unwrap();

        assert_eq!(commits.len(), 3, "feature has 1 own + 2 inherited commits");
        assert_eq!(commits[0].summary, "Feature work");
    }

    #[test]
    fn test_branch_commits_nonexistent() {
        let (_dir, path) = create_repo_with_n_commits(2);
        let repo = Repository::open(&path).unwrap();

        let result = repo.branch_commits("nonexistent", 100);
        assert!(result.is_err(), "nonexistent branch should return an error");
    }
}

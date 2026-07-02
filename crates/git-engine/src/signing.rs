//! Commit/tag signing: config detection, signed-commit routing, and
//! signature inspection.
//!
//! BeardGit honors the user's existing git signing configuration
//! (`commit.gpgsign`, `gpg.format`, `user.signingkey`, `tag.gpgSign`).
//! When signing is enabled, commit creation is routed through the system
//! `git` CLI so ssh/gpg/x509 backends and their agents work natively —
//! `libgit2` cannot sign. When signing is off the byte-identical `git2`
//! path is used (see [`Repository::create_commit`]).
//!
//! Signature *inspection* is split by cost: presence
//! ([`Repository::commit_signature`]) is a cheap `git2` object read;
//! verification ([`Repository::verify_commit_signature`]) shells to
//! `git verify-commit` and is meant to run lazily for a single commit.

use std::process::Command;

use serde::Serialize;
use tracing::instrument;

use crate::cli::{GitCliResult, configure_no_window};
use crate::error::GitError;
use crate::repository::Repository;

/// The signature backend configured via `gpg.format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SigningFormat {
    /// OpenPGP / GnuPG (the git default when `gpg.format` is unset).
    Gpg,
    /// SSH key signing (`gpg.format=ssh`).
    Ssh,
    /// X.509 / S/MIME (`gpg.format=x509`).
    X509,
}

impl SigningFormat {
    /// Parse a `gpg.format` config value; anything unrecognised (including
    /// the absent/`openpgp` default) maps to [`SigningFormat::Gpg`].
    fn from_config(value: Option<&str>) -> Self {
        match value {
            Some("ssh") => SigningFormat::Ssh,
            Some("x509") => SigningFormat::X509,
            _ => SigningFormat::Gpg,
        }
    }
}

/// The user's effective signing configuration, read from merged git config.
#[derive(Debug, Clone, Serialize)]
pub struct SigningConfig {
    /// `commit.gpgsign` — whether new commits should be signed.
    pub enabled: bool,
    /// `tag.gpgSign` — whether annotated tags should be signed.
    pub tag_enabled: bool,
    /// `gpg.format` — the signature backend.
    pub format: SigningFormat,
    /// `user.signingkey` — the configured key (path or key id), if any.
    pub signing_key: Option<String>,
}

/// Diagnostic signing status for the UI (commit-box chip + settings).
#[derive(Debug, Clone, Serialize)]
pub struct SigningStatus {
    /// Whether `commit.gpgsign` is on.
    pub enabled: bool,
    /// The configured signature backend.
    pub format: SigningFormat,
    /// Best-effort check that the configured key is usable — file exists for
    /// ssh key paths, `gpg --list-secret-keys` hit for gpg. Diagnostic only:
    /// it NEVER blocks committing, it only drives a hint in settings.
    pub key_present: bool,
}

/// Presence (not validity) of a commit's embedded signature.
#[derive(Debug, Clone, Serialize)]
pub struct CommitSignature {
    /// `true` when the commit object carries a `gpgsig` header.
    pub present: bool,
    /// Cheap format hint sniffed from the signature armor (`"gpg"`, `"ssh"`,
    /// or `"x509"`); `None` when unsigned.
    pub format: Option<String>,
}

/// Result of a lazy `git verify-commit` for a single commit.
#[derive(Debug, Clone, Serialize)]
pub struct SignatureVerification {
    /// One of `"verified"`, `"unverified"`, or `"unsigned"`.
    pub status: String,
    /// Human-readable detail from git/gpg/ssh (empty for `"unsigned"`).
    pub detail: String,
}

/// Result of the "Test signing" diagnostic.
#[derive(Debug, Clone, Serialize)]
pub struct SigningTestResult {
    /// `true` when a throwaway commit was signed successfully.
    pub success: bool,
    /// On failure, the exact git/gpg/ssh stderr; on success, a short note.
    pub message: String,
}

impl Repository {
    /// Read the effective signing configuration from merged git config.
    ///
    /// Uses `git2` (which merges system/global/local scopes) so a missing key
    /// resolves to its git default rather than erroring.
    pub fn signing_config(&self) -> Result<SigningConfig, GitError> {
        let config = self.inner().config()?;
        let enabled = config.get_bool("commit.gpgsign").unwrap_or(false);
        let tag_enabled = config.get_bool("tag.gpgSign").unwrap_or(false);
        let format = SigningFormat::from_config(config.get_string("gpg.format").ok().as_deref());
        let signing_key = config
            .get_string("user.signingkey")
            .ok()
            .filter(|s| !s.is_empty());
        Ok(SigningConfig {
            enabled,
            tag_enabled,
            format,
            signing_key,
        })
    }

    /// Diagnostic signing status for the UI. Never fails the commit path.
    pub fn signing_status(&self) -> Result<SigningStatus, GitError> {
        let cfg = self.signing_config()?;
        let key_present = match (&cfg.signing_key, cfg.format) {
            (None, _) => false,
            (Some(key), SigningFormat::Ssh) => ssh_key_present(key),
            (Some(key), SigningFormat::Gpg) => gpg_key_present(key),
            // X.509 keys live in the platform cert store; presence is not
            // cheaply checkable, so report configured-as-present.
            (Some(_), SigningFormat::X509) => true,
        };
        Ok(SigningStatus {
            enabled: cfg.enabled,
            format: cfg.format,
            key_present,
        })
    }

    /// Run `git commit …` non-interactively so the user's signing config is
    /// honored. `base_args` is e.g. `["commit"]` or `["commit", "--amend"]`.
    ///
    /// `GIT_TERMINAL_PROMPT=0` guarantees a locked key fails fast instead of
    /// hanging the app on a passphrase prompt (there is no controlling TTY);
    /// agent-based setups (ssh-agent / gpg-agent) are the supported mode.
    fn commit_via_cli(&self, base_args: &[&str], message: &str) -> Result<GitCliResult, GitError> {
        let mut args: Vec<&str> = base_args.to_vec();
        args.push("-m");
        args.push(message);
        self.git_cmd_with_env(&args, &[("GIT_TERMINAL_PROMPT", "0")])
    }

    /// Create a commit through the git CLI, honoring signing config.
    ///
    /// Used by [`Repository::create_commit`] when `commit.gpgsign` is on.
    #[instrument(skip(self))]
    pub fn create_commit_signed(&self, message: &str) -> Result<String, GitError> {
        let result = self.commit_via_cli(&["commit"], message)?;
        if !result.success {
            return Err(GitError::SigningFailed(result.stderr));
        }
        let head = self.git_cmd(&["rev-parse", "HEAD"])?;
        Ok(head.stdout.trim().to_string())
    }

    /// Amend HEAD through the git CLI, honoring signing config.
    ///
    /// When signing is enabled a failure surfaces as
    /// [`GitError::SigningFailed`]; otherwise (plain amend) it stays a
    /// [`GitError::CliError`] so existing callers see no change.
    #[instrument(skip(self))]
    pub fn amend_commit_cli(&self, message: &str) -> Result<(), GitError> {
        let signing = self.signing_config()?.enabled;
        let result = self.commit_via_cli(&["commit", "--amend"], message)?;
        if result.success {
            Ok(())
        } else if signing {
            Err(GitError::SigningFailed(result.stderr))
        } else {
            Err(GitError::CliError(result.stderr))
        }
    }

    /// Presence (not validity) of a commit's signature via `git2`.
    ///
    /// Cheap object read — safe to call for the commit open in the detail
    /// pane. Does NOT verify the signature (see
    /// [`Repository::verify_commit_signature`]).
    pub fn commit_signature(&self, oid_str: &str) -> Result<CommitSignature, GitError> {
        let oid = git2::Oid::from_str(oid_str)?;
        match self.inner().extract_signature(&oid, None) {
            Ok((sig, _)) => {
                let armor = String::from_utf8_lossy(&sig);
                let format = if armor.contains("SSH SIGNATURE") {
                    "ssh"
                } else if armor.contains("PGP") {
                    "gpg"
                } else {
                    "x509"
                };
                Ok(CommitSignature {
                    present: true,
                    format: Some(format.to_string()),
                })
            }
            // No signature header → unsigned (not an error).
            Err(_) => Ok(CommitSignature {
                present: false,
                format: None,
            }),
        }
    }

    /// Lazily verify a single commit's signature by shelling to
    /// `git verify-commit`.
    ///
    /// Returns `"unsigned"` (no signature present), `"verified"` (git's
    /// verdict is good), or `"unverified"` (a signature is present but git
    /// could not validate it — e.g. no `gpg.ssh.allowedSignersFile`, unknown
    /// key, expired cert). Renders git's verdict; it does not manage trust.
    #[instrument(skip(self), fields(oid = %oid_str))]
    pub fn verify_commit_signature(
        &self,
        oid_str: &str,
    ) -> Result<SignatureVerification, GitError> {
        // Presence first, so an unsigned commit doesn't pay for a subprocess.
        if !self.commit_signature(oid_str)?.present {
            return Ok(SignatureVerification {
                status: "unsigned".to_string(),
                detail: String::new(),
            });
        }
        let result =
            self.git_cmd_with_env(&["verify-commit", oid_str], &[("GIT_TERMINAL_PROMPT", "0")])?;
        // `git verify-commit` prints the gpg/ssh verdict to stderr in both
        // the good and bad cases.
        let detail = result.stderr.trim().to_string();
        let status = if result.success {
            "verified"
        } else {
            "unverified"
        };
        Ok(SignatureVerification {
            status: status.to_string(),
            detail,
        })
    }

    /// Exercise the user's effective signing config end-to-end without
    /// touching this repository: replicate the relevant keys into a throwaway
    /// temp repo and produce a signed empty commit there.
    pub fn test_signing(&self) -> Result<SigningTestResult, GitError> {
        let config = self.inner().config()?;
        // Copy every key that shapes signing so the temp repo signs exactly
        // as this repo would. Missing keys are simply skipped.
        let copy_keys = [
            "user.name",
            "user.email",
            "user.signingkey",
            "gpg.format",
            "gpg.program",
            "gpg.ssh.program",
            "gpg.ssh.allowedSignersFile",
            "gpg.x509.program",
        ];

        let tmp = tempfile::tempdir().map_err(GitError::Io)?;
        let dir = tmp.path();

        run_git(dir, &["init", "-q"])?;
        // Identity is required for `git commit`; fall back to a placeholder so
        // the test reports a *signing* error rather than a missing-identity one.
        let mut have_name = false;
        let mut have_email = false;
        for key in copy_keys {
            if let Ok(value) = config.get_string(key)
                && !value.is_empty()
            {
                run_git(dir, &["config", key, &value])?;
                have_name |= key == "user.name";
                have_email |= key == "user.email";
            }
        }
        if !have_name {
            run_git(dir, &["config", "user.name", "BeardGit"])?;
        }
        if !have_email {
            run_git(dir, &["config", "user.email", "signing-test@beardgit"])?;
        }

        // `-S` forces signing so this tests the key even when commit.gpgsign
        // is off (a pure "does my key work?" diagnostic).
        let result = run_git_env(
            dir,
            &[
                "commit",
                "--allow-empty",
                "-S",
                "-m",
                "BeardGit signing test",
            ],
            &[("GIT_TERMINAL_PROMPT", "0")],
        )?;
        Ok(SigningTestResult {
            success: result.success,
            message: if result.success {
                "Signature produced successfully.".to_string()
            } else {
                result.stderr.trim().to_string()
            },
        })
    }
}

/// Whether an ssh `user.signingkey` value resolves to something usable.
///
/// A literal key (`ssh-ed25519 AAAA…`) is present as-is; otherwise the value
/// is a path — expand a leading `~` and check the file exists.
fn ssh_key_present(key: &str) -> bool {
    if key.starts_with("ssh-") {
        return true;
    }
    let expanded = expand_tilde(key);
    std::path::Path::new(&expanded).exists()
}

/// Whether gpg knows a secret key matching `user.signingkey`.
fn gpg_key_present(key: &str) -> bool {
    let mut cmd = Command::new("gpg");
    cmd.args(["--list-secret-keys", key]);
    configure_no_window(&mut cmd);
    cmd.output().map(|o| o.status.success()).unwrap_or(false)
}

/// Expand a leading `~/` to the user's home directory.
fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = home_dir()
    {
        return format!("{home}/{rest}");
    }
    path.to_string()
}

fn home_dir() -> Option<String> {
    std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok())
}

/// Run a git command in `dir`, erroring on a failed spawn.
fn run_git(dir: &std::path::Path, args: &[&str]) -> Result<GitCliResult, GitError> {
    run_git_env(dir, args, &[])
}

/// Run a git command in `dir` with extra environment variables.
fn run_git_env(
    dir: &std::path::Path,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<GitCliResult, GitError> {
    let mut cmd = Command::new("git");
    cmd.args(args).current_dir(dir);
    for (k, v) in env_vars {
        cmd.env(k, v);
    }
    configure_no_window(&mut cmd);
    let output = cmd.output().map_err(GitError::Io)?;
    Ok(GitCliResult {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// Skip a test gracefully when `ssh-keygen` is unavailable (keeps the
    /// suite green on minimal environments; CI has it on all three OSes).
    fn ssh_keygen_available() -> bool {
        Command::new("ssh-keygen")
            .arg("-A")
            .arg("-h")
            .output()
            .is_ok()
    }

    /// Build a repo with one commit and an ssh signing config wired to a
    /// freshly-generated throwaway ed25519 key. Returns the temp dir (kept
    /// alive), the open repo, and the public-key path (for allowed-signers).
    fn repo_with_ssh_signing(gpgsign: bool) -> (tempfile::TempDir, Repository, std::path::PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        // Throwaway ed25519 key with no passphrase (agent-free, no prompt).
        let key_path = dir.join("id_test");
        let out = Command::new("ssh-keygen")
            .args(["-t", "ed25519", "-N", "", "-C", "beardgit-test", "-f"])
            .arg(&key_path)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "ssh-keygen failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let pub_path = dir.join("id_test.pub");

        let git = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(dir)
                .output()
                .unwrap()
        };
        git(&["init", "-q"]);
        git(&["config", "user.name", "Test"]);
        git(&["config", "user.email", "test@test.com"]);
        git(&["config", "core.autocrlf", "false"]);
        git(&["config", "gpg.format", "ssh"]);
        git(&["config", "user.signingkey", key_path.to_str().unwrap()]);
        // allowedSignersFile is only needed for verify-commit; wire it so the
        // verification-path test can pass where the environment allows.
        let signers = dir.join("allowed_signers");
        let pubkey = std::fs::read_to_string(&pub_path).unwrap();
        std::fs::write(&signers, format!("test@test.com {}", pubkey.trim())).unwrap();
        git(&[
            "config",
            "gpg.ssh.allowedSignersFile",
            signers.to_str().unwrap(),
        ]);
        if gpgsign {
            git(&["config", "commit.gpgsign", "true"]);
        }

        std::fs::write(dir.join("f.txt"), "hello\n").unwrap();
        git(&["add", "."]);
        // Initial commit unsigned so tests control which commit is signed.
        git(&["commit", "-q", "--no-gpg-sign", "-m", "init"]);

        let repo = Repository::open(dir).unwrap();
        (tmp, repo, pub_path)
    }

    #[test]
    fn signing_config_reads_ssh_settings() {
        if !ssh_keygen_available() {
            eprintln!("skipping: ssh-keygen unavailable");
            return;
        }
        let (_tmp, repo, _pub) = repo_with_ssh_signing(true);
        let cfg = repo.signing_config().unwrap();
        assert!(cfg.enabled, "commit.gpgsign=true should be read as enabled");
        assert_eq!(cfg.format, SigningFormat::Ssh);
        assert!(cfg.signing_key.is_some());
    }

    #[test]
    fn signing_config_defaults_when_absent() {
        // A plain repo with no signing config: disabled, gpg default, no key.
        let tmp = tempfile::tempdir().unwrap();
        let git = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(tmp.path())
                .output()
                .unwrap()
        };
        git(&["init", "-q"]);
        let repo = Repository::open(tmp.path()).unwrap();
        let cfg = repo.signing_config().unwrap();
        assert!(!cfg.enabled);
        assert_eq!(cfg.format, SigningFormat::Gpg);
        assert!(cfg.signing_key.is_none());
    }

    #[test]
    fn create_commit_signs_when_gpgsign_enabled() {
        if !ssh_keygen_available() {
            eprintln!("skipping: ssh-keygen unavailable");
            return;
        }
        let (tmp, repo, _pub) = repo_with_ssh_signing(true);
        std::fs::write(tmp.path().join("f.txt"), "changed\n").unwrap();
        repo.stage_files(&["f.txt".to_string()]).unwrap();
        let oid = repo.create_commit("signed commit").unwrap();

        // The new HEAD carries a signature.
        let sig = repo.commit_signature(&oid).unwrap();
        assert!(sig.present, "commit created under gpgsign should be signed");
        assert_eq!(sig.format.as_deref(), Some("ssh"));
    }

    #[test]
    fn create_commit_unsigned_when_gpgsign_disabled() {
        if !ssh_keygen_available() {
            eprintln!("skipping: ssh-keygen unavailable");
            return;
        }
        // Signing config present (key + format) but commit.gpgsign=false → the
        // git2 path is used and the commit is NOT signed.
        let (tmp, repo, _pub) = repo_with_ssh_signing(false);
        std::fs::write(tmp.path().join("f.txt"), "changed\n").unwrap();
        repo.stage_files(&["f.txt".to_string()]).unwrap();
        let oid = repo.create_commit("plain commit").unwrap();

        let sig = repo.commit_signature(&oid).unwrap();
        assert!(!sig.present, "gpgsign=false must not sign");
    }

    #[test]
    fn amend_keeps_signature_when_enabled() {
        if !ssh_keygen_available() {
            eprintln!("skipping: ssh-keygen unavailable");
            return;
        }
        let (tmp, repo, _pub) = repo_with_ssh_signing(true);
        std::fs::write(tmp.path().join("f.txt"), "changed\n").unwrap();
        repo.stage_files(&["f.txt".to_string()]).unwrap();
        let oid = repo.create_commit("to amend").unwrap();
        assert!(repo.commit_signature(&oid).unwrap().present);

        repo.amend_commit_cli("amended message").unwrap();
        let head = repo.git_cmd(&["rev-parse", "HEAD"]).unwrap();
        let new_oid = head.stdout.trim();
        assert_eq!(repo.get_head_message().unwrap().trim(), "amended message");
        assert!(
            repo.commit_signature(new_oid).unwrap().present,
            "amend under gpgsign should keep the commit signed"
        );
    }

    #[test]
    fn merge_commit_is_signed_via_config() {
        if !ssh_keygen_available() {
            eprintln!("skipping: ssh-keygen unavailable");
            return;
        }
        // Merge goes through the git CLI, so a non-fast-forward merge commit
        // is signed automatically when commit.gpgsign is on.
        let (tmp, repo, _pub) = repo_with_ssh_signing(true);
        let dir = tmp.path();
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(dir)
                .output()
                .unwrap()
        };
        // Diverge: a feature branch commit + a base commit so merge can't ff.
        // Capture the default branch name (init.defaultBranch varies by env).
        let default_branch = repo.get_current_branch().unwrap().unwrap();
        git(&["checkout", "-q", "-b", "feature"]);
        std::fs::write(dir.join("feat.txt"), "feat\n").unwrap();
        repo.stage_files(&["feat.txt".to_string()]).unwrap();
        repo.create_commit("feature work").unwrap();
        git(&["checkout", "-q", &default_branch]);
        std::fs::write(dir.join("base.txt"), "base\n").unwrap();
        repo.stage_files(&["base.txt".to_string()]).unwrap();
        repo.create_commit("base work").unwrap();

        let result = repo.merge_branch("feature").unwrap();
        assert!(result.success, "merge should succeed: {}", result.stderr);
        let head = repo.git_cmd(&["rev-parse", "HEAD"]).unwrap();
        let merge_oid = head.stdout.trim();
        assert_eq!(
            repo.get_commit(merge_oid).unwrap().parents.len(),
            2,
            "expected a real (non-ff) merge commit"
        );
        assert!(
            repo.commit_signature(merge_oid).unwrap().present,
            "merge commit under gpgsign should be signed"
        );
    }

    #[test]
    fn revert_commit_is_signed_via_config() {
        if !ssh_keygen_available() {
            eprintln!("skipping: ssh-keygen unavailable");
            return;
        }
        // Revert also shells out, so its new commit signs via config.
        let (tmp, repo, _pub) = repo_with_ssh_signing(true);
        std::fs::write(tmp.path().join("f.txt"), "v2\n").unwrap();
        repo.stage_files(&["f.txt".to_string()]).unwrap();
        let target = repo.create_commit("change to revert").unwrap();

        let result = repo.revert_commit(&target).unwrap();
        assert!(result.success, "revert should succeed: {}", result.stderr);
        let head = repo.git_cmd(&["rev-parse", "HEAD"]).unwrap();
        assert!(
            repo.commit_signature(head.stdout.trim()).unwrap().present,
            "revert commit under gpgsign should be signed"
        );
    }

    #[test]
    fn verify_commit_reports_unsigned() {
        if !ssh_keygen_available() {
            eprintln!("skipping: ssh-keygen unavailable");
            return;
        }
        let (_tmp, repo, _pub) = repo_with_ssh_signing(true);
        // The init commit was created --no-gpg-sign.
        let head = repo.git_cmd(&["rev-parse", "HEAD"]).unwrap();
        let v = repo.verify_commit_signature(head.stdout.trim()).unwrap();
        assert_eq!(v.status, "unsigned");
    }

    #[test]
    fn verify_commit_on_signed_reports_verdict() {
        if !ssh_keygen_available() {
            eprintln!("skipping: ssh-keygen unavailable");
            return;
        }
        let (tmp, repo, _pub) = repo_with_ssh_signing(true);
        std::fs::write(tmp.path().join("f.txt"), "changed\n").unwrap();
        repo.stage_files(&["f.txt".to_string()]).unwrap();
        let oid = repo.create_commit("signed commit").unwrap();

        // With allowedSignersFile wired to our key, git should verify it as
        // good on environments where ssh signature verification is supported.
        // We assert the verdict is one of the two *signed* states (never
        // "unsigned"), since older git builds may lack ssh verify support.
        let v = repo.verify_commit_signature(&oid).unwrap();
        assert!(
            v.status == "verified" || v.status == "unverified",
            "signed commit must not report unsigned, got {:?}",
            v
        );
    }

    #[test]
    fn annotated_tag_is_signed_when_tag_gpgsign_set() {
        if !ssh_keygen_available() {
            eprintln!("skipping: ssh-keygen unavailable");
            return;
        }
        // No code change signs tags: annotated-tag creation goes through the
        // git CLI, which honors tag.gpgSign. This locks that behavior.
        let (_tmp, repo, _pub) = repo_with_ssh_signing(false);
        repo.set_config(crate::ConfigScope::Local, "tag.gpgSign", "true")
            .unwrap();

        let result = repo.create_tag("v1.0.0", Some("release")).unwrap();
        assert!(result.success, "tag creation failed: {}", result.stderr);

        // The tag object carries a signature block.
        let obj = repo.git_cmd(&["cat-file", "tag", "v1.0.0"]).unwrap();
        assert!(
            obj.stdout.contains("SSH SIGNATURE") || obj.stdout.contains("PGP SIGNATURE"),
            "annotated tag under tag.gpgSign should be signed, got:\n{}",
            obj.stdout
        );
    }

    #[test]
    fn test_signing_succeeds_with_valid_ssh_key() {
        if !ssh_keygen_available() {
            eprintln!("skipping: ssh-keygen unavailable");
            return;
        }
        let (_tmp, repo, _pub) = repo_with_ssh_signing(false);
        let result = repo.test_signing().unwrap();
        assert!(
            result.success,
            "test_signing should sign with a valid ssh key, got: {}",
            result.message
        );
    }
}

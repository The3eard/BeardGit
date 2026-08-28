//! Structured file logging with daily rotation.
//!
//! Writes logs to platform-specific directories:
//! - macOS: `~/Library/Logs/BeardGit/`
//! - Linux: `~/.local/share/beardgit/logs/`
//! - Windows: `%APPDATA%/BeardGit/logs/`

use std::borrow::Cow;
use std::io;
use std::path::PathBuf;
use std::sync::OnceLock;

use regex::Regex;
use tracing_subscriber::fmt::format::{DefaultFields, FmtSpan, Format, Full};
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::registry::Registry;
use tracing_subscriber::{
    EnvFilter, Layer, fmt, layer::SubscriberExt, reload, util::SubscriberInitExt,
};

/// Maximum number of rotated log files retained on disk. Older days are
/// dropped automatically by `tracing_appender`.
const MAX_LOG_FILES: usize = 14;

/// The log levels the Settings selector offers, coarsest first.
///
/// Deliberately three, not the full `tracing` ladder: `error` for "only
/// tell me when something broke", `info` for the default narrative
/// (project open/close, mutations, network tasks), and `debug` for the
/// per-command detail a bug report needs.
pub const LOG_LEVELS: &[&str] = &["error", "info", "debug"];

/// Workspace crates promoted to `debug` when the user selects `debug`.
///
/// An allowlist rather than a bare `debug` directive: a global `debug`
/// drags in `hyper`, `tao`, `wry`, and `notify`, which bury our own
/// events under transport and event-loop chatter.
const WORKSPACE_TARGETS: &[&str] = &[
    "beardgit",
    "beardgit_lib",
    "app_core",
    "git_engine",
    "graph_builder",
    "forge_provider",
    "cli_provider",
    "provider",
    "github_api",
    "gitlab_api",
    "ai_provider",
    "ai_provider_common",
    "ai_runner",
    "claude_code",
    "codex",
    "opencode",
    "auth",
    "storage",
    "task_runner",
    "terminal",
    "watcher",
    "mutation_events",
    "requests_runner",
    "requests_store",
];

/// The concrete file layer type, needed to name the reload handle.
///
/// The writer is boxed rather than generic so this stays a single nameable
/// type — that is what lets the reload handle live in a `static`, and what
/// lets tests install an in-memory writer through the same code path.
type FileFmtLayer = fmt::Layer<Registry, DefaultFields, Format<Full>, BoxMakeWriter>;

/// Rebuilds a writer on demand so [`set_level`] can construct a fresh fmt
/// layer against the same sink. `BoxMakeWriter` is not `Clone`, so we keep
/// the factory rather than the writer.
type WriterFactory = Box<dyn Fn() -> BoxMakeWriter + Send + Sync>;

static WRITER_FACTORY: OnceLock<WriterFactory> = OnceLock::new();

/// Swaps the fmt layer, which is what changes span-event rendering.
static FMT_HANDLE: OnceLock<reload::Handle<FileFmtLayer, Registry>> = OnceLock::new();

/// Swaps the level filter.
///
/// **Two handles, not one, and this is load-bearing.** The obvious design
/// — wrapping the whole `Filtered<fmt, EnvFilter>` in one `reload::Layer`
/// — is broken: `Filtered::new` starts with `FilterId::disabled()` and
/// only receives its real id from `Layer::on_layer`, which runs once at
/// subscriber construction. `Handle::reload` swaps the value without
/// re-running it, so the replacement keeps the disabled id and the next
/// event trips a `debug_assert!` ("a `Filtered` layer was used, but it had
/// no `FilterId`"). Reloading the fmt layer and the filter *separately*
/// means the `Filtered` wrapper itself is built exactly once and keeps its
/// id for the life of the process.
static FILTER_HANDLE: OnceLock<reload::Handle<EnvFilter, Registry>> = OnceLock::new();

/// Canonicalize a user-supplied level string, or `None` if unrecognized.
///
/// Callers treat `None` as "reject" rather than "fall back", so a typo in
/// `settings.json` surfaces instead of silently logging at the wrong
/// verbosity.
pub fn normalize_level(level: &str) -> Option<&'static str> {
    match level.trim().to_ascii_lowercase().as_str() {
        "error" => Some("error"),
        "info" => Some("info"),
        "debug" => Some("debug"),
        _ => None,
    }
}

/// Build the `EnvFilter` directive string for a normalized level.
fn directives_for(level: &str) -> String {
    match level {
        "error" => "error".to_string(),
        "debug" => {
            let mut s = String::from("info");
            for target in WORKSPACE_TARGETS {
                s.push(',');
                s.push_str(target);
                s.push_str("=debug");
            }
            s
        }
        _ => "info".to_string(),
    }
}

/// Build a fresh fmt layer for `level`.
///
/// Span close events are enabled only at `debug`: the ~165
/// `#[instrument]` attributes across the workspace default to INFO
/// spans, so emitting their close events at `info` would put a timing
/// line under every single IPC command.
fn build_fmt_layer(level: &str, writer: BoxMakeWriter) -> FileFmtLayer {
    let span_events = if level == "debug" {
        FmtSpan::CLOSE
    } else {
        FmtSpan::NONE
    };
    fmt::layer()
        .with_writer(writer)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_span_events(span_events)
}

/// Install the reloadable subscriber. Shared by [`init_logging`] and the
/// test entry point so both exercise the same wiring.
fn install(level: &str, filter: EnvFilter, factory: WriterFactory) -> Result<(), String> {
    let _ = WRITER_FACTORY.set(factory);
    let make_writer = WRITER_FACTORY.get().ok_or("writer factory unavailable")?();

    let (fmt_layer, fmt_handle) = reload::Layer::new(build_fmt_layer(level, make_writer));
    let (filter_layer, filter_handle) = reload::Layer::new(filter);
    let _ = FMT_HANDLE.set(fmt_handle);
    let _ = FILTER_HANDLE.set(filter_handle);

    tracing_subscriber::registry()
        .with(fmt_layer.with_filter(filter_layer))
        .try_init()
        .map_err(|e| e.to_string())
}

/// Change the active log level without restarting the app.
///
/// Swaps the filter and the fmt layer (span-event config) through their
/// reload handles. Each swap rebuilds tracing's callsite interest cache,
/// so callsites registered before the change start or stop emitting
/// immediately rather than keeping their original verdict.
///
/// Unlike [`init_logging`], this ignores `RUST_LOG`: an explicit choice
/// in Settings must take effect even in a dev shell that exports it.
///
/// # Errors
/// Returns an error for an unrecognized level, or if called before
/// [`init_logging`] succeeded.
pub fn set_level(level: &str) -> Result<(), String> {
    let level = normalize_level(level).ok_or_else(|| format!("unknown log level: {level}"))?;
    let fmt_handle = FMT_HANDLE.get().ok_or("logging is not initialized")?;
    let filter_handle = FILTER_HANDLE.get().ok_or("logging is not initialized")?;
    let factory = WRITER_FACTORY.get().ok_or("logging is not initialized")?;

    fmt_handle
        .reload(build_fmt_layer(level, factory()))
        .map_err(|e| e.to_string())?;
    filter_handle
        .reload(EnvFilter::new(directives_for(level)))
        .map_err(|e| e.to_string())?;

    tracing::info!(level, "log level changed");
    Ok(())
}

/// Install the subscriber against an arbitrary writer, for tests that need
/// to read back what was actually emitted.
///
/// Production goes through [`init_logging`]; this exists because the
/// reload path cannot be verified any other way — the real file appender
/// flushes on a background thread, and the `FilterId` bug this wiring
/// works around only shows up with a live subscriber. The writer is
/// wrapped in the same redaction layer production uses.
#[doc(hidden)]
pub fn init_with_writer<W>(level: &str, writer: W) -> Result<(), String>
where
    W: for<'a> fmt::MakeWriter<'a> + Clone + Send + Sync + 'static,
{
    let level = normalize_level(level).unwrap_or("info");
    install(
        level,
        EnvFilter::new(directives_for(level)),
        Box::new(move || {
            BoxMakeWriter::new(RedactingMakeWriter {
                inner: writer.clone(),
            })
        }),
    )
}

/// Compiled regex catching common credential shapes that may leak into log
/// streams (errors emitted by `git`/`gh`/`glab`, accidental debug prints, …).
///
/// Covers GitHub / GitLab personal access tokens, the `x-access-token` git
/// credential helper, `Authorization` headers, and `user:password@` segments
/// embedded in URLs. The match is intentionally conservative — we only redact
/// when a known prefix is present so unrelated identifiers are not mangled.
fn secret_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(concat!(
            r"(?i)",
            // GitHub PATs (classic + fine-grained + scoped variants).
            r"(?:ghp_|gho_|ghu_|ghs_|ghr_|github_pat_)[A-Za-z0-9_]{16,}",
            r"|",
            // GitLab PATs and runner tokens.
            r"(?:glpat-|glptt-|glrt-)[A-Za-z0-9_\-]{16,}",
            r"|",
            // git's credential-helper format.
            r"x-access-token:[A-Za-z0-9_\-\.]+",
            r"|",
            // Authorization header (bearer / basic / token).
            r"authorization:\s*(?:bearer|basic|token)\s+[A-Za-z0-9._\-+/=]+",
            r"|",
            // GitLab's own header name, used by `http.*.extraHeader`.
            r"private-token:\s*[A-Za-z0-9._\-+/=]+",
            r"|",
            // Anthropic API keys — the AI provider crates surface these in
            // config errors.
            r"sk-ant-[A-Za-z0-9._\-]{16,}",
            r"|",
            // Userinfo in a URL, with or without a password half. The
            // single-token form (`https://<token>@host`) is what `git
            // remote set-url` produces from a PAT.
            r"//[^/\s@]+@",
        ))
        .expect("redaction regex must compile")
    })
}

/// Replace credential-like substrings in `s` with `<redacted>`. Returns the
/// borrowed input unchanged when nothing matches.
pub fn redact_secrets(s: &str) -> Cow<'_, str> {
    secret_pattern().replace_all(s, "<redacted>")
}

/// `io::Write` wrapper that redacts known credential patterns before forwarding
/// bytes to the inner writer. Tracing's fmt layer writes each event as a single
/// `write` call, so chunk boundaries do not split tokens in practice.
struct RedactingWriter<W: io::Write> {
    inner: W,
}

impl<W: io::Write> io::Write for RedactingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let original_len = buf.len();
        let owned;
        let text: &str = match std::str::from_utf8(buf) {
            Ok(s) => s,
            Err(_) => {
                owned = String::from_utf8_lossy(buf).into_owned();
                &owned
            }
        };
        match redact_secrets(text) {
            Cow::Borrowed(_) => self.inner.write(buf),
            Cow::Owned(replaced) => {
                self.inner.write_all(replaced.as_bytes())?;
                Ok(original_len)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// `MakeWriter` adapter that wraps each created writer in a [`RedactingWriter`].
struct RedactingMakeWriter<M> {
    inner: M,
}

impl<'a, M> fmt::MakeWriter<'a> for RedactingMakeWriter<M>
where
    M: fmt::MakeWriter<'a>,
{
    type Writer = RedactingWriter<M::Writer>;
    fn make_writer(&'a self) -> Self::Writer {
        RedactingWriter {
            inner: self.inner.make_writer(),
        }
    }
}

/// Debug information for error reports and the "About" screen.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DebugInfo {
    /// Application version from Cargo metadata.
    pub app_version: String,
    /// Operating system and architecture (e.g. `"macos aarch64"`).
    pub os: String,
    /// CPU architecture (e.g. `"aarch64"`).
    pub arch: String,
    /// Output of `git --version`, if git is on PATH.
    pub git_version: Option<String>,
    /// Filesystem path to the log directory.
    pub log_path: String,
}

/// Get the platform-specific log directory.
///
/// Falls back to the OS temp directory (rather than the empty `PathBuf`)
/// when the user's home / config dir cannot be resolved, so logs land in a
/// well-known writable location instead of silently degrading to a relative
/// path under cwd.
pub fn log_directory() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("Library/Logs/BeardGit")
    }
    #[cfg(target_os = "linux")]
    {
        dirs::data_local_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join(".local/share")))
            .unwrap_or_else(std::env::temp_dir)
            .join("beardgit/logs")
    }
    #[cfg(target_os = "windows")]
    {
        dirs::config_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("BeardGit/logs")
    }
}

/// Delete log files older than `max_age_days` from `log_dir`.
///
/// Only removes files whose names contain `"log"`. This matches both the
/// current `beardgit.{date}.log` layout and any legacy `beardgit.log.{date}`
/// files left behind by pre-rename installs. Returns the number of files deleted.
///
/// # Errors
/// Returns an I/O error if the directory cannot be read.
pub fn purge_old_logs(log_dir: &std::path::Path, max_age_days: u64) -> std::io::Result<usize> {
    use std::time::{Duration, SystemTime};

    let cutoff = SystemTime::now() - Duration::from_secs(max_age_days * 86400);
    let mut deleted = 0usize;

    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Only consider files whose name contains "log"
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) if n.contains("log") => n.to_string(),
            _ => continue,
        };

        // Skip directories
        if !path.is_file() {
            continue;
        }

        // Check modification time
        let metadata = std::fs::metadata(&path)?;
        let modified = metadata.modified()?;

        if modified < cutoff {
            if let Err(e) = std::fs::remove_file(&path) {
                tracing::warn!(file = %name, error = %e, "Failed to remove old log file");
            } else {
                deleted += 1;
            }
        }
    }

    if deleted > 0 {
        tracing::info!(count = deleted, "Purged old log files");
    }

    Ok(deleted)
}

/// Build the daily-rotating file appender used by `init_logging`.
///
/// Filename layout: `beardgit.{YYYY-MM-DD}.log` — the `.log` suffix is last
/// so `*.log` globs and standard log viewers recognize the file.
fn build_file_appender(
    log_dir: &std::path::Path,
) -> tracing_appender::rolling::RollingFileAppender {
    tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("beardgit")
        .filename_suffix("log")
        .max_log_files(MAX_LOG_FILES)
        .build(log_dir)
        .expect("rolling file appender builder should not fail for a valid directory")
}

/// Initialize the global tracing subscriber with file logging at `level`.
///
/// Creates a daily-rotating log file in the platform log directory and
/// installs a reloadable file layer so [`set_level`] can change verbosity
/// later without a restart. The non-blocking writer guard is intentionally
/// leaked so it stays alive for the entire process lifetime.
///
/// `RUST_LOG`, when set, seeds the startup filter and wins over `level` —
/// the dev escape hatch for targeting one noisy crate. It does not affect
/// span events, and [`set_level`] ignores it entirely.
pub fn init_logging(level: &str) -> Result<(), String> {
    let log_dir = log_directory();
    std::fs::create_dir_all(&log_dir).map_err(|e| format!("failed to create log dir: {e}"))?;

    let file_appender = build_file_appender(&log_dir);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Keep the guard alive for the lifetime of the app.
    // We leak it intentionally — it is a singleton that lives until process exit.
    std::mem::forget(guard);

    let level = normalize_level(level).unwrap_or("info");
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(directives_for(level)));

    install(
        level,
        env_filter,
        Box::new(move || {
            BoxMakeWriter::new(RedactingMakeWriter {
                inner: non_blocking.clone(),
            })
        }),
    )?;

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        level,
        log_dir = %log_dir.display(),
        "BeardGit logging initialized"
    );

    Ok(())
}

/// Collect debug information about the running application.
///
/// Queries the system git binary for its version string and gathers
/// platform metadata for error reports.
pub fn collect_debug_info() -> DebugInfo {
    let git_version = std::process::Command::new("git")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    DebugInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        os: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        arch: std::env::consts::ARCH.to_string(),
        git_version,
        log_path: log_directory().to_string_lossy().into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::time::{Duration, SystemTime};

    // Test fixtures use the literal `EXAMPLE_FAKE` infix so secret-scanning
    // tools (GitHub, TruffleHog, …) don't pattern-match them as live tokens.

    #[test]
    fn normalize_level_accepts_the_three_offered_levels() {
        for level in LOG_LEVELS {
            assert_eq!(normalize_level(level), Some(*level));
        }
        // Case and surrounding whitespace are tolerated — the value round
        // trips through settings.json where hand-edits happen.
        assert_eq!(normalize_level(" DEBUG "), Some("debug"));
    }

    #[test]
    fn normalize_level_rejects_unknown_levels() {
        // `trace` and `warn` are real tracing levels but not offered by the
        // UI; rejecting them keeps settings.json and the selector in sync.
        for bad in ["", "trace", "warn", "verbose", "off"] {
            assert_eq!(
                normalize_level(bad),
                None,
                "expected {bad:?} to be rejected"
            );
        }
    }

    #[test]
    fn directives_for_error_silences_everything_below_error() {
        assert_eq!(directives_for("error"), "error");
    }

    #[test]
    fn directives_for_info_is_a_bare_info_filter() {
        // No per-crate promotions: `info` must be the plain narrative level,
        // otherwise "debug" has nothing left to add.
        assert_eq!(directives_for("info"), "info");
    }

    #[test]
    fn directives_for_debug_promotes_only_workspace_crates() {
        let directives = directives_for("debug");
        assert!(
            directives.starts_with("info,"),
            "third-party crates must stay at info, got {directives:?}"
        );
        // Hardcoded, not derived from WORKSPACE_TARGETS: asserting the
        // constant contains its own entries passes for any content. These
        // are the crates that actually emit events today, so dropping one
        // silently makes `debug` useless for the area it covers.
        for target in [
            "app_core",
            "git_engine",
            "storage",
            "watcher",
            "mutation_events",
            "task_runner",
        ] {
            assert!(
                directives.contains(&format!("{target}=debug")),
                "{target} missing from {directives:?}"
            );
        }
        // A bare global `debug` would drown our events in transport noise.
        assert!(!directives.split(',').any(|d| d == "debug"));
    }

    #[test]
    fn each_level_parses_and_raises_the_ceiling_it_claims() {
        use tracing::level_filters::LevelFilter;

        // Narrow on purpose: `EnvFilter` has no idea which crates exist, so
        // this would pass with a bogus allowlist. What it proves is that
        // each level's directive string parses and widens the ceiling as
        // advertised — an empty or malformed one would leave `debug` at
        // INFO. Whether the allowlist names *real* targets is covered by
        // `tests/log_level_reload.rs`, against a live subscriber.
        for (level, expected) in [
            ("error", LevelFilter::ERROR),
            ("info", LevelFilter::INFO),
            ("debug", LevelFilter::DEBUG),
        ] {
            let filter = EnvFilter::new(directives_for(level));
            assert_eq!(
                Layer::<Registry>::max_level_hint(&filter),
                Some(expected),
                "level {level:?} produced the wrong ceiling"
            );
        }
    }

    #[test]
    fn set_level_rejects_unknown_level_before_touching_the_subscriber() {
        // Validation happens before the handle lookup, so this is the error
        // even in a process where logging was never initialized.
        let err = set_level("shout").unwrap_err();
        assert!(err.contains("unknown log level"), "got {err:?}");
    }

    #[test]
    fn set_level_errors_when_logging_was_never_initialized() {
        // These unit tests never call `init_logging` (it installs a global
        // subscriber), so the handle is unset and a valid level still fails.
        let err = set_level("debug").unwrap_err();
        assert!(err.contains("not initialized"), "got {err:?}");
    }

    #[test]
    fn redact_strips_github_classic_pat() {
        let s = "auth: ghp_EXAMPLE_FAKE_PAT_VALUE_1234567890 done";
        let r = redact_secrets(s);
        assert!(r.contains("<redacted>"), "got {r}");
        assert!(!r.contains("ghp_"));
    }

    #[test]
    fn redact_strips_github_fine_grained_pat() {
        let s = "token=github_pat_EXAMPLE_FAKE_VALUE_1234567890ABC more";
        let r = redact_secrets(s);
        assert!(!r.contains("github_pat_"));
    }

    #[test]
    fn redact_strips_gitlab_pat() {
        let s = "PRIVATE-TOKEN: glpat-EXAMPLE_FAKE_VALUE_1234567890";
        let r = redact_secrets(s);
        assert!(!r.contains("glpat-"));
    }

    #[test]
    fn redact_strips_x_access_token_in_url() {
        let s =
            "https://x-access-token:ghp_EXAMPLE_FAKE_PAT_VALUE_1234567890@github.com/foo/bar.git";
        let r = redact_secrets(s);
        assert!(!r.contains("ghp_"));
        assert!(!r.contains("x-access-token:"));
    }

    #[test]
    fn redact_strips_basic_auth_in_url() {
        let s = "remote=https://alice:EXAMPLE_FAKE_PASSWORD@example.com/repo.git";
        let r = redact_secrets(s);
        assert!(!r.contains("alice:EXAMPLE_FAKE_PASSWORD"));
    }

    #[test]
    fn redact_strips_authorization_header() {
        let s = "headers: Authorization: Bearer EXAMPLE_FAKE_BEARER_VALUE_12345";
        let r = redact_secrets(s);
        assert!(r.contains("<redacted>"), "got {r}");
        assert!(!r.contains("EXAMPLE_FAKE_BEARER_VALUE"));
    }

    #[test]
    fn redact_strips_single_token_userinfo_in_url() {
        // `git remote set-url` with a PAT produces this shape — no colon,
        // so the password-style pattern alone missed it.
        let s = "remote=https://ghp_EXAMPLE_FAKE_PAT_VALUE_1234567890@github.com/foo/bar.git";
        let r = redact_secrets(s);
        assert!(!r.contains("ghp_"), "got {r}");
    }

    #[test]
    fn redact_strips_private_token_header() {
        let s = "http.extraHeader=PRIVATE-TOKEN: EXAMPLE_FAKE_GITLAB_HEADER_VALUE";
        let r = redact_secrets(s);
        assert!(!r.contains("EXAMPLE_FAKE_GITLAB_HEADER_VALUE"), "got {r}");
    }

    #[test]
    fn redact_strips_anthropic_api_key() {
        let s = "provider error: key sk-ant-EXAMPLE_FAKE_ANTHROPIC_VALUE_123 rejected";
        let r = redact_secrets(s);
        assert!(!r.contains("sk-ant-EXAMPLE"), "got {r}");
    }

    #[test]
    fn redact_leaves_credential_free_urls_alone() {
        // The userinfo pattern must not eat ordinary remote URLs, which are
        // allowed in the log and are often the useful part of the line.
        let s = "remote=https://github.com/foo/bar.git";
        let r = redact_secrets(s);
        assert_eq!(r.as_ref(), s);
    }

    #[test]
    fn redact_passes_through_clean_text() {
        let s = "INFO: branch updated to main; oid=4b825dc";
        let r = redact_secrets(s);
        assert_eq!(r.as_ref(), s);
        assert!(matches!(r, Cow::Borrowed(_)));
    }

    #[test]
    fn redacting_writer_replaces_secret_chunks() {
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut w = RedactingWriter { inner: &mut buf };
            w.write_all(b"line=ghp_EXAMPLE_FAKE_PAT_VALUE_1234567890 ok\n")
                .unwrap();
            w.write_all(b"plain line, nothing to redact\n").unwrap();
        }
        let s = String::from_utf8(buf).unwrap();
        assert!(!s.contains("ghp_"));
        assert!(s.contains("<redacted>"));
        assert!(s.contains("plain line, nothing to redact"));
    }

    /// Helper: create a log file with a modified time set to `days_ago` days in the past.
    fn create_aged_log(dir: &std::path::Path, name: &str, days_ago: u64) {
        let path = dir.join(name);
        fs::write(&path, "log content").unwrap();
        let age = SystemTime::now() - Duration::from_secs(days_ago * 86400);
        filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(age)).unwrap();
    }

    #[test]
    fn purge_deletes_old_logs() {
        let tmp = tempfile::tempdir().unwrap();
        create_aged_log(tmp.path(), "beardgit.2026-04-01.log", 10);
        create_aged_log(tmp.path(), "beardgit.2026-04-10.log", 3);

        let deleted = purge_old_logs(tmp.path(), 7).unwrap();
        assert_eq!(deleted, 1);
        assert!(!tmp.path().join("beardgit.2026-04-01.log").exists());
        assert!(tmp.path().join("beardgit.2026-04-10.log").exists());
    }

    #[test]
    fn purge_ignores_non_log_files() {
        let tmp = tempfile::tempdir().unwrap();
        create_aged_log(tmp.path(), "beardgit.2026-04-01.log", 10);
        create_aged_log(tmp.path(), "settings.json", 10);

        let deleted = purge_old_logs(tmp.path(), 7).unwrap();
        assert_eq!(deleted, 1);
        assert!(tmp.path().join("settings.json").exists());
    }

    #[test]
    fn purge_returns_zero_on_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let deleted = purge_old_logs(tmp.path(), 7).unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    fn purge_handles_nonexistent_dir() {
        let result = purge_old_logs(std::path::Path::new("/nonexistent/path"), 7);
        assert!(result.is_err());
    }

    #[test]
    fn purge_keeps_all_when_none_old_enough() {
        let tmp = tempfile::tempdir().unwrap();
        create_aged_log(tmp.path(), "beardgit.2026-04-14.log", 2);
        create_aged_log(tmp.path(), "beardgit.2026-04-15.log", 1);

        let deleted = purge_old_logs(tmp.path(), 7).unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    fn purge_matches_new_filename_pattern() {
        let tmp = tempfile::tempdir().unwrap();
        // New shape — old enough to purge.
        create_aged_log(tmp.path(), "beardgit.2026-04-01.log", 10);
        // New shape — recent, should survive.
        create_aged_log(tmp.path(), "beardgit.2026-04-20.log", 1);

        let deleted = purge_old_logs(tmp.path(), 7).unwrap();
        assert_eq!(deleted, 1);
        assert!(!tmp.path().join("beardgit.2026-04-01.log").exists());
        assert!(tmp.path().join("beardgit.2026-04-20.log").exists());
    }

    #[test]
    fn purge_handles_legacy_filenames_without_crashing() {
        // Legacy `beardgit.log.{date}` files may linger from pre-rename installs.
        // Rotation should treat them like any other log file: age-based purge, no panic.
        let tmp = tempfile::tempdir().unwrap();
        create_aged_log(tmp.path(), "beardgit.log.2026-04-01", 10); // legacy, old
        create_aged_log(tmp.path(), "beardgit.2026-04-20.log", 1); // new, recent

        let deleted = purge_old_logs(tmp.path(), 7).unwrap();
        assert_eq!(deleted, 1, "legacy old file should be purged by age");
        assert!(!tmp.path().join("beardgit.log.2026-04-01").exists());
        assert!(tmp.path().join("beardgit.2026-04-20.log").exists());
    }

    #[test]
    fn init_logging_produces_filename_matching_new_pattern() {
        // The rolling appender writes `beardgit.{YYYY-MM-DD}.log`.
        // We build the appender via the production helper to assert
        // the filename shape without touching the global subscriber.
        let tmp = tempfile::tempdir().unwrap();
        let appender = build_file_appender(tmp.path());

        // Force a write so the file is created.
        use std::io::Write;
        let mut w = appender;
        writeln!(w, "probe").unwrap();
        drop(w);

        let entries: Vec<String> = std::fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(|e| e.ok().and_then(|e| e.file_name().into_string().ok()))
            .collect();

        assert_eq!(
            entries.len(),
            1,
            "expected exactly one log file, got {entries:?}"
        );
        let name = &entries[0];
        assert!(
            name.starts_with("beardgit.") && name.ends_with(".log"),
            "filename {name:?} does not match beardgit.{{date}}.log"
        );
        // Reject the legacy shape: prefix `beardgit.log.` means the `.log`
        // slot is in the middle, which is exactly what we are fixing.
        assert!(
            !name.starts_with("beardgit.log."),
            "filename {name:?} still uses the legacy beardgit.log.{{date}} shape"
        );
    }
}

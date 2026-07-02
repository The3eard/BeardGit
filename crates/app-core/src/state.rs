//! Shared application state managed by Tauri for the lifetime of the process.
//!
//! [`AppState`] is registered as a Tauri-managed singleton and injected into
//! every command handler via `State<'_, AppState>`. All mutable fields are
//! wrapped in `Mutex` so they can be accessed safely from multiple async tasks.

use std::path::PathBuf;
use std::sync::Mutex;

use ai_provider::AvailableAiProvider;
use auth::CredentialStore;
use git_engine::Repository;
use graph_builder::GraphLayout;
use provider::{ProviderKind, ProviderUser};
use requests_store::RequestsDatabase;
use std::sync::Arc;
use storage::config::AppConfig;
use storage::database::Database;
use watcher::RepoWatcher;

use crate::ai_background::AiBackgroundCoordinator;
use crate::commands::GraphLayoutOptions;

/// A cached `Arc<dyn ForgeProvider>` tagged with the state it was built for.
pub struct ForgeProviderCacheEntry {
    /// Active provider index at construction time.
    pub provider_index: usize,
    /// Active project path at construction time.
    pub project_path: PathBuf,
    /// The erased trait object, ready for reuse.
    pub provider: std::sync::Arc<dyn forge_provider::ForgeProvider>,
}

/// A single authenticated provider connection held in application state.
///
/// Stores the provider metadata and the detected project for the current repo.
/// The actual HTTP client (`Box<dyn CiProvider>`) is recreated on demand from
/// the credential store because it is not `Clone`.
pub struct ProviderConnection {
    /// Which provider type this is.
    pub kind: ProviderKind,
    /// Base URL of the provider instance (e.g. `"https://gitlab.com"`).
    pub instance_url: String,
    /// The authenticated user's profile (captured at connect time).
    pub user: ProviderUser,
    /// Project reference detected from the current repo's remote URL
    /// (URL-encoded path for GitLab, `"owner/repo"` for GitHub).
    /// `None` if no repo is open or the remote doesn't match this provider.
    pub project_ref: Option<String>,
    /// Human-readable project name from the provider API.
    /// `None` if no project detected.
    pub project_name: Option<String>,
}

/// A single project slot in the multi-project tab bar.
///
/// Heavy fields (`repo`, `layout`, `watcher`) are only populated for the
/// active project. Background tabs keep lightweight metadata only.
pub struct ProjectSlot {
    /// Absolute filesystem path to the repository root.
    pub path: String,
    /// Repository name (last path segment).
    pub name: String,
    /// The git repository handle. `None` if not loaded (lazy/background tab).
    pub repo: Option<Repository>,
    /// Pre-computed graph layout. `None` if not loaded.
    pub layout: Option<GraphLayout>,
    /// The options `layout` was computed with. Viewport commands compare this
    /// against the frontend-requested options and rebuild on mismatch, so the
    /// slot always carries the layout for the mode currently on screen.
    pub layout_options: GraphLayoutOptions,
    /// Filesystem watcher. `None` if not loaded.
    pub watcher: Option<RepoWatcher>,
    /// Current HEAD branch name. Always populated (cheap to read).
    pub head_branch: Option<String>,
    /// Number of uncommitted changes. Always populated (cheap to read).
    pub change_count: usize,
    /// `true` when this slot points at a linked git worktree rather
    /// than the main working directory. Computed once at open time —
    /// the value never changes for the life of the slot since we don't
    /// re-home tabs across worktrees.
    pub is_worktree: bool,
}

/// Shared global state for the BeardGit Tauri application.
///
/// Holds the list of open project slots, the currently active slot index,
/// the SQLite database, user configuration, credential store, and all
/// authenticated provider connections.
pub struct AppState {
    /// All open project slots (one per tab).
    pub projects: Mutex<Vec<ProjectSlot>>,
    /// Index into `projects` for the currently active project.
    /// `None` if no project has been opened yet.
    pub active_index: Mutex<Option<usize>>,
    /// SQLite database for persistent application data.
    pub db: Mutex<Database>,
    /// SQLite database for the Requests panel (global collections / items / env / history).
    pub requests_db: Mutex<RequestsDatabase>,
    /// User-facing application configuration (loaded from `settings.json`).
    pub config: Mutex<AppConfig>,
    /// Filesystem path to `settings.json`, used when saving config changes.
    pub config_path: PathBuf,
    /// Root config directory for BeardGit (parent of `settings.json`). Used
    /// by persistent caches (layouts, project snapshots) that need to write
    /// under `<config_dir>/…`.
    pub config_dir: PathBuf,
    /// Encrypted credential store for PAT / OAuth tokens.
    pub credential_store: CredentialStore,
    /// All authenticated provider connections.
    pub providers: Mutex<Vec<ProviderConnection>>,
    /// Index into `providers` for the currently active provider
    /// (determined by matching the repo's remote URL).
    /// `None` if no repo is open or no provider matches.
    pub active_provider_index: Mutex<Option<usize>>,
    /// Cache of resolved CLI binary paths per provider kind.
    ///
    /// Populated lazily on first `resolve_cli_binary` call and invalidated
    /// when the user connects or reconnects a provider. Avoids walking
    /// `$PATH` on every forge IPC handler.
    pub cli_binary_cache: Mutex<std::collections::HashMap<ProviderKind, std::path::PathBuf>>,
    /// Cache of the current forge provider instance, keyed by
    /// `(active_provider_index, active_project_path)`. Invalidated on
    /// provider change, project switch, or reconnect.
    pub forge_provider_cache: Mutex<Option<ForgeProviderCacheEntry>>,
    /// Detected AI providers (populated at startup and on refresh).
    pub ai_providers: Mutex<Vec<AvailableAiProvider>>,
    /// Filesystem watcher for AI session directories. Started once after the
    /// first successful provider detection and kept alive for the process
    /// lifetime. `None` if no provider has been detected yet.
    pub ai_session_watcher: Mutex<Option<watcher::AiSessionWatcher>>,
    /// Filesystem watcher for AI config directories (`.claude/` in project
    /// and home). `None` if no watcher has been started.
    pub ai_config_watcher: Mutex<Option<watcher::AiConfigWatcher>>,
    /// Shared coordinator for AI background runs. Populated in
    /// `src-tauri/src/lib.rs` during `.setup()` because it needs an
    /// `Arc<TaskManager>` and an `AppHandle` for the event sink.
    pub ai_background_coordinator: Mutex<Option<Arc<AiBackgroundCoordinator>>>,
    /// Active in-flight Requests-panel runs, keyed by frontend-generated
    /// ticket id. Each entry's [`tokio_util::sync::CancellationToken`] is
    /// fired by `requests_cancel` to abort the matching `requests_run`
    /// before its `reqwest` future resolves. Entries are inserted by
    /// `requests_run` *before* the first await and removed via a Drop
    /// guard on every exit path (success, error, cancel, panic).
    pub requests_cancellations:
        Mutex<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>,
    /// Per-repo snapshot of `references()` name→OID taken when the slot's
    /// cached [`GraphLayout`] was last (re)built, keyed by repo path. Lets
    /// `refresh_graph_layout` detect a "simple advance" (exactly one branch
    /// moved forward, nothing else changed) and patch the layout incrementally
    /// instead of re-walking the whole graph. Entries are best-effort hints:
    /// a miss or a stale entry just falls back to a full rebuild.
    pub layout_ref_snapshots: Mutex<std::collections::BTreeMap<String, RefSnapshot>>,
    /// Cache of `(branch_tip_oid, upstream_tip_oid) → (ahead, behind)`, shared
    /// across all open repos. Keyed on the tip OIDs, so it is self-invalidating:
    /// when either tip moves the key changes and the entry is recomputed. Lets
    /// `branches()` skip the O(divergence) `graph_ahead_behind` walk for every
    /// tracking branch whose tips are unchanged since the last call.
    pub ahead_behind_cache: Mutex<std::collections::HashMap<(String, String), (usize, usize)>>,
}

/// A repo's `references()` name→OID picture (symbolic refs like HEAD excluded),
/// used by the incremental graph-refresh fast path. See
/// [`AppState::layout_ref_snapshots`].
pub type RefSnapshot = std::collections::BTreeMap<String, String>;

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Create a new `AppState`, opening the database and loading config from
    /// the platform config directory (`~/.config/beardgit/` on Linux/macOS).
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("beardgit");
        let db_path = config_dir.join("data.db");
        let config_path = config_dir.join("settings.json");

        let db = Database::open(&db_path).expect("Failed to open database");
        let requests_db_path = config_dir.join("requests.db");
        let requests_db =
            RequestsDatabase::open(&requests_db_path).expect("Failed to open requests database");
        let config = AppConfig::load(&config_path).unwrap_or_default();
        let credential_store =
            CredentialStore::new(&config_dir).expect("Failed to initialize credential store");

        Self {
            projects: Mutex::new(Vec::new()),
            active_index: Mutex::new(None),
            db: Mutex::new(db),
            requests_db: Mutex::new(requests_db),
            config: Mutex::new(config),
            config_path,
            config_dir,
            credential_store,
            providers: Mutex::new(Vec::new()),
            active_provider_index: Mutex::new(None),
            cli_binary_cache: Mutex::new(std::collections::HashMap::new()),
            forge_provider_cache: Mutex::new(None),
            ai_providers: Mutex::new(Vec::new()),
            ai_session_watcher: Mutex::new(None),
            ai_config_watcher: Mutex::new(None),
            ai_background_coordinator: Mutex::new(None),
            requests_cancellations: Mutex::new(std::collections::HashMap::new()),
            layout_ref_snapshots: Mutex::new(std::collections::BTreeMap::new()),
            ahead_behind_cache: Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Record the refs a slot's layout was built from (see
    /// [`Self::layout_ref_snapshots`]). Best-effort — a poisoned lock is
    /// ignored, degrading only to a full-rebuild fallback next refresh.
    pub fn store_layout_ref_snapshot(&self, path: &str, snap: RefSnapshot) {
        if let Ok(mut map) = self.layout_ref_snapshots.lock() {
            map.insert(path.to_string(), snap);
        }
    }

    /// Fetch the refs snapshot recorded for `path`, if any.
    pub fn layout_ref_snapshot(&self, path: &str) -> Option<RefSnapshot> {
        self.layout_ref_snapshots
            .lock()
            .ok()
            .and_then(|map| map.get(path).cloned())
    }
}

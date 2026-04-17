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
use storage::config::AppConfig;
use storage::database::Database;
use watcher::RepoWatcher;

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
    /// Filesystem watcher. `None` if not loaded.
    pub watcher: Option<RepoWatcher>,
    /// Current HEAD branch name. Always populated (cheap to read).
    pub head_branch: Option<String>,
    /// Number of uncommitted changes. Always populated (cheap to read).
    pub change_count: usize,
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
    /// User-facing application configuration (loaded from `settings.json`).
    pub config: Mutex<AppConfig>,
    /// Filesystem path to `settings.json`, used when saving config changes.
    pub config_path: PathBuf,
    /// Encrypted credential store for PAT / OAuth tokens.
    pub credential_store: CredentialStore,
    /// All authenticated provider connections.
    pub providers: Mutex<Vec<ProviderConnection>>,
    /// Index into `providers` for the currently active provider
    /// (determined by matching the repo's remote URL).
    /// `None` if no repo is open or no provider matches.
    pub active_provider_index: Mutex<Option<usize>>,
    /// Detected AI providers (populated at startup and on refresh).
    pub ai_providers: Mutex<Vec<AvailableAiProvider>>,
    /// Filesystem watcher for AI session directories. Started once after the
    /// first successful provider detection and kept alive for the process
    /// lifetime. `None` if no provider has been detected yet.
    pub ai_session_watcher: Mutex<Option<watcher::AiSessionWatcher>>,
    /// Filesystem watcher for AI config directories (`.claude/` in project
    /// and home). `None` if no watcher has been started.
    pub ai_config_watcher: Mutex<Option<watcher::AiConfigWatcher>>,
}

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
        let config = AppConfig::load(&config_path).unwrap_or_default();
        let credential_store =
            CredentialStore::new(&config_dir).expect("Failed to initialize credential store");

        Self {
            projects: Mutex::new(Vec::new()),
            active_index: Mutex::new(None),
            db: Mutex::new(db),
            config: Mutex::new(config),
            config_path,
            credential_store,
            providers: Mutex::new(Vec::new()),
            active_provider_index: Mutex::new(None),
            ai_providers: Mutex::new(Vec::new()),
            ai_session_watcher: Mutex::new(None),
            ai_config_watcher: Mutex::new(None),
        }
    }
}

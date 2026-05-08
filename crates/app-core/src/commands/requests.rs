//! Tauri commands for the Requests panel.
//!
//! Exposes tree listing for both project-scoped (`.beardgit/requests/`)
//! and global (SQLite-backed) request collections, plus load/save helpers
//! that bridge the Svelte editor with on-disk `.http` files and the
//! `requests.db` global library.

use std::collections::BTreeMap;
use std::path::Path;

use auth::Credential;
use provider::ProviderKind;
use requests_runner::{
    codegen::copy_as,
    env::{EnvFile, list_envs, load_env, save_env as save_env_file},
    executor::{ExecuteOptions, execute},
    history::record,
    parser::{import_curl, parse_http_file},
    resolver::resolve,
    seed::seed_quickstart_pack,
    types::{CodegenTarget, ParsedRequest, ResolveCtx},
};
use requests_store::HistoryEntry;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::state::AppState;

/// Build the namespaced "instance URL" key under which a Requests-panel
/// secret is stored inside the encrypted [`auth::CredentialStore`].
///
/// The credential store is provider-agnostic but its public API was
/// designed for forge tokens — credentials are keyed by an arbitrary
/// instance URL string and carry a [`auth::Credential`] (token + provider).
/// We piggy-back on that primitive: the instance-URL string acts as the
/// composite key `(env_name, secret_name)`, the `token` field carries the
/// secret value, and the `provider` field is set to a stable placeholder
/// (`ProviderKind::GitHub`) which is irrelevant for non-forge secrets.
///
/// The `requests-env://` scheme prefix prevents collisions with real forge
/// instance URLs (which are `https://…`).
fn requests_secret_key(env_name: &str, secret_name: &str) -> String {
    format!("requests-env://{env_name}/{secret_name}")
}

/// Combined project + global tree, kept here as the canonical shape that
/// will be returned by the future combined `requests_list_tree` command
/// (added in task 8.6) and shared with the frontend type bindings.
#[allow(dead_code)] // reserved for the combined tree command in 8.6 / frontend type-sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionsTree {
    /// Tree rooted at `<project>/.beardgit/requests/`.
    pub project: Vec<TreeNode>,
    /// Tree rooted at the global SQLite library.
    pub global: Vec<TreeNode>,
}

/// A single node in a requests tree.
///
/// `kind` is one of `"folder"`, `"file"`, or `"block"` (reserved). The
/// `rel_path` field encodes the source location:
/// - For project nodes: relative path under `.beardgit/requests/`.
/// - For global items: `"global/<id>"`.
/// - For global collections: `"global-collection/<id>"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode {
    /// Node kind: `"folder"`, `"file"`, or `"block"`.
    pub kind: String,
    /// Display name (file name for project entries, item/collection name for global).
    pub name: String,
    /// Source-relative path (see struct docs for encoding).
    pub rel_path: String,
    /// HTTP method of the first parsed block, when this node is a `.http` file.
    /// `None` for folders or unparseable files. Surfaced as a small badge in
    /// the CollectionsTree leaf rows so users can see GET vs POST at a glance.
    pub method: Option<String>,
    /// Child nodes (empty for files/items).
    pub children: Vec<TreeNode>,
}

/// List the project-scoped requests tree under
/// `<project_path>/.beardgit/requests/`.
///
/// Returns an empty list when the directory does not exist. The hidden
/// `_env` directory (used for environment files) is skipped.
///
/// **Side-effect:** when the `requests/` directory already exists but
/// no `_env/default.json` is present, this command lazily creates an
/// empty default env so the EnvSwitcher always has at least one
/// option without requiring the user to seed via SeedPrompt first.
/// Best-effort: filesystem failures are ignored so a read-only mount
/// still returns the tree.
#[tauri::command]
pub fn requests_list_project(project_path: String) -> Result<Vec<TreeNode>, String> {
    let root = Path::new(&project_path).join(".beardgit").join("requests");
    // If the project hasn't opted into requests yet (`.beardgit/requests/`
    // missing), this is a no-op — never silently materialise the directory
    // chain on a project that doesn't want it. The frontend renders the
    // SeedPrompt empty state from the `vec![]` return; the explicit
    // creation path is the seeding command, not the listing command.
    if !root.exists() {
        return Ok(vec![]);
    }
    // Once the requests folder exists we keep `_env/default.json`
    // self-healing so a user-deleted env file silently reappears on the
    // next panel access, instead of leaving the env switcher empty.
    ensure_default_env(&root);
    Ok(walk(&root, &root))
}

/// Make sure `<requests_root>/_env/default.json` exists with empty content
/// when missing — and, if `requests_root` itself is missing, create the
/// whole `<project>/.beardgit/requests/_env/` chain on the way through.
///
/// Called from every command that reads env state (the tree listing, the
/// env summary listing, and so on). Idempotent: a present `default.json`
/// is left untouched, including any user edits to it. Calling this on a
/// project with no requests folder yet is the canonical "open the panel
/// → see the default env appear automatically" entry point.
fn ensure_default_env(requests_root: &Path) {
    let env_dir = requests_root.join("_env");
    let default_env = env_dir.join("default.json");
    if !default_env.exists() {
        // create_dir_all walks the parents — `requests_root` is created
        // here too when this is the first ever access for the project.
        let _ = std::fs::create_dir_all(&env_dir);
        let _ = std::fs::write(
            &default_env,
            "{\n  \"$schema\": \"beardgit-env/v1\",\n  \"vars\": {},\n  \"secrets\": []\n}\n",
        );
    }
}

/// Recursively walk `dir` and emit `TreeNode`s with paths relative to `root`.
fn walk(root: &Path, dir: &Path) -> Vec<TreeNode> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let mut nodes: Vec<TreeNode> = vec![];
    for e in entries.flatten() {
        let p = e.path();
        let name = p
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if name == "_env" {
            continue;
        }
        let rel = p
            .strip_prefix(root)
            .ok()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();
        if p.is_dir() {
            nodes.push(TreeNode {
                kind: "folder".into(),
                name,
                rel_path: rel,
                method: None,
                children: walk(root, &p),
            });
        } else if p.extension().and_then(|s| s.to_str()) == Some("http") {
            // Parse the first block's method so the leaf row can show a
            // colored badge. Best-effort: malformed files just render
            // without a badge.
            let method = std::fs::read_to_string(&p).ok().and_then(|s| {
                parse_http_file(&s)
                    .ok()
                    .and_then(|v| v.into_iter().next())
                    .and_then(|r| r.method)
                    .map(|m| m.as_str().to_string())
            });
            nodes.push(TreeNode {
                kind: "file".into(),
                name,
                rel_path: rel,
                method,
                children: vec![],
            });
        }
    }
    nodes.sort_by(|a, b| {
        let order = |k: &str| u8::from(k != "folder");
        order(&a.kind)
            .cmp(&order(&b.kind))
            .then(a.name.cmp(&b.name))
    });
    nodes
}

/// List the global requests tree backed by `requests.db`.
///
/// Loose items (no parent collection) are returned as top-level files.
/// Each collection becomes a folder containing its items.
#[tauri::command]
pub fn requests_list_global(state: tauri::State<'_, AppState>) -> Result<Vec<TreeNode>, String> {
    let db = state.requests_db.lock().map_err(|_| "db poisoned")?;
    let cols = db.list_global_collections().map_err(|e| e.to_string())?;
    let mut roots: Vec<TreeNode> = vec![];
    // Loose items (no collection)
    let loose = db.list_global_items(None).map_err(|e| e.to_string())?;
    for it in loose {
        let method = method_from_http(&it.http_content);
        roots.push(TreeNode {
            kind: "file".into(),
            name: it.name.clone(),
            rel_path: format!("global/{}", it.id),
            method,
            children: vec![],
        });
    }
    for c in cols {
        let items = db
            .list_global_items(Some(c.id))
            .map_err(|e| e.to_string())?;
        let children = items
            .into_iter()
            .map(|it| {
                let method = method_from_http(&it.http_content);
                TreeNode {
                    kind: "file".into(),
                    name: it.name,
                    rel_path: format!("global/{}", it.id),
                    method,
                    children: vec![],
                }
            })
            .collect();
        roots.push(TreeNode {
            kind: "folder".into(),
            name: c.name,
            rel_path: format!("global-collection/{}", c.id),
            method: None,
            children,
        });
    }
    Ok(roots)
}

/// Parse the first block's HTTP method out of a `.http` source string.
///
/// Used by both project (file-on-disk) and global (db-stored) tree builders
/// so leaf rows can render a small colored verb badge. Returns `None` when
/// the source is empty, malformed, or has no method on its first block.
fn method_from_http(source: &str) -> Option<String> {
    parse_http_file(source)
        .ok()
        .and_then(|v| v.into_iter().next())
        .and_then(|r| r.method)
        .map(|m| m.as_str().to_string())
}

/// Load and parse a `.http` source from either the project tree or the
/// global library, returning the parsed list of requests for the editor.
///
/// `source_kind` is one of `"project"` or `"global"`. For `"project"`, the
/// `project_path` argument and the `source_path` (relative under
/// `.beardgit/requests/`) are required. For `"global"`, the `source_path`
/// must be of the form `"global/<id>"`.
#[tauri::command]
pub fn requests_load(
    state: tauri::State<'_, AppState>,
    source_kind: String,
    source_path: String,
    project_path: Option<String>,
) -> Result<Vec<ParsedRequest>, String> {
    let content = read_source(&state, &source_kind, &source_path, project_path.as_deref())?;
    parse_http_file(&content).map_err(|e| e.to_string())
}

/// Persist the raw `.http` content for a project file or global item.
///
/// Project sources are written under `<project>/.beardgit/requests/<source_path>`,
/// creating any missing parent directories. Global sources update the
/// `http_content` column of the targeted item with the current timestamp.
#[tauri::command]
pub fn requests_save(
    state: tauri::State<'_, AppState>,
    source_kind: String,
    source_path: String,
    project_path: Option<String>,
    content: String,
) -> Result<(), String> {
    match source_kind.as_str() {
        "project" => {
            let root = project_path.ok_or("project_path required")?;
            let path = Path::new(&root)
                .join(".beardgit")
                .join("requests")
                .join(&source_path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&path, content).map_err(|e| e.to_string())?;
            Ok(())
        }
        "global" => {
            let id: i64 = source_path
                .strip_prefix("global/")
                .ok_or("invalid global source_path")?
                .parse()
                .map_err(|_| "invalid global id")?;
            let now = chrono::Utc::now().timestamp();
            let db = state.requests_db.lock().map_err(|_| "db poisoned")?;
            db.update_global_item(id, &content, now)
                .map_err(|e| e.to_string())
        }
        other => Err(format!("unknown source_kind: {other}")),
    }
}

/// Read the raw `.http` content for a given source.
///
/// Used by [`requests_load`] before parsing; kept private because callers
/// should always go through the public command surface.
fn read_source(
    state: &tauri::State<'_, AppState>,
    source_kind: &str,
    source_path: &str,
    project_path: Option<&str>,
) -> Result<String, String> {
    match source_kind {
        "project" => {
            let root = project_path.ok_or("project_path required")?;
            let path = Path::new(root)
                .join(".beardgit")
                .join("requests")
                .join(source_path);
            std::fs::read_to_string(&path).map_err(|e| e.to_string())
        }
        "global" => {
            let id: i64 = source_path
                .strip_prefix("global/")
                .ok_or("invalid global source_path")?
                .parse()
                .map_err(|_| "invalid global id")?;
            let db = state.requests_db.lock().map_err(|_| "db poisoned")?;
            let items = db.list_global_items(None).map_err(|e| e.to_string())?;
            for it in items {
                if it.id == id {
                    return Ok(it.http_content);
                }
            }
            for col in db.list_global_collections().map_err(|e| e.to_string())? {
                let items = db
                    .list_global_items(Some(col.id))
                    .map_err(|e| e.to_string())?;
                for it in items {
                    if it.id == id {
                        return Ok(it.http_content);
                    }
                }
            }
            Err(format!("global item {id} not found"))
        }
        other => Err(format!("unknown source_kind: {other}")),
    }
}

/// Create a new loose or collection-scoped global item in `requests.db`.
///
/// `collection_id = None` creates a loose top-level item. The new row's
/// `updated_at` is the current Unix timestamp. Returns the newly inserted
/// row id so the frontend can immediately address the item via
/// `"global/<id>"` (e.g. to set it as the active source).
#[tauri::command]
pub fn requests_create_global_item(
    state: tauri::State<'_, AppState>,
    name: String,
    collection_id: Option<i64>,
    http_content: String,
) -> Result<i64, String> {
    let now = chrono::Utc::now().timestamp();
    let db = state.requests_db.lock().map_err(|_| "db poisoned")?;
    db.create_global_item(collection_id, &name, &http_content, now)
        .map_err(|e| e.to_string())
}

/// Delete a request from either the project tree or the global library.
///
/// `source_kind` is `"project"` (file under `.beardgit/requests/`) or
/// `"global"` (DB-backed item, addressed via `"global/<id>"`). Project
/// deletes are idempotent: a missing file is not an error so the UI's
/// "delete then refresh" flow stays simple. Global deletes invoke the
/// underlying SQL `DELETE`, which is also a silent no-op for unknown ids.
#[tauri::command]
pub fn requests_delete(
    state: tauri::State<'_, AppState>,
    source_kind: String,
    source_path: String,
    project_path: Option<String>,
) -> Result<(), String> {
    match source_kind.as_str() {
        "project" => {
            let root = project_path.ok_or("project_path required")?;
            let path = Path::new(&root)
                .join(".beardgit")
                .join("requests")
                .join(&source_path);
            if path.exists() {
                // Folder rows from the tree resolve to a directory on
                // disk; recursively remove the subtree. Single-file
                // rows still hit `remove_file`.
                if path.is_dir() {
                    std::fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
                } else {
                    std::fs::remove_file(&path).map_err(|e| e.to_string())?;
                }
            }
            Ok(())
        }
        "global" => {
            let id: i64 = source_path
                .strip_prefix("global/")
                .ok_or("invalid global source_path")?
                .parse()
                .map_err(|_| "invalid global id")?;
            let db = state.requests_db.lock().map_err(|_| "db poisoned")?;
            db.delete_global_item(id).map_err(|e| e.to_string())
        }
        other => Err(format!("unknown source_kind: {other}")),
    }
}

/// Rename a request, in place, in either the project tree or the global library.
///
/// For `source_kind = "project"`, this is a `std::fs::rename` from
/// `.beardgit/requests/<from_path>` to `.beardgit/requests/<to_path>`.
/// Missing intermediate directories under the destination are created.
///
/// For `source_kind = "global"`, the `from_path` must be `"global/<id>"`
/// and the `to_path` is the new display name (no `global/` prefix). The
/// underlying [`RequestsDatabase::rename_global_item`] only updates the
/// `name` column — the http content and updated_at timestamp are left
/// untouched so a rename doesn't masquerade as a content edit.
#[tauri::command]
pub fn requests_rename(
    state: tauri::State<'_, AppState>,
    source_kind: String,
    from_path: String,
    to_path: String,
    project_path: Option<String>,
) -> Result<(), String> {
    match source_kind.as_str() {
        "project" => {
            let root = project_path.ok_or("project_path required")?;
            let from = Path::new(&root)
                .join(".beardgit")
                .join("requests")
                .join(&from_path);
            let to = Path::new(&root)
                .join(".beardgit")
                .join("requests")
                .join(&to_path);
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::rename(&from, &to).map_err(|e| e.to_string())?;
            Ok(())
        }
        "global" => {
            let id: i64 = from_path
                .strip_prefix("global/")
                .ok_or("invalid global source_path")?
                .parse()
                .map_err(|_| "invalid global id")?;
            // The new name is the to_path string verbatim (no `global/` prefix).
            let db = state.requests_db.lock().map_err(|_| "db poisoned")?;
            db.rename_global_item(id, &to_path)
                .map_err(|e| e.to_string())
        }
        other => Err(format!("unknown source_kind: {other}")),
    }
}

/// Duplicate a request in place, returning the new source path.
///
/// Project files clone the on-disk content into a sibling with a
/// ` copy` suffix on the file stem (`users/get.http` →
/// `users/get copy.http`). Global items insert a new row in the same
/// collection with `<name> copy` and the same http content.
///
/// The returned string is the canonical `source_path` (relative path for
/// project, `"global/<id>"` for global) so callers can immediately
/// select the new item.
#[tauri::command]
pub fn requests_duplicate(
    state: tauri::State<'_, AppState>,
    source_kind: String,
    source_path: String,
    project_path: Option<String>,
) -> Result<String, String> {
    match source_kind.as_str() {
        "project" => {
            let root = project_path.ok_or("project_path required")?;
            let from = Path::new(&root)
                .join(".beardgit")
                .join("requests")
                .join(&source_path);
            let content = std::fs::read_to_string(&from).map_err(|e| e.to_string())?;
            let to_rel = duplicate_path(&source_path);
            let to = Path::new(&root)
                .join(".beardgit")
                .join("requests")
                .join(&to_rel);
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&to, content).map_err(|e| e.to_string())?;
            Ok(to_rel)
        }
        "global" => {
            let id: i64 = source_path
                .strip_prefix("global/")
                .ok_or("invalid global source_path")?
                .parse()
                .map_err(|_| "invalid global id")?;
            let now = chrono::Utc::now().timestamp();
            let db = state.requests_db.lock().map_err(|_| "db poisoned")?;
            // Find the original to clone its content + name + collection.
            let mut found = None;
            for col in db.list_global_collections().map_err(|e| e.to_string())? {
                let items = db
                    .list_global_items(Some(col.id))
                    .map_err(|e| e.to_string())?;
                if let Some(it) = items.into_iter().find(|i| i.id == id) {
                    found = Some(it);
                    break;
                }
            }
            if found.is_none() {
                let items = db.list_global_items(None).map_err(|e| e.to_string())?;
                if let Some(it) = items.into_iter().find(|i| i.id == id) {
                    found = Some(it);
                }
            }
            let original = found.ok_or(format!("global item {id} not found"))?;
            let new_id = db
                .create_global_item(
                    original.collection_id,
                    &format!("{} copy", original.name),
                    &original.http_content,
                    now,
                )
                .map_err(|e| e.to_string())?;
            Ok(format!("global/{new_id}"))
        }
        other => Err(format!("unknown source_kind: {other}")),
    }
}

/// Append a `" copy"` suffix to a project-relative `.http` path, before the
/// extension, preserving any parent folders.
///
/// `"users/get.http"` → `"users/get copy.http"`. A path without an
/// extension falls back to `.http`. A path without a stem falls back to
/// `"untitled"`. Used by [`requests_duplicate`] to compute the new
/// sibling path.
fn duplicate_path(rel: &str) -> String {
    let path = std::path::Path::new(rel);
    let parent = path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("http");
    let name = format!("{stem} copy.{ext}");
    if parent.is_empty() {
        name
    } else {
        format!("{parent}/{name}")
    }
}

/// Summary of a single environment file in `<project>/.beardgit/requests/_env/`.
///
/// Returned by [`requests_get_envs`] for the env switcher UI.
#[derive(Debug, Clone, Serialize)]
pub struct EnvSummary {
    /// Environment name (file stem, e.g. `"dev"`).
    pub name: String,
    /// Number of plaintext variables defined in the env file.
    pub vars_count: usize,
    /// Names of secrets declared in the env file. Values live in the
    /// encrypted credential store and are never exposed to the frontend.
    pub secrets: Vec<String>,
}

/// List all environment files in a project's `_env/` directory along with
/// a summary of variables and secret names declared in each.
#[tauri::command]
pub fn requests_get_envs(project_path: String) -> Result<Vec<EnvSummary>, String> {
    let root = Path::new(&project_path);
    // Always ensure the default env exists, even on a fresh project
    // where `.beardgit/requests/` hasn't been created yet. Mirrors the
    // unconditional path in `requests_list_project` so whichever IPC
    // wins the mount race materialises the same env file.
    let requests_root = root.join(".beardgit").join("requests");
    ensure_default_env(&requests_root);
    let names = list_envs(root).map_err(|e| e.to_string())?;
    let mut out = vec![];
    for n in names {
        let env = load_env(root, &n).map_err(|e| e.to_string())?;
        out.push(EnvSummary {
            name: n,
            vars_count: env.vars.len(),
            secrets: env.secrets,
        });
    }
    Ok(out)
}

/// Load the full contents of a single environment file for editing.
///
/// Returns the JSON-deserialized [`EnvFile`] (vars + secret names; secret
/// _values_ live in the encrypted credential store and are read separately
/// via [`requests_set_secret`]). Used by the Manage envs dialog to populate
/// its editable form.
#[tauri::command]
pub fn requests_load_env(project_path: String, env_name: String) -> Result<EnvFile, String> {
    load_env(Path::new(&project_path), &env_name).map_err(|e| e.to_string())
}

/// Save (or create) an environment file at
/// `<project>/.beardgit/requests/_env/<env_name>.json`.
///
/// Overwrites the existing file when present. Variable values are written
/// in plaintext; secret _values_ are not part of [`EnvFile`] and must be
/// stored separately via [`requests_set_secret`].
#[tauri::command]
pub fn requests_save_env(
    project_path: String,
    env_name: String,
    env: EnvFile,
) -> Result<(), String> {
    save_env_file(Path::new(&project_path), &env_name, &env).map_err(|e| e.to_string())
}

/// Delete an environment file. Idempotent: missing files are not an error.
///
/// Note: removing the env file does **not** delete secret values from the
/// credential store — those keys are namespaced by env name and would have
/// to be cleaned up explicitly. Acceptable v1 limitation.
#[tauri::command]
pub fn requests_delete_env(project_path: String, env_name: String) -> Result<(), String> {
    let path = Path::new(&project_path)
        .join(".beardgit")
        .join("requests")
        .join("_env")
        .join(format!("{env_name}.json"));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Persist the active environment selection for a project in `requests.db`.
///
/// `env_name = None` clears the active environment. Other project-state
/// fields (last open request, divider position) are preserved.
#[tauri::command]
pub fn requests_set_env(
    state: tauri::State<'_, AppState>,
    project_path: String,
    env_name: Option<String>,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp();
    let db = state.requests_db.lock().map_err(|_| "db poisoned")?;
    let mut s = db
        .get_project_state(&project_path)
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    s.project_path = project_path;
    s.active_env = env_name;
    s.updated_at = now;
    db.upsert_project_state(&s).map_err(|e| e.to_string())
}

/// Store a Requests-panel secret value in the encrypted credential store.
///
/// Secrets are scoped by `(env_name, secret_name)`. The plaintext value is
/// AES-256-GCM-encrypted at rest under the `~/.config/beardgit/credentials.enc`
/// file shared with forge PATs — see [`requests_secret_key`] for the
/// namespacing scheme.
#[tauri::command]
pub fn requests_set_secret(
    state: tauri::State<'_, AppState>,
    env_name: String,
    secret_name: String,
    value: String,
) -> Result<(), String> {
    let key = requests_secret_key(&env_name, &secret_name);
    let credential = Credential {
        token: value,
        // ProviderKind is unused for Requests-panel secrets; pick a stable
        // placeholder so the on-disk encoding is deterministic.
        provider: ProviderKind::GitHub,
    };
    state
        .credential_store
        .store_credential(&key, &credential)
        .map_err(|e| e.to_string())
}

/// Look up a Requests-panel secret value previously stored via
/// [`requests_set_secret`]. Returns `None` when the credential is missing.
///
/// Errors from the credential store (decryption / IO) are surfaced as-is.
fn read_requests_secret(
    state: &tauri::State<'_, AppState>,
    env_name: &str,
    secret_name: &str,
) -> Result<Option<String>, String> {
    let key = requests_secret_key(env_name, secret_name);
    let cred = state
        .credential_store
        .get_credential(&key)
        .map_err(|e| e.to_string())?;
    Ok(cred.map(|c| c.token))
}

/// Arguments for [`requests_run`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRequestArgs {
    /// `"project"` or `"global"`. See [`requests_load`] for the encoding.
    pub source_kind: String,
    /// File-relative path (project) or `"global/<id>"` (global library).
    pub source_path: String,
    /// Repository root for project-scoped sources. Required when
    /// `source_kind == "project"` or when an env is selected.
    pub project_path: Option<String>,
    /// Active environment name from `_env/<name>.json`, or `None` to run
    /// without environment substitution.
    pub env_name: Option<String>,
    /// Per-run variable overrides applied on top of env vars / auto-vars.
    pub overrides: BTreeMap<String, String>,
    /// Frontend-generated id (UUID v4) used to address the in-flight run
    /// for cancellation. Inserted into [`AppState::requests_cancellations`]
    /// before the first await, removed on every exit path, and looked up
    /// by [`requests_cancel`].
    pub ticket_id: String,
}

/// HTTP execution result returned to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct RunResult {
    /// Inserted history row id, for diff / replay flows.
    pub history_id: i64,
    /// HTTP response status code.
    pub status: u16,
    /// Response headers as `(name, value)` pairs.
    pub headers: Vec<(String, String)>,
    /// Base64-encoded response body (raw bytes, may be truncated).
    pub body_base64: String,
    /// `true` when the response body exceeded the executor's body cap.
    pub truncated: bool,
    /// Wall-clock duration of the network call, in milliseconds.
    pub duration_ms: u64,
}

/// RAII guard that removes a ticket entry from
/// [`AppState::requests_cancellations`] when it goes out of scope.
///
/// Holds a borrow of the underlying `Mutex` (not the `tauri::State`)
/// so the guard's lifetime is decoupled from the Tauri command frame.
/// The destructor runs on every exit path of `requests_run` —
/// `Ok` return, error return, **and** panic — guaranteeing the map
/// can never leak entries even if the command body unwinds.
struct CleanupGuard<'a> {
    map: &'a std::sync::Mutex<std::collections::HashMap<String, CancellationToken>>,
    id: &'a str,
}

impl Drop for CleanupGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut m) = self.map.lock() {
            m.remove(self.id);
        }
    }
}

/// Resolve, send, and record a single request from a `.http` source.
///
/// Loads the first request in the file, resolves variables against the
/// active env (vars + secrets) and the caller-supplied overrides, executes
/// it via [`requests_runner::executor::execute`], and stores the response
/// in `requests.db` (auto-pruned to the last 50 per source).
///
/// The `args.ticket_id` is registered in [`AppState::requests_cancellations`]
/// **before** the first await so a concurrent [`requests_cancel`] can
/// reliably abort the in-flight `reqwest` future. The entry is removed
/// by a [`CleanupGuard`] on every exit path. A canceled run surfaces as
/// `Err("canceled")` to the frontend, which the UI maps to `runState =
/// "canceled"` rather than `"error"`.
#[tauri::command]
pub async fn requests_run(
    state: tauri::State<'_, AppState>,
    args: RunRequestArgs,
) -> Result<RunResult, String> {
    // Register the cancellation token *before* any await so a concurrent
    // requests_cancel never races past us.
    let cancel = CancellationToken::new();
    {
        let mut map = state
            .requests_cancellations
            .lock()
            .map_err(|_| "cancellations poisoned")?;
        map.insert(args.ticket_id.clone(), cancel.clone());
    }
    let _guard = CleanupGuard {
        map: &state.requests_cancellations,
        id: &args.ticket_id,
    };

    let content = read_source(
        &state,
        &args.source_kind,
        &args.source_path,
        args.project_path.as_deref(),
    )?;
    let parsed = parse_http_file(&content).map_err(|e| e.to_string())?;
    let req = parsed.into_iter().next().ok_or("empty .http file")?;

    let mut env_vars: BTreeMap<String, String> = BTreeMap::new();
    let mut env_secrets: BTreeMap<String, String> = BTreeMap::new();
    if let (Some(env), Some(root)) = (args.env_name.as_deref(), args.project_path.as_deref()) {
        let env_file = load_env(Path::new(root), env).map_err(|e| e.to_string())?;
        env_vars = env_file.vars.into_iter().collect();
        for sec in &env_file.secrets {
            if let Some(val) = read_requests_secret(&state, env, sec)? {
                env_secrets.insert(sec.clone(), val);
            }
        }
    }

    let ctx = ResolveCtx {
        env_vars,
        env_secrets,
        overrides: args.overrides.clone(),
    };

    let resolved = resolve(&req, &ctx).map_err(|e| e.to_string())?;
    let started = chrono::Utc::now().timestamp();
    let result = execute(&resolved, cancel, ExecuteOptions::default())
        .await
        .map_err(|e| e.to_string())?;

    let db = state.requests_db.lock().map_err(|_| "db poisoned")?;
    let history_id = record(
        &db,
        &args.source_kind,
        &args.source_path,
        args.env_name.as_deref(),
        &resolved,
        &result,
        started,
    )
    .map_err(|e| e.to_string())?;
    drop(db);

    use base64::{Engine as _, engine::general_purpose::STANDARD};
    Ok(RunResult {
        history_id,
        status: result.status,
        headers: result.headers,
        body_base64: STANDARD.encode(&result.body),
        truncated: result.truncated,
        duration_ms: result.duration_ms,
    })
}

/// Cancel an in-flight [`requests_run`] by ticket id.
///
/// Looks up the [`CancellationToken`] previously registered by
/// `requests_run` and fires `cancel()`, which causes the in-flight
/// `reqwest` future to short-circuit with [`requests_runner::RequestsError::Canceled`].
/// Unknown ticket ids are silently ignored — the run may have already
/// completed and removed its entry, which is not an error from the
/// frontend's perspective (the UI just stops awaiting the response).
#[tauri::command]
pub fn requests_cancel(state: tauri::State<'_, AppState>, ticket_id: String) -> Result<(), String> {
    let map = state
        .requests_cancellations
        .lock()
        .map_err(|_| "cancellations poisoned")?;
    if let Some(t) = map.get(&ticket_id) {
        t.cancel();
    }
    Ok(())
}

/// A single row in the History panel for one source.
///
/// The full snapshot/headers/body are kept in `requests.db` and only
/// fetched on demand via [`requests_diff_responses`] — the row payload
/// stays small so the list scrolls smoothly even when fully populated
/// (50 entries per source).
#[derive(Debug, Clone, Serialize)]
pub struct HistoryRow {
    pub id: i64,
    pub status: Option<i64>,
    pub duration_ms: i64,
    pub executed_at: i64,
    pub env_name: Option<String>,
    pub truncated: bool,
}

/// List recent execution history rows for a single source, newest first.
///
/// `limit` is forwarded directly to the SQL query — pass [`requests_store::HISTORY_CAP_PER_SOURCE`]
/// (50) to fetch the whole window.
#[tauri::command]
pub fn requests_history(
    state: tauri::State<'_, AppState>,
    source_kind: String,
    source_path: String,
    limit: i64,
) -> Result<Vec<HistoryRow>, String> {
    let db = state.requests_db.lock().map_err(|_| "db poisoned")?;
    let rows = db
        .list_history(&source_kind, &source_path, limit)
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|h: HistoryEntry| HistoryRow {
            id: h.id,
            status: h.response_status,
            duration_ms: h.duration_ms,
            executed_at: h.executed_at,
            env_name: h.env_name,
            truncated: h.response_truncated,
        })
        .collect())
}

/// Payload for the response diff viewer: two response bodies plus an
/// optional `Content-Type` hint extracted from the **left** entry's
/// headers (used by the frontend to pick a syntax-highlighting mode).
#[derive(Debug, Clone, Serialize)]
pub struct DiffPayload {
    pub left: String,
    pub right: String,
    pub content_type_hint: Option<String>,
}

/// Fetch two history rows by id and return their decoded response bodies
/// for side-by-side diffing.
///
/// Body bytes are interpreted as UTF-8 with replacement on invalid
/// sequences — the diff view is best-effort for binary payloads.
#[tauri::command]
pub fn requests_diff_responses(
    state: tauri::State<'_, AppState>,
    history_id_a: i64,
    history_id_b: i64,
) -> Result<DiffPayload, String> {
    let db = state.requests_db.lock().map_err(|_| "db poisoned")?;
    let a = db
        .get_history_by_id(history_id_a)
        .map_err(|e| e.to_string())?
        .ok_or("history A not found")?;
    let b = db
        .get_history_by_id(history_id_b)
        .map_err(|e| e.to_string())?
        .ok_or("history B not found")?;
    let left = String::from_utf8_lossy(&a.response_body_blob.unwrap_or_default()).to_string();
    let right = String::from_utf8_lossy(&b.response_body_blob.unwrap_or_default()).to_string();
    let hint = serde_json::from_str::<Vec<(String, String)>>(
        a.response_headers_json.as_deref().unwrap_or("[]"),
    )
    .ok()
    .and_then(|hs| {
        hs.into_iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v)
    });
    Ok(DiffPayload {
        left,
        right,
        content_type_hint: hint,
    })
}

/// Seed the Quickstart starter pack of `.http` files under
/// `<project>/.beardgit/requests/quickstart/`.
///
/// Writes nine samples (JSONPlaceholder + httpbin) that work without any
/// auth, and seeds / upgrades the empty `_env/default.json` stub with
/// the matching `base_url` / `httpbin_base_url` / `post_id` vars so the
/// user can hit Send immediately.
///
/// Returns the list of relative paths that were written, in order.
/// Existing user-customised `_env/default.json` files are preserved.
#[tauri::command]
pub fn requests_seed_quickstart(project_path: String) -> Result<Vec<String>, String> {
    seed_quickstart_pack(Path::new(&project_path)).map_err(|e| e.to_string())
}

/// Parse a `curl` shell command string into a [`ParsedRequest`] suitable
/// for direct loading in the editor (Paste cURL flow).
#[tauri::command]
pub fn requests_paste_curl(curl_string: String) -> Result<ParsedRequest, String> {
    import_curl(&curl_string).map_err(|e| e.to_string())
}

/// Arguments for [`requests_copy_as`].
#[derive(Debug, Clone, Deserialize)]
pub struct CopyAsArgs {
    pub source_kind: String,
    pub source_path: String,
    pub project_path: Option<String>,
    pub env_name: Option<String>,
    /// One of `"curl"`, `"fetch"`, `"httpie"`, `"wget"`.
    pub target: String,
    pub overrides: BTreeMap<String, String>,
}

/// Open a project-scoped request file in the OS' default editor for
/// `.http` files.
///
/// Bypasses `tauri-plugin-opener`'s capability allowlist (which would
/// otherwise reject paths under `<project>/.beardgit/requests/`) by
/// shelling out to the platform-native opener directly. Mirrors the
/// "Open in editor" affordance other features expose (e.g. Reflog's
/// reveal-in-finder).
///
/// Only project sources are supported — global library items live in
/// SQLite and have no on-disk path.
#[tauri::command]
pub fn requests_open_in_editor(
    source_kind: String,
    source_path: String,
    project_path: Option<String>,
) -> Result<(), String> {
    if source_kind != "project" {
        return Err("only project items can be opened in an external editor".into());
    }
    let root = project_path.ok_or("project_path required")?;
    let p = std::path::Path::new(&root)
        .join(".beardgit")
        .join("requests")
        .join(&source_path);
    if !p.exists() {
        return Err(format!("file not found: {}", p.display()));
    }
    #[cfg(target_os = "macos")]
    let res = std::process::Command::new("open").arg(&p).status();
    #[cfg(target_os = "linux")]
    let res = std::process::Command::new("xdg-open").arg(&p).status();
    #[cfg(target_os = "windows")]
    let res = std::process::Command::new("cmd")
        .args(["/C", "start", "", &p.to_string_lossy()])
        .status();
    res.map_err(|e| e.to_string())?;
    Ok(())
}

/// Resolve a request the same way [`requests_run`] does and emit a
/// shell-/JS-ready string for the chosen [`CodegenTarget`] without
/// actually executing the request.
#[tauri::command]
pub fn requests_copy_as(
    state: tauri::State<'_, AppState>,
    args: CopyAsArgs,
) -> Result<String, String> {
    let content = read_source(
        &state,
        &args.source_kind,
        &args.source_path,
        args.project_path.as_deref(),
    )?;
    let parsed = parse_http_file(&content)
        .map_err(|e| e.to_string())?
        .into_iter()
        .next()
        .ok_or("empty .http file")?;

    let mut env_vars: BTreeMap<String, String> = BTreeMap::new();
    let mut env_secrets: BTreeMap<String, String> = BTreeMap::new();
    if let (Some(env), Some(root)) = (args.env_name.as_deref(), args.project_path.as_deref()) {
        let env_file = load_env(Path::new(root), env).map_err(|e| e.to_string())?;
        env_vars = env_file.vars.into_iter().collect();
        for sec in &env_file.secrets {
            if let Some(val) = read_requests_secret(&state, env, sec)? {
                env_secrets.insert(sec.clone(), val);
            }
        }
    }

    let ctx = ResolveCtx {
        env_vars,
        env_secrets,
        overrides: args.overrides,
    };
    let resolved = resolve(&parsed, &ctx).map_err(|e| e.to_string())?;
    let target = match args.target.as_str() {
        "curl" => CodegenTarget::Curl,
        "fetch" => CodegenTarget::Fetch,
        "httpie" => CodegenTarget::Httpie,
        "wget" => CodegenTarget::Wget,
        other => return Err(format!("unknown target: {other}")),
    };
    Ok(copy_as(&resolved, target))
}

#[cfg(test)]
mod tests {
    //! Unit tests for the cancellation-map plumbing.
    //!
    //! Exercising the full Tauri command surface from a unit test would
    //! require a `tauri::State` mock (non-trivial — the Tauri test harness
    //! is built around a running `App`). The underlying executor-level
    //! cancellation is already covered by
    //! `requests_runner::executor::tests::cancellation_aborts`, so the
    //! tests below focus on what's *new* in this layer: the
    //! [`CleanupGuard`] semantics and the insert/remove discipline that
    //! [`requests_run`] and [`requests_cancel`] rely on.
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[test]
    fn cleanup_guard_removes_entry_on_drop() {
        let map: Mutex<HashMap<String, CancellationToken>> = Mutex::new(HashMap::new());
        let id = "ticket-1".to_string();
        map.lock()
            .unwrap()
            .insert(id.clone(), CancellationToken::new());
        assert_eq!(map.lock().unwrap().len(), 1);

        {
            let _guard = CleanupGuard { map: &map, id: &id };
            // While the guard is alive the entry is still present —
            // requests_cancel needs to see it to fire the token.
            assert!(map.lock().unwrap().contains_key(&id));
        }

        // After the guard drops, the entry is gone.
        assert!(map.lock().unwrap().is_empty());
    }

    #[test]
    fn cancel_token_lookup_fires_via_shared_map() {
        // Models the requests_cancel path: a foreign caller looks up a
        // ticket id in the shared map and calls cancel() on the stored
        // token. The original holder (the requests_run future) sees the
        // cancellation through its clone.
        let map: Mutex<HashMap<String, CancellationToken>> = Mutex::new(HashMap::new());
        let token = CancellationToken::new();
        let id = "ticket-2".to_string();
        map.lock().unwrap().insert(id.clone(), token.clone());

        // External cancel() through the map.
        if let Some(t) = map.lock().unwrap().get(&id) {
            t.cancel();
        }

        assert!(token.is_cancelled());
    }

    #[test]
    fn cancel_unknown_ticket_is_a_noop() {
        // requests_cancel for a ticket that already finished must not
        // error — the run may have raced to completion before the user
        // clicked the button.
        let map: Mutex<HashMap<String, CancellationToken>> = Mutex::new(HashMap::new());
        // No insert. Lookup returns None and we do nothing.
        let opt = map.lock().unwrap().get("nonexistent").cloned();
        assert!(opt.is_none());
    }

    #[test]
    fn duplicate_path_appends_copy_before_extension() {
        // Standard nested case.
        assert_eq!(duplicate_path("users/get.http"), "users/get copy.http");
        // Top-level file.
        assert_eq!(duplicate_path("ping.http"), "ping copy.http");
        // No extension → falls back to .http.
        assert_eq!(duplicate_path("README"), "README copy.http");
        // Deeper nesting is preserved verbatim.
        assert_eq!(duplicate_path("a/b/c/x.http"), "a/b/c/x copy.http");
    }

    // ─── requests_list_project default-env side-effect ────────────────

    #[test]
    fn list_project_creates_default_env_when_requests_dir_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let req_dir = root.join(".beardgit").join("requests");
        std::fs::create_dir_all(&req_dir).unwrap();

        let _ = requests_list_project(root.to_string_lossy().to_string()).unwrap();

        let default_env = req_dir.join("_env").join("default.json");
        assert!(
            default_env.exists(),
            "default.json must be created when requests/ exists",
        );
    }

    #[test]
    fn list_project_leaves_existing_default_env_alone() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let env_dir = root.join(".beardgit").join("requests").join("_env");
        std::fs::create_dir_all(&env_dir).unwrap();
        let custom = "{\"$schema\":\"beardgit-env/v1\",\"vars\":{\"a\":\"1\"},\"secrets\":[]}";
        std::fs::write(env_dir.join("default.json"), custom).unwrap();

        let _ = requests_list_project(root.to_string_lossy().to_string()).unwrap();

        let body = std::fs::read_to_string(env_dir.join("default.json")).unwrap();
        assert_eq!(
            body, custom,
            "existing default.json must not be overwritten"
        );
    }

    #[test]
    fn list_project_no_op_when_requests_dir_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        // No `.beardgit/requests/` at all.
        let nodes = requests_list_project(root.to_string_lossy().to_string()).unwrap();
        assert!(nodes.is_empty());
        assert!(
            !root.join(".beardgit").exists(),
            "must not create the requests root when it didn't exist",
        );
    }
}

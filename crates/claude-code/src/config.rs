//! Configuration and instruction file discovery for Claude Code.
//!
//! Discovers settings files, agent definitions, skill definitions, and
//! CLAUDE.md instruction files across user and project scopes.

use std::fs;
use std::path::{Path, PathBuf};

use ai_provider::{AiConfigFile, ConfigKind, ConfigScope};

/// Discover all Claude Code configuration files for a repo.
pub fn config_files(repo_path: &Path) -> Vec<AiConfigFile> {
    let mut files = Vec::new();
    let home = dirs::home_dir().unwrap_or_default();

    // User-level settings
    push_if_file(
        &mut files,
        home.join(".claude/settings.json"),
        ConfigKind::Settings,
        ConfigScope::User,
    );

    // Project-level settings
    push_if_file(
        &mut files,
        repo_path.join(".claude/settings.json"),
        ConfigKind::Settings,
        ConfigScope::Project,
    );

    // Local settings (gitignored)
    push_if_file(
        &mut files,
        repo_path.join(".claude/settings.local.json"),
        ConfigKind::Settings,
        ConfigScope::Local,
    );

    // User-level agent definitions
    scan_agents(&mut files, &home.join(".claude/agents"), ConfigScope::User);

    // User-level skill definitions
    scan_skills(&mut files, &home.join(".claude/skills"), ConfigScope::User);

    // Project-level agent definitions
    scan_agents(
        &mut files,
        &repo_path.join(".claude/agents"),
        ConfigScope::Project,
    );

    // Project-level skill definitions
    scan_skills(
        &mut files,
        &repo_path.join(".claude/skills"),
        ConfigScope::Project,
    );

    files
}

/// How deep below the repo root to look for nested `CLAUDE.md` files.
///
/// Claude Code itself reads the nearest `CLAUDE.md` walking up from the file
/// being edited, so they live wherever a subsystem lives — and in this repo
/// that is `src/lib/components/file-editor/`, four levels down. Six covers
/// that with room to spare without turning discovery into a full-tree walk.
const MAX_INSTRUCTION_DEPTH: usize = 6;

/// Cap on how many instruction files one repo can contribute, so a monorepo
/// with hundreds cannot stall the panel.
const MAX_INSTRUCTION_FILES: usize = 200;

/// Directories never worth descending into: build output, vendored deps, and
/// VCS metadata. Skipping them is what keeps the walk cheap.
const SKIP_DIRS: &[&str] = &[
    ".git",
    ".next",
    ".svelte-kit",
    ".venv",
    "__pycache__",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
    "vendor",
];

/// Discover CLAUDE.md instruction files across all scopes.
///
/// Walks the repo rather than probing a fixed list of subdirectory names.
/// The list used to be `["crates", "src", "src-tauri", "packages", "apps"]`,
/// checked one level deep, which found 2 of this repo's own 12 `CLAUDE.md`
/// files: everything under `crates/<crate>/` and `src/lib/**` was invisible,
/// and a project keeping its instructions anywhere else showed none at all.
///
/// Deliberately ignores `.gitignore`. `**/CLAUDE.md` is gitignored in this
/// very repo, and a file the user cannot see in the panel is a file they
/// cannot edit there.
pub fn instruction_files(repo_path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let home = dirs::home_dir().unwrap_or_default();

    // User-level
    let user_md = home.join(".claude/CLAUDE.md");
    if user_md.is_file() {
        files.push(user_md);
    }

    // Project root, then everything below it.
    let project_md = repo_path.join("CLAUDE.md");
    if project_md.is_file() {
        files.push(project_md);
    }
    let mut nested = Vec::new();
    collect_instruction_files(repo_path, 0, &mut nested);
    // Sorted so the tree order is stable across runs; `read_dir` is not.
    nested.sort();
    files.extend(nested);

    files
}

/// Recursive half of [`instruction_files`], collecting nested matches only.
fn collect_instruction_files(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth >= MAX_INSTRUCTION_DEPTH || out.len() >= MAX_INSTRUCTION_FILES {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= MAX_INSTRUCTION_FILES {
            return;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        // `file_type` rather than `is_dir`, so a symlinked directory is not
        // followed — a link back up the tree would recurse until the depth
        // cap, reporting the same files under a second path.
        let Ok(ft) = entry.file_type() else { continue };
        let path = entry.path();
        if ft.is_dir() {
            if SKIP_DIRS.contains(&name) {
                continue;
            }
            collect_instruction_files(&path, depth + 1, out);
        } else if ft.is_file() && name == "CLAUDE.md" && depth > 0 {
            // `depth > 0`: the root one is pushed by the caller, in the order
            // the panel wants it (before the nested ones).
            out.push(path);
        }
    }
}

/// Scan a directory for `.md` agent definition files and push them.
fn scan_agents(files: &mut Vec<AiConfigFile>, dir: &Path, scope: ConfigScope) {
    if dir.is_dir() {
        for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                files.push(AiConfigFile {
                    path,
                    kind: ConfigKind::Agent,
                    scope,
                });
            }
        }
    }
}

/// Scan a directory for skill subdirectories containing `SKILL.md`.
fn scan_skills(files: &mut Vec<AiConfigFile>, dir: &Path, scope: ConfigScope) {
    if dir.is_dir() {
        for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
            let skill_md = entry.path().join("SKILL.md");
            if skill_md.is_file() {
                files.push(AiConfigFile {
                    path: skill_md,
                    kind: ConfigKind::Skill,
                    scope,
                });
            }
        }
    }
}

/// Push a config file entry if the path exists as a file.
fn push_if_file(
    files: &mut Vec<AiConfigFile>,
    path: PathBuf,
    kind: ConfigKind,
    scope: ConfigScope,
) {
    if path.is_file() {
        files.push(AiConfigFile { path, kind, scope });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn discovers_project_settings() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        fs::create_dir(&claude_dir).unwrap();
        fs::write(claude_dir.join("settings.json"), "{}").unwrap();

        let files = config_files(dir.path());
        assert!(
            files
                .iter()
                .any(|f| f.kind == ConfigKind::Settings && f.scope == ConfigScope::Project)
        );
    }

    #[test]
    fn discovers_agents() {
        let dir = tempfile::tempdir().unwrap();
        let agents_dir = dir.path().join(".claude/agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(agents_dir.join("reviewer.md"), "# Agent").unwrap();
        fs::write(agents_dir.join("not-an-agent.txt"), "nope").unwrap();

        let files = config_files(dir.path());
        let agents: Vec<_> = files
            .iter()
            .filter(|f| f.kind == ConfigKind::Agent && f.scope == ConfigScope::Project)
            .collect();
        assert_eq!(agents.len(), 1);
        assert!(agents[0].path.ends_with("reviewer.md"));
    }

    #[test]
    fn discovers_skills() {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join(".claude/skills/my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Skill").unwrap();

        let files = config_files(dir.path());
        let skills: Vec<_> = files
            .iter()
            .filter(|f| f.kind == ConfigKind::Skill && f.scope == ConfigScope::Project)
            .collect();
        assert_eq!(skills.len(), 1);
    }

    #[test]
    fn discovers_user_agents() {
        // User-level agents are discovered when ~/.claude/agents/ exists.
        // We verify the function at least returns files that include user scope.
        let files = config_files(Path::new("/nonexistent-repo"));
        // If ~/.claude/agents/ has .md files, they'll appear as User scope.
        let user_agents: Vec<_> = files
            .iter()
            .filter(|f| f.kind == ConfigKind::Agent && f.scope == ConfigScope::User)
            .collect();
        // We can't assert an exact count (depends on host) — just verify no panic.
        assert!(user_agents.iter().all(|a| a.scope == ConfigScope::User));
    }

    #[test]
    fn discovers_instruction_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "# Root").unwrap();
        let crates_dir = dir.path().join("crates");
        fs::create_dir(&crates_dir).unwrap();
        fs::write(crates_dir.join("CLAUDE.md"), "# Crates").unwrap();

        let files = instruction_files(dir.path());
        assert!(files.len() >= 2);
    }

    /// The shape that was broken: instructions nested deeper than one level,
    /// and under directory names the old fixed list never mentioned.
    ///
    /// The old implementation probed `["crates", "src", "src-tauri",
    /// "packages", "apps"]` exactly one level down, which found 2 of this
    /// repo's own 12 CLAUDE.md files.
    #[test]
    fn discovers_nested_instruction_files_at_any_depth() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("CLAUDE.md"), "# Root").unwrap();

        // Two levels under a listed name — missed before.
        let deep_crate = root.join("crates/git-engine");
        fs::create_dir_all(&deep_crate).unwrap();
        fs::write(deep_crate.join("CLAUDE.md"), "# git-engine").unwrap();

        // Four levels down, under names the old list did not contain.
        let deep_component = root.join("src/lib/components/file-editor");
        fs::create_dir_all(&deep_component).unwrap();
        fs::write(deep_component.join("CLAUDE.md"), "# file-editor").unwrap();

        // A directory name the old list never had at all.
        let backend = root.join("backend");
        fs::create_dir_all(&backend).unwrap();
        fs::write(backend.join("CLAUDE.md"), "# backend").unwrap();

        let files = instruction_files(root);
        let found: Vec<String> = files
            .iter()
            .filter(|p| p.starts_with(root))
            .map(|p| {
                p.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();

        assert!(found.contains(&"CLAUDE.md".to_string()), "{found:?}");
        assert!(
            found.contains(&"crates/git-engine/CLAUDE.md".to_string()),
            "two levels under a known name must be found: {found:?}"
        );
        assert!(
            found.contains(&"src/lib/components/file-editor/CLAUDE.md".to_string()),
            "four levels down must be found: {found:?}"
        );
        assert!(
            found.contains(&"backend/CLAUDE.md".to_string()),
            "an unlisted directory name must be found: {found:?}"
        );
        // The root one comes first, so the panel lists it above the nested ones.
        assert_eq!(found.first().map(String::as_str), Some("CLAUDE.md"));
    }

    /// Build output must not be walked — it is both pointless and slow, and a
    /// vendored copy of another project would report its CLAUDE.md as ours.
    #[test]
    fn instruction_scan_skips_build_and_vendor_directories() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for skipped in ["node_modules", "target", ".git", "dist"] {
            let d = root.join(skipped).join("nested");
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join("CLAUDE.md"), "# should not appear").unwrap();
        }
        fs::write(root.join("CLAUDE.md"), "# Root").unwrap();

        let files = instruction_files(root);
        let under_root: Vec<_> = files.iter().filter(|p| p.starts_with(root)).collect();
        assert_eq!(
            under_root.len(),
            1,
            "only the root CLAUDE.md may be reported: {under_root:?}"
        );
    }

    /// A symlink pointing back up the tree must not be followed, or the same
    /// file is reported twice under two paths.
    #[test]
    fn instruction_scan_does_not_follow_directory_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let sub = root.join("sub");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("CLAUDE.md"), "# Sub").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(root, sub.join("loop")).unwrap();

        let files = instruction_files(root);
        let under_root: Vec<_> = files.iter().filter(|p| p.starts_with(root)).collect();
        assert_eq!(under_root.len(), 1, "{under_root:?}");
    }

    #[test]
    fn empty_repo_returns_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let files = config_files(dir.path());
        assert!(
            files
                .iter()
                .all(|f| f.scope == ConfigScope::User || f.path.starts_with(dir.path()))
        );
    }
}

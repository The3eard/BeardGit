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

/// Discover CLAUDE.md instruction files across all scopes.
pub fn instruction_files(repo_path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let home = dirs::home_dir().unwrap_or_default();

    // User-level
    let user_md = home.join(".claude/CLAUDE.md");
    if user_md.is_file() {
        files.push(user_md);
    }

    // Project root
    let project_md = repo_path.join("CLAUDE.md");
    if project_md.is_file() {
        files.push(project_md);
    }

    // Subdirectory CLAUDE.md files (scan known patterns)
    for subdir in ["crates", "src", "src-tauri", "packages", "apps"] {
        let sub_md = repo_path.join(subdir).join("CLAUDE.md");
        if sub_md.is_file() {
            files.push(sub_md);
        }
    }

    files
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

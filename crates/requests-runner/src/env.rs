//! Load/save `.beardgit/requests/_env/<name>.json` files and discover
//! the list of available envs in a project.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::RequestsError;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvFile {
    #[serde(rename = "$schema", default = "default_schema")]
    pub schema: String,
    #[serde(default)]
    pub vars: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub secrets: Vec<String>,
}

fn default_schema() -> String {
    "beardgit-env/v1".into()
}

pub fn env_dir(project_root: &Path) -> PathBuf {
    project_root.join(".beardgit").join("requests").join("_env")
}

pub fn load_env(project_root: &Path, name: &str) -> Result<EnvFile, RequestsError> {
    let path = env_dir(project_root).join(format!("{name}.json"));
    let content = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&content)?)
}

pub fn save_env(project_root: &Path, name: &str, env: &EnvFile) -> Result<(), RequestsError> {
    let dir = env_dir(project_root);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{name}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(env)?)?;
    Ok(())
}

pub fn list_envs(project_root: &Path) -> Result<Vec<String>, RequestsError> {
    let dir = env_dir(project_root);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut names = vec![];
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str())
            && entry.path().extension().and_then(|s| s.to_str()) == Some("json")
        {
            names.push(stem.to_string());
        }
    }
    names.sort();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut env = EnvFile {
            schema: "beardgit-env/v1".into(),
            ..Default::default()
        };
        env.vars.insert("base_url".into(), "https://x".into());
        env.secrets.push("TOKEN".into());
        save_env(dir.path(), "dev", &env).unwrap();
        let loaded = load_env(dir.path(), "dev").unwrap();
        assert_eq!(loaded.vars.get("base_url").unwrap(), "https://x");
        assert_eq!(loaded.secrets, vec!["TOKEN"]);
    }

    #[test]
    fn list_envs_returns_sorted() {
        let dir = tempfile::tempdir().unwrap();
        save_env(dir.path(), "prod", &EnvFile::default()).unwrap();
        save_env(dir.path(), "dev", &EnvFile::default()).unwrap();
        save_env(dir.path(), "staging", &EnvFile::default()).unwrap();
        let names = list_envs(dir.path()).unwrap();
        assert_eq!(names, vec!["dev", "prod", "staging"]);
    }

    #[test]
    fn list_envs_missing_dir_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_envs(dir.path()).unwrap().is_empty());
    }
}

//! Folder content counter used by the init-repo dialog to preview
//! how many files would be staged for the initial commit.

use std::path::Path;

#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct FolderCount {
    pub files: u64,
    pub bytes: u64,
    /// True when the cap was hit and the count was truncated.
    pub truncated: bool,
}

const FILE_CAP: u64 = 50_000;
const BYTE_CAP: u64 = 1024 * 1024 * 1024; // 1 GiB

/// Walk `path` and count files + bytes that *would* be committed.
///
/// Respects any pre-existing `.gitignore` plus a built-in skiplist
/// (`.git/`, `node_modules/`, `target/`, `.venv/`, `dist/`, `build/`,
/// `.next/`, `__pycache__/`). Hard-caps at 50k files / 1 GiB.
pub fn walk(path: &Path) -> FolderCount {
    use ignore::WalkBuilder;
    let mut files = 0u64;
    let mut bytes = 0u64;
    let mut truncated = false;

    let extra: std::collections::HashSet<&'static str> = [
        ".git",
        "node_modules",
        "target",
        ".venv",
        "dist",
        "build",
        ".next",
        "__pycache__",
    ]
    .into_iter()
    .collect();
    let extra_for_filter = extra;

    let mut builder = WalkBuilder::new(path);
    builder
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(false)
        .require_git(false)
        .filter_entry(move |dent| {
            dent.file_name()
                .to_str()
                .map(|n| !extra_for_filter.contains(n))
                .unwrap_or(true)
        });
    let walker = builder.build();
    for dent in walker {
        let dent = match dent {
            Ok(d) => d,
            Err(_) => continue,
        };
        if dent.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        files += 1;
        if let Ok(meta) = dent.metadata() {
            bytes += meta.len();
        }
        if files >= FILE_CAP || bytes >= BYTE_CAP {
            truncated = true;
            break;
        }
    }
    FolderCount {
        files,
        bytes,
        truncated,
    }
}

/// Tauri command: count files + bytes in a folder for the init-repo preview.
#[tauri::command]
#[tracing::instrument(name = "cmd::folder_count")]
pub fn count_folder_contents(path: String) -> Result<FolderCount, String> {
    Ok(walk(Path::new(&path)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn counts_simple_folder() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a.txt"), "abc").unwrap();
        fs::write(tmp.path().join("b.txt"), "defg").unwrap();
        let c = walk(tmp.path());
        assert_eq!(c.files, 2);
        assert_eq!(c.bytes, 7);
        assert!(!c.truncated);
    }

    #[test]
    fn skips_node_modules_without_gitignore() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("keep.txt"), "x").unwrap();
        fs::create_dir(tmp.path().join("node_modules")).unwrap();
        fs::write(tmp.path().join("node_modules/skip.js"), "yyyy").unwrap();
        let c = walk(tmp.path());
        assert_eq!(c.files, 1);
        assert_eq!(c.bytes, 1);
    }

    #[test]
    fn respects_existing_gitignore() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join(".gitignore"), "ignored.txt\n").unwrap();
        fs::write(tmp.path().join("kept.txt"), "k").unwrap();
        fs::write(tmp.path().join("ignored.txt"), "ignored").unwrap();
        let c = walk(tmp.path());
        // .gitignore (12 bytes: "ignored.txt\n") + kept.txt (1 byte: "k") = 2 files.
        // ignored.txt is filtered out by the gitignore.
        assert_eq!(c.files, 2);
        assert_eq!(c.bytes, 1 + 12);
    }
}

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct FileEntry {
    pub path: String,
    #[serde(skip)]
    pub abs: PathBuf,
    pub size: u64,
    pub ext: String,
}

pub fn walk(root: &Path) -> anyhow::Result<Vec<FileEntry>> {
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(false)
        .require_git(false)
        .filter_entry(|e| e.file_name() != ".git")
        .build();

    let mut files = Vec::new();
    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let abs = entry.into_path();
        let rel = abs.strip_prefix(root).unwrap_or(&abs);
        let size = std::fs::metadata(&abs).map(|m| m.len()).unwrap_or(0);
        let ext = abs
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        files.push(FileEntry {
            path: rel.to_string_lossy().replace('\\', "/"),
            abs,
            size,
            ext,
        });
    }
    Ok(files)
}

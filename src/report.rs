use std::path::Path;

use serde::Serialize;

use crate::deps::Dependency;
use crate::dupes::DupGroup;
use crate::git::GitInfo;
use crate::imports::Insights;
use crate::scan::FileEntry;
use crate::stack::Stack;

#[derive(Serialize)]
pub struct Report {
    pub root: String,
    pub generated_at: i64,
    pub summary: Summary,
    pub stack: Stack,
    pub insights: Insights,
    pub dependencies: Vec<Dependency>,
    pub duplicates: Vec<DupGroup>,
    pub git: Option<GitInfo>,
    pub files: Vec<FileEntry>,
}

#[derive(Serialize)]
pub struct Summary {
    pub files: usize,
    pub dirs: usize,
    pub bytes: u64,
}

const TEMPLATE: &str = include_str!("../templates/report.html");

impl Report {
    pub fn write_json(&self, path: &Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn write_html(&self, path: &Path) -> anyhow::Result<()> {
        let data = serde_json::to_string(self)?.replace("</", "<\\/");
        let html = TEMPLATE.replace("\"__DATA__\"", &data);
        std::fs::write(path, html)?;
        Ok(())
    }
}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::human_bytes;

    #[test]
    fn formats_sizes() {
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1024), "1.0 KB");
        assert_eq!(human_bytes(1536), "1.5 KB");
    }
}

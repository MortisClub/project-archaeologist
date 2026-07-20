use std::collections::HashMap;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::scan::FileEntry;

#[derive(Serialize)]
pub struct DupGroup {
    pub hash: String,
    pub size: u64,
    pub paths: Vec<String>,
}

pub fn find(files: &[FileEntry]) -> Vec<DupGroup> {
    let mut by_size: HashMap<u64, Vec<&FileEntry>> = HashMap::new();
    for f in files {
        if f.size == 0 {
            continue;
        }
        by_size.entry(f.size).or_default().push(f);
    }

    let mut groups: HashMap<String, DupGroup> = HashMap::new();
    for candidates in by_size.into_values() {
        if candidates.len() < 2 {
            continue;
        }
        for f in candidates {
            let data = match std::fs::read(&f.abs) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let hash = format!("{:x}", Sha256::digest(&data));
            groups
                .entry(hash.clone())
                .or_insert_with(|| DupGroup {
                    hash,
                    size: f.size,
                    paths: Vec::new(),
                })
                .paths
                .push(f.path.clone());
        }
    }

    let mut out: Vec<DupGroup> = groups.into_values().filter(|g| g.paths.len() > 1).collect();
    out.sort_by_key(|g| std::cmp::Reverse(g.size * g.paths.len() as u64));
    out
}

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use serde::Serialize;

#[derive(Serialize)]
pub struct GitInfo {
    pub commits: usize,
    pub first_commit_at: Option<i64>,
    pub last_commit_at: Option<i64>,
    pub hotspots: Vec<Hotspot>,
    pub first_seen: Vec<FirstSeen>,
    pub dir_growth: Vec<DirGrowth>,
}

#[derive(Serialize)]
pub struct Hotspot {
    pub path: String,
    pub changes: usize,
}

#[derive(Serialize)]
pub struct FirstSeen {
    pub path: String,
    pub at: i64,
}

#[derive(Serialize)]
pub struct DirGrowth {
    pub dir: String,
    pub files_added: usize,
    pub first_at: i64,
}

pub fn analyze(root: &Path, churn: &HashMap<String, usize>) -> Option<GitInfo> {
    run(root, &["rev-parse", "--is-inside-work-tree"])?;

    let commits = run(root, &["rev-list", "--count", "HEAD"])
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    if commits == 0 {
        return None;
    }

    let last_commit_at =
        run(root, &["log", "-1", "--format=%ct"]).and_then(|s| s.trim().parse().ok());
    let first_commit_at = run(root, &["log", "--reverse", "--format=%ct"])
        .and_then(|s| s.lines().next().and_then(|l| l.trim().parse().ok()));

    let hotspots = hotspots(churn);
    let first_seen = first_seen(root);
    let dir_growth = dir_growth(&first_seen);

    Some(GitInfo {
        commits,
        first_commit_at,
        last_commit_at,
        hotspots,
        first_seen: first_seen.into_iter().take(40).collect(),
        dir_growth,
    })
}

pub fn churn_map(root: &Path) -> HashMap<String, usize> {
    let log = match run(root, &["log", "--format=", "--name-only"]) {
        Some(l) => l,
        None => return HashMap::new(),
    };
    let mut counts: HashMap<String, usize> = HashMap::new();
    for line in log.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        *counts.entry(line.to_string()).or_insert(0) += 1;
    }
    counts
}

fn hotspots(churn: &HashMap<String, usize>) -> Vec<Hotspot> {
    let mut out: Vec<Hotspot> = churn
        .iter()
        .map(|(path, &changes)| Hotspot {
            path: path.clone(),
            changes,
        })
        .collect();
    out.sort_by(|a, b| b.changes.cmp(&a.changes).then(a.path.cmp(&b.path)));
    out.truncate(25);
    out
}

fn first_seen(root: &Path) -> Vec<FirstSeen> {
    let log = match run(
        root,
        &[
            "log",
            "--diff-filter=A",
            "--reverse",
            "--format=@%ct",
            "--name-only",
        ],
    ) {
        Some(l) => l,
        None => return Vec::new(),
    };

    let mut seen: HashMap<String, i64> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut ts = 0i64;
    for line in log.lines() {
        if let Some(rest) = line.strip_prefix('@') {
            ts = rest.trim().parse().unwrap_or(ts);
        } else if !line.trim().is_empty() {
            let path = line.trim().to_string();
            if !seen.contains_key(&path) {
                seen.insert(path.clone(), ts);
                order.push(path);
            }
        }
    }

    order
        .into_iter()
        .map(|path| {
            let at = seen[&path];
            FirstSeen { path, at }
        })
        .collect()
}

fn dir_growth(first_seen: &[FirstSeen]) -> Vec<DirGrowth> {
    let mut acc: HashMap<&str, (usize, i64)> = HashMap::new();
    for f in first_seen {
        let dir = match f.path.split_once('/') {
            Some((head, _)) => head,
            None => "(root)",
        };
        let e = acc.entry(dir).or_insert((0, i64::MAX));
        e.0 += 1;
        e.1 = e.1.min(f.at);
    }
    let mut out: Vec<DirGrowth> = acc
        .into_iter()
        .map(|(dir, (files_added, first_at))| DirGrowth {
            dir: dir.to_string(),
            files_added,
            first_at,
        })
        .collect();
    out.sort_by_key(|d| std::cmp::Reverse(d.files_added));
    out
}

fn run(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

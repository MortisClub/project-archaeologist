mod dupes;
mod git;
mod report;
mod scan;
mod stack;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use report::{human_bytes, Report, Summary};
use scan::FileEntry;

#[derive(Parser)]
#[command(name = "archaeologist", version, about = "Understand any codebase in a minute")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Scan a project directory and build a report
    Scan {
        path: PathBuf,
        /// Directory to write the report into (default: current directory)
        #[arg(long)]
        out: Option<PathBuf>,
        /// Open the HTML report when finished
        #[arg(long)]
        open: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Scan { path, out, open } => scan_project(path, out, open),
    }
}

fn scan_project(path: PathBuf, out: Option<PathBuf>, open: bool) -> Result<()> {
    let root = path
        .canonicalize()
        .with_context(|| format!("cannot open {}", path.display()))?;

    let files = scan::walk(&root)?;
    let summary = summarize(&files);
    let stack = stack::detect(&root, &files);
    let duplicates = dupes::find(&files);
    let git = git::analyze(&root);

    let root_display = root
        .display()
        .to_string()
        .trim_start_matches(r"\\?\")
        .to_string();

    let report = Report {
        root: root_display,
        generated_at: now(),
        summary,
        stack,
        duplicates,
        git,
        files,
    };

    print_summary(&report);

    let out_dir = out.unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&out_dir)?;
    let json_path = out_dir.join("archaeology-report.json");
    let html_path = out_dir.join("archaeology-report.html");
    report.write_json(&json_path)?;
    report.write_html(&html_path)?;

    println!("\n  report: {}", html_path.display());
    println!("  json:   {}", json_path.display());

    if open {
        open_in_browser(&html_path);
    }
    Ok(())
}

fn summarize(files: &[FileEntry]) -> Summary {
    let mut dirs: HashSet<String> = HashSet::new();
    let mut bytes = 0u64;
    for f in files {
        bytes += f.size;
        let mut path = f.path.as_str();
        while let Some(idx) = path.rfind('/') {
            dirs.insert(path[..idx].to_string());
            path = &path[..idx];
        }
    }
    Summary {
        files: files.len(),
        dirs: dirs.len(),
        bytes,
    }
}

fn print_summary(report: &Report) {
    let s = &report.summary;
    println!("\n  {}", report.root);
    println!(
        "  {} files, {} directories, {}",
        s.files,
        s.dirs,
        human_bytes(s.bytes)
    );

    if !report.stack.markers.is_empty() {
        println!("  stack: {}", report.stack.markers.join(", "));
    }
    if !report.stack.frameworks.is_empty() {
        println!("  frameworks: {}", report.stack.frameworks.join(", "));
    }

    let top: Vec<String> = report
        .stack
        .languages
        .iter()
        .take(5)
        .map(|l| format!("{} ({})", l.language, l.files))
        .collect();
    if !top.is_empty() {
        println!("  languages: {}", top.join(", "));
    }

    if !report.duplicates.is_empty() {
        let copies: usize = report.duplicates.iter().map(|g| g.paths.len()).sum();
        println!(
            "  duplicates: {} identical files in {} groups",
            copies,
            report.duplicates.len()
        );
    }

    if let Some(git) = &report.git {
        println!("  git: {} commits", git.commits);
        if let Some(top) = git.hotspots.first() {
            println!("  most-changed: {} ({} changes)", top.path, top.changes);
        }
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn open_in_browser(path: &Path) {
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(path)
        .spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(path).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

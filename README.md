# Project Archaeologist

A local, language-agnostic tool that reads any project folder and tells you what it is:
the stack, the shape of the tree, which files are duplicates, and — when the project is a
git repo — how it grew over time. No server, no cloud, everything runs on your machine.

## Usage

```
archaeologist scan <path>
```

This prints a short summary and writes two files into the current directory:

- `archaeology-report.json` — the full report
- `archaeology-report.html` — a self-contained interactive map you can open in a browser

Pass `--out <dir>` to write them elsewhere, or `--open` to open the HTML report when it's done.

## What it reports

- Stack detection from manifest files (`Cargo.toml`, `package.json`, `go.mod`, `Dockerfile`, …)
  and frameworks pulled from those manifests (React, Django, Axum, …)
- Language breakdown by file count and size
- Duplicate files, matched by content hash
- A directory map sized by bytes, plus the largest files
- Git history: total commits, the age range, the most-changed files, which directories the
  project grew into first, and the oldest files still in the tree

Git history is the honest source of a project's timeline. Without a repo the tool still runs,
but the history section is skipped — filesystem timestamps lie too often (copies, archives,
checkouts) to reconstruct evolution from them.

## Build

```
cargo build --release
```

The binary lands in `target/release/archaeologist`.

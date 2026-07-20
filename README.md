# Project Archaeologist

A local, language-agnostic tool that reads any project folder and tells you what it is:
the stack, the shape of the tree, which files nothing imports, which files matter most,
which dependencies are behind, and — when the project is a git repo — how it grew over
time. No server, no cloud; everything runs on your machine.

## Usage

```
archaeologist scan <path>
```

This prints a short summary and writes two files into the current directory:

- `archaeology-report.json` — the full report
- `archaeology-report.html` — a self-contained interactive map you can open in a browser

Flags:

- `--out <dir>` write the report somewhere other than the current directory
- `--open` open the HTML report when it's done
- `--check-updates` query npm, crates.io and PyPI to flag outdated dependencies (needs network)

## What it reports

**Stack** — detected from manifest files (`Cargo.toml`, `package.json`, `go.mod`,
`Dockerfile`, …) plus frameworks pulled from those manifests (React, Django, Axum, …).

**Languages** — breakdown by file count and size.

**Unused files** — the import graph is walked to find source files that nothing imports.
This is the honest version: entry points (`main`, `index`, `__init__.py`, …) and tests are
excluded, so what's left is genuinely dead. Coverage is JavaScript/TypeScript and Python;
other languages are inventoried but not graphed, and the report says so.

**Most important files** — ranked by a transparent blend of how many files import them,
how often git has touched them, and their size. It's an estimate, not an oracle, and the
report shows the numbers behind each rank.

**Dependencies** — every declared dependency across ecosystems. With `--check-updates` each
one is compared against the registry's latest release and flagged if it's behind.

**Duplicates** — identical files matched by content hash.

**Project map** — directories sized by bytes, plus the largest files.

**Git history** — total commits, the age range, the most-changed files, which directories
the project grew into first, and the oldest files still in the tree.

Git history is the honest source of a project's timeline. Without a repo the tool still
runs, but the history section is skipped — filesystem timestamps lie too often (copies,
archives, checkouts) to reconstruct evolution from them.

## Build

```
cargo build --release
```

The binary lands in `target/release/archaeologist`.

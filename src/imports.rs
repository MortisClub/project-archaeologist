use std::collections::{HashMap, HashSet};

use regex::Regex;
use serde::Serialize;

use crate::scan::FileEntry;

#[derive(Serialize)]
pub struct Insights {
    pub analyzed_languages: Vec<String>,
    pub internal_edges: usize,
    pub dead_files: Vec<String>,
    pub important: Vec<ImportantFile>,
}

#[derive(Serialize)]
pub struct ImportantFile {
    pub path: String,
    pub in_degree: usize,
    pub changes: usize,
    pub score: f64,
}

const JS_EXT: [&str; 6] = ["js", "mjs", "cjs", "jsx", "ts", "tsx"];

pub fn analyze(files: &[FileEntry], churn: &HashMap<String, usize>) -> Insights {
    let paths: HashSet<&str> = files.iter().map(|f| f.path.as_str()).collect();

    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut edges = 0usize;
    let js = JsPatterns::new();

    for f in files {
        let content = match std::fs::read_to_string(&f.abs) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let targets = if JS_EXT.contains(&f.ext.as_str()) {
            js.resolve(&f.path, &content, &paths)
        } else if f.ext == "py" {
            resolve_python(&f.path, &content, &paths)
        } else {
            continue;
        };
        for target in targets {
            if target != f.path {
                *in_degree
                    .entry(paths.get(target.as_str()).copied().unwrap())
                    .or_insert(0) += 1;
                edges += 1;
            }
        }
    }

    let mut analyzed_languages = Vec::new();
    if files.iter().any(|f| JS_EXT.contains(&f.ext.as_str())) {
        analyzed_languages.push("JavaScript/TypeScript".to_string());
    }
    if files.iter().any(|f| f.ext == "py") {
        analyzed_languages.push("Python".to_string());
    }

    let dead_files = dead_files(files, &in_degree);
    let important = importance(files, &in_degree, churn);

    Insights {
        analyzed_languages,
        internal_edges: edges,
        dead_files,
        important,
    }
}

fn dead_files(files: &[FileEntry], in_degree: &HashMap<&str, usize>) -> Vec<String> {
    files
        .iter()
        .filter(|f| JS_EXT.contains(&f.ext.as_str()) || f.ext == "py")
        .filter(|f| in_degree.get(f.path.as_str()).copied().unwrap_or(0) == 0)
        .filter(|f| !is_entry(&f.path) && !is_test(&f.path))
        .map(|f| f.path.clone())
        .collect()
}

fn importance(
    files: &[FileEntry],
    in_degree: &HashMap<&str, usize>,
    churn: &HashMap<String, usize>,
) -> Vec<ImportantFile> {
    let max_deg = in_degree.values().copied().max().unwrap_or(0).max(1) as f64;
    let max_churn = churn.values().copied().max().unwrap_or(0).max(1) as f64;
    let max_size = files.iter().map(|f| f.size).max().unwrap_or(0).max(1) as f64;

    let mut scored: Vec<ImportantFile> = files
        .iter()
        .map(|f| {
            let deg = in_degree.get(f.path.as_str()).copied().unwrap_or(0);
            let changes = churn.get(&f.path).copied().unwrap_or(0);
            let score = 0.5 * (deg as f64 / max_deg)
                + 0.3 * (changes as f64 / max_churn)
                + 0.2 * (f.size as f64 / max_size);
            ImportantFile {
                path: f.path.clone(),
                in_degree: deg,
                changes,
                score,
            }
        })
        .collect();

    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    scored.truncate(20);
    scored
}

fn is_entry(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    let stem = name.split('.').next().unwrap_or(name);
    matches!(stem, "main" | "index" | "app" | "cli" | "server")
        || matches!(
            name,
            "__init__.py" | "__main__.py" | "setup.py" | "conftest.py" | "manage.py"
        )
}

fn is_test(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.contains("/test")
        || lower.contains("/__tests__/")
        || lower.starts_with("test")
        || lower.contains("test_")
        || lower.contains(".test.")
        || lower.contains(".spec.")
}

struct JsPatterns {
    from: Regex,
    bare: Regex,
    require: Regex,
    dynamic: Regex,
}

impl JsPatterns {
    fn new() -> Self {
        JsPatterns {
            from: Regex::new(r#"(?:import|export)\b[^'"]*?\bfrom\s*['"]([^'"]+)['"]"#).unwrap(),
            bare: Regex::new(r#"^\s*import\s*['"]([^'"]+)['"]"#).unwrap(),
            require: Regex::new(r#"\brequire\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap(),
            dynamic: Regex::new(r#"\bimport\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap(),
        }
    }

    fn resolve(&self, from_file: &str, content: &str, paths: &HashSet<&str>) -> Vec<String> {
        let mut out = Vec::new();
        let specs = self
            .from
            .captures_iter(content)
            .chain(self.bare.captures_iter(content))
            .chain(self.require.captures_iter(content))
            .chain(self.dynamic.captures_iter(content))
            .map(|c| c[1].to_string());
        for spec in specs {
            if !spec.starts_with('.') {
                continue;
            }
            let base = join_relative(from_file, &spec);
            if let Some(target) = resolve_js_file(&base, paths) {
                out.push(target);
            }
        }
        out
    }
}

fn join_relative(from_file: &str, spec: &str) -> String {
    let mut parts: Vec<&str> = from_file.split('/').collect();
    parts.pop();
    for seg in spec.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("/")
}

fn resolve_js_file(base: &str, paths: &HashSet<&str>) -> Option<String> {
    if paths.contains(base) {
        return Some(base.to_string());
    }
    for ext in JS_EXT {
        let candidate = format!("{base}.{ext}");
        if paths.contains(candidate.as_str()) {
            return Some(candidate);
        }
    }
    for ext in JS_EXT {
        let candidate = format!("{base}/index.{ext}");
        if paths.contains(candidate.as_str()) {
            return Some(candidate);
        }
    }
    None
}

fn resolve_python(from_file: &str, content: &str, paths: &HashSet<&str>) -> Vec<String> {
    let mut out = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("from ") {
            let Some((module, names)) = rest.split_once(" import ") else {
                continue;
            };
            let module = module.trim();
            if let Some(t) = resolve_py_module(from_file, module, paths) {
                out.push(t);
            }
            for name in names.split(',') {
                let name = name.split_whitespace().next().unwrap_or("");
                if name.is_empty() || name == "*" {
                    continue;
                }
                let submodule = format!("{}.{}", module.trim_end_matches('.'), name);
                if let Some(t) = resolve_py_module(from_file, &submodule, paths) {
                    out.push(t);
                }
            }
        } else if let Some(rest) = line.strip_prefix("import ") {
            for part in rest.split(',') {
                let module = part.split_whitespace().next().unwrap_or("");
                if let Some(t) = resolve_py_module(from_file, module, paths) {
                    out.push(t);
                }
            }
        }
    }
    out
}

fn resolve_py_module(from_file: &str, module: &str, paths: &HashSet<&str>) -> Option<String> {
    let dots = module.chars().take_while(|c| *c == '.').count();
    let rest = &module[dots..];

    let mut parts: Vec<String> = Vec::new();
    if dots > 0 {
        let mut dir: Vec<&str> = from_file.split('/').collect();
        dir.pop();
        for _ in 1..dots {
            dir.pop();
        }
        parts.extend(dir.iter().map(|s| s.to_string()));
    }
    parts.extend(
        rest.split('.')
            .filter(|s| !s.is_empty())
            .map(str::to_string),
    );
    if parts.is_empty() {
        return None;
    }

    let joined = parts.join("/");
    [format!("{joined}.py"), format!("{joined}/__init__.py")]
        .into_iter()
        .find(|candidate| paths.contains(candidate.as_str()))
}

#[cfg(test)]
mod tests {
    use super::join_relative;

    #[test]
    fn resolves_relative_paths() {
        assert_eq!(
            join_relative("src/index.js", "./utils/greet"),
            "src/utils/greet"
        );
        assert_eq!(join_relative("a/b/c.js", "../d"), "a/d");
        assert_eq!(join_relative("main.py", "./pkg/mod"), "pkg/mod");
    }
}

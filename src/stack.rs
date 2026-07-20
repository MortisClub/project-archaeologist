use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::scan::FileEntry;

#[derive(Serialize)]
pub struct Stack {
    pub languages: Vec<LangStat>,
    pub markers: Vec<String>,
    pub frameworks: Vec<String>,
}

#[derive(Serialize)]
pub struct LangStat {
    pub language: String,
    pub files: usize,
    pub bytes: u64,
}

pub fn detect(root: &Path, files: &[FileEntry]) -> Stack {
    Stack {
        languages: languages(files),
        markers: markers(files),
        frameworks: frameworks(root),
    }
}

fn languages(files: &[FileEntry]) -> Vec<LangStat> {
    let mut acc: BTreeMap<&'static str, (usize, u64)> = BTreeMap::new();
    for f in files {
        if let Some(lang) = lang_for_ext(&f.ext) {
            let e = acc.entry(lang).or_default();
            e.0 += 1;
            e.1 += f.size;
        }
    }
    let mut out: Vec<LangStat> = acc
        .into_iter()
        .map(|(language, (files, bytes))| LangStat {
            language: language.to_string(),
            files,
            bytes,
        })
        .collect();
    out.sort_by_key(|l| std::cmp::Reverse(l.bytes));
    out
}

fn markers(files: &[FileEntry]) -> Vec<String> {
    let mut found = Vec::new();
    for f in files {
        let name = f.path.rsplit('/').next().unwrap_or(&f.path);
        let label = match name {
            "Cargo.toml" => "Cargo (Rust)",
            "package.json" => "npm (Node.js)",
            "pyproject.toml" | "requirements.txt" | "setup.py" => "Python",
            "go.mod" => "Go modules",
            "pom.xml" => "Maven (Java)",
            "build.gradle" | "build.gradle.kts" => "Gradle",
            "Gemfile" => "Bundler (Ruby)",
            "composer.json" => "Composer (PHP)",
            "Dockerfile" => "Docker",
            "docker-compose.yml" | "docker-compose.yaml" => "Docker Compose",
            _ => continue,
        };
        if !found.iter().any(|m| m == label) {
            found.push(label.to_string());
        }
    }
    found
}

fn frameworks(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    if let Ok(text) = std::fs::read_to_string(root.join("package.json")) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
            let mut deps = Vec::new();
            for key in ["dependencies", "devDependencies"] {
                if let Some(obj) = json.get(key).and_then(|v| v.as_object()) {
                    deps.extend(obj.keys().cloned());
                }
            }
            for (dep, label) in JS_FRAMEWORKS {
                if deps.iter().any(|d| d == dep) {
                    found.push(label.to_string());
                }
            }
        }
    }
    if let Ok(text) = std::fs::read_to_string(root.join("Cargo.toml")) {
        for (dep, label) in RUST_FRAMEWORKS {
            if text.contains(dep) {
                found.push(label.to_string());
            }
        }
    }
    for manifest in ["requirements.txt", "pyproject.toml"] {
        if let Ok(text) = std::fs::read_to_string(root.join(manifest)) {
            let lower = text.to_lowercase();
            for (dep, label) in PY_FRAMEWORKS {
                if lower.contains(dep) && !found.iter().any(|f| f == label) {
                    found.push(label.to_string());
                }
            }
        }
    }
    found
}

const JS_FRAMEWORKS: &[(&str, &str)] = &[
    ("react", "React"),
    ("vue", "Vue"),
    ("@angular/core", "Angular"),
    ("svelte", "Svelte"),
    ("next", "Next.js"),
    ("nuxt", "Nuxt"),
    ("express", "Express"),
    ("@nestjs/core", "NestJS"),
    ("electron", "Electron"),
];

const RUST_FRAMEWORKS: &[(&str, &str)] = &[
    ("actix-web", "Actix Web"),
    ("axum", "Axum"),
    ("rocket", "Rocket"),
    ("bevy", "Bevy"),
    ("tauri", "Tauri"),
];

const PY_FRAMEWORKS: &[(&str, &str)] = &[
    ("django", "Django"),
    ("flask", "Flask"),
    ("fastapi", "FastAPI"),
];

fn lang_for_ext(ext: &str) -> Option<&'static str> {
    let lang = match ext {
        "rs" => "Rust",
        "js" | "mjs" | "cjs" => "JavaScript",
        "ts" => "TypeScript",
        "tsx" | "jsx" => "React",
        "py" => "Python",
        "go" => "Go",
        "java" => "Java",
        "kt" | "kts" => "Kotlin",
        "rb" => "Ruby",
        "php" => "PHP",
        "c" | "h" => "C",
        "cpp" | "cc" | "cxx" | "hpp" => "C++",
        "cs" => "C#",
        "swift" => "Swift",
        "sh" | "bash" => "Shell",
        "html" | "htm" => "HTML",
        "css" | "scss" | "sass" | "less" => "CSS",
        "vue" => "Vue",
        "svelte" => "Svelte",
        "json" => "JSON",
        "yml" | "yaml" => "YAML",
        "toml" => "TOML",
        "md" | "markdown" => "Markdown",
        "sql" => "SQL",
        _ => return None,
    };
    Some(lang)
}

use std::path::Path;
use std::time::Duration;

use serde::Serialize;

#[derive(Serialize)]
pub struct Dependency {
    pub ecosystem: String,
    pub name: String,
    pub required: String,
    pub latest: Option<String>,
    pub outdated: bool,
}

pub fn collect(root: &Path, check_updates: bool) -> Vec<Dependency> {
    let mut deps = Vec::new();
    deps.extend(from_cargo(root));
    deps.extend(from_package_json(root));
    deps.extend(from_requirements(root));
    deps.extend(from_pyproject(root));

    deps.sort_by(|a, b| a.ecosystem.cmp(&b.ecosystem).then(a.name.cmp(&b.name)));
    deps.dedup_by(|a, b| a.ecosystem == b.ecosystem && a.name == b.name);

    if check_updates {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(6))
            .build();
        for dep in &mut deps {
            if let Some(latest) = latest_version(&agent, dep) {
                dep.outdated = is_newer(&latest, &dep.required);
                dep.latest = Some(latest);
            }
        }
    }
    deps
}

fn from_cargo(root: &Path) -> Vec<Dependency> {
    let text = match std::fs::read_to_string(root.join("Cargo.toml")) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    let value: toml::Value = match text.parse() {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut out = Vec::new();
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        let table = match value.get(section).and_then(|v| v.as_table()) {
            Some(t) => t,
            None => continue,
        };
        for (name, spec) in table {
            let required = match spec {
                toml::Value::String(s) => s.clone(),
                toml::Value::Table(t) => t
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("*")
                    .to_string(),
                _ => "*".to_string(),
            };
            out.push(dep("cargo", name, &required));
        }
    }
    out
}

fn from_package_json(root: &Path) -> Vec<Dependency> {
    let text = match std::fs::read_to_string(root.join("package.json")) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    let json: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut out = Vec::new();
    for section in ["dependencies", "devDependencies"] {
        if let Some(obj) = json.get(section).and_then(|v| v.as_object()) {
            for (name, spec) in obj {
                let required = spec.as_str().unwrap_or("*").to_string();
                out.push(dep("npm", name, &required));
            }
        }
    }
    out
}

fn from_requirements(root: &Path) -> Vec<Dependency> {
    let text = match std::fs::read_to_string(root.join("requirements.txt")) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with('-'))
        .filter_map(|line| {
            let end = line
                .find(|c: char| "=<>!~ ".contains(c))
                .unwrap_or(line.len());
            let name = line[..end].trim();
            if name.is_empty() {
                return None;
            }
            let required = line[end..].trim().trim_start_matches("==").trim();
            Some(dep("pypi", name, required))
        })
        .collect()
}

fn from_pyproject(root: &Path) -> Vec<Dependency> {
    let text = match std::fs::read_to_string(root.join("pyproject.toml")) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    let value: toml::Value = match text.parse() {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut out = Vec::new();
    if let Some(arr) = value
        .get("project")
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_array())
    {
        for item in arr.iter().filter_map(|v| v.as_str()) {
            let end = item
                .find(|c: char| "=<>!~ ".contains(c))
                .unwrap_or(item.len());
            let name = item[..end].trim();
            if !name.is_empty() {
                out.push(dep("pypi", name, item[end..].trim()));
            }
        }
    }
    if let Some(table) = value
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_table())
    {
        for (name, spec) in table {
            if name == "python" {
                continue;
            }
            let required = spec.as_str().unwrap_or("*").to_string();
            out.push(dep("pypi", name, &required));
        }
    }
    out
}

fn dep(ecosystem: &str, name: &str, required: &str) -> Dependency {
    Dependency {
        ecosystem: ecosystem.to_string(),
        name: name.to_string(),
        required: if required.is_empty() {
            "*".to_string()
        } else {
            required.to_string()
        },
        latest: None,
        outdated: false,
    }
}

fn latest_version(agent: &ureq::Agent, dep: &Dependency) -> Option<String> {
    let (url, pointer) = match dep.ecosystem.as_str() {
        "npm" => (
            format!("https://registry.npmjs.org/{}/latest", dep.name),
            "/version",
        ),
        "cargo" => (
            format!("https://crates.io/api/v1/crates/{}", dep.name),
            "/crate/max_stable_version",
        ),
        "pypi" => (
            format!("https://pypi.org/pypi/{}/json", dep.name),
            "/info/version",
        ),
        _ => return None,
    };

    let body: serde_json::Value = agent
        .get(&url)
        .set("User-Agent", "project-archaeologist")
        .call()
        .ok()?
        .into_json()
        .ok()?;
    body.pointer(pointer)
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

fn is_newer(latest: &str, required: &str) -> bool {
    let a = numeric_parts(latest);
    let b = numeric_parts(required);
    if a.is_empty() || b.is_empty() {
        return false;
    }
    a > b
}

fn numeric_parts(version: &str) -> Vec<u64> {
    let trimmed = version.trim_start_matches(|c: char| !c.is_ascii_digit());
    let end = trimmed
        .find(|c: char| c != '.' && !c.is_ascii_digit())
        .unwrap_or(trimmed.len());
    trimmed[..end]
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{is_newer, numeric_parts};

    #[test]
    fn detects_newer_release() {
        assert!(is_newer("1.5.0", "1.2.0"));
        assert!(is_newer("19.2.7", "^17.0.0"));
        assert!(is_newer("3.3.0", "2"));
    }

    #[test]
    fn same_or_older_is_not_newer() {
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("1.2.0", "1.5.0"));
    }

    #[test]
    fn parses_leading_numeric() {
        assert_eq!(numeric_parts("^1.2.3"), vec![1, 2, 3]);
        assert_eq!(numeric_parts("0.8"), vec![0, 8]);
        assert!(numeric_parts("*").is_empty());
    }
}

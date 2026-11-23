//! precommitguard: a fast, offline supply-chain and hygiene linter for
//! `.pre-commit-config.yaml` and `.pre-commit-hooks.yaml`.

pub mod config;
pub mod data;
pub mod output;
pub mod rules;
pub mod yaml;

use serde::Serialize;
use std::path::{Path, PathBuf};

pub use config::Config;

/// Finding severity, ordered from least to most severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

impl std::str::FromStr for Severity {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "info" | "note" => Ok(Severity::Info),
            "warning" | "warn" => Ok(Severity::Warning),
            "error" => Ok(Severity::Error),
            "never" | "none" => Err("never".into()),
            o => Err(format!(
                "unknown severity '{o}' (expected info, warning, error)"
            )),
        }
    }
}

/// What kind of file is being linted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `.pre-commit-config.yaml`: the consumer config.
    Config,
    /// `.pre-commit-hooks.yaml`: the manifest of a hook repository.
    Manifest,
}

impl Kind {
    /// Infer the kind from a file name.
    pub fn for_path(path: &Path) -> Kind {
        match path.file_name().and_then(|n| n.to_str()) {
            Some(n) if n.contains("pre-commit-hooks") => Kind::Manifest,
            _ => Kind::Config,
        }
    }
}

/// A single lint finding.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Finding {
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
    pub line: usize,
    pub column: usize,
}

/// Findings for one file.
#[derive(Debug, Clone, Serialize)]
pub struct FileReport {
    pub path: String,
    pub findings: Vec<Finding>,
}

const MARKER: &str = "precommitguard:";

/// What an inline `# precommitguard: ...` comment asks for.
#[derive(Debug, PartialEq)]
enum Directive {
    IgnoreFile,
    /// Rule IDs to ignore; `ALL` means every rule.
    Ignore(Vec<String>),
}

fn parse_directive(text: &str) -> Option<Directive> {
    let idx = text.find(MARKER)?;
    let rest = text[idx + MARKER.len()..].trim_start();
    if rest.starts_with("ignore-file") {
        return Some(Directive::IgnoreFile);
    }
    let rest = rest.strip_prefix("ignore")?;
    let ids: Vec<String> = rest
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_uppercase())
        .take_while(|s| s.starts_with("PCG"))
        .collect();
    Some(Directive::Ignore(if ids.is_empty() {
        vec!["ALL".into()]
    } else {
        ids
    }))
}

/// Collect (start_line, end_line) of every node and mapping entry. Sequence
/// nodes are skipped: an ignore comment above `- repo: ...` targets that item,
/// not the whole `repos:` list that starts on the same line.
fn spans(node: &yaml::Node, out: &mut Vec<(usize, usize)>) {
    if !matches!(node.value, yaml::Value::Seq(_)) {
        out.push((node.line, node.end_line));
    }
    match &node.value {
        yaml::Value::Seq(items) => items.iter().for_each(|n| spans(n, out)),
        yaml::Value::Map(pairs) => {
            for (k, v) in pairs {
                out.push((k.line, v.end_line.max(k.line)));
                spans(v, out);
            }
        }
        _ => {}
    }
}

/// Lint file contents.
pub fn lint_str(src: &str, kind: Kind, cfg: &Config) -> Vec<Finding> {
    let comments = yaml::comments(src);
    let directives: Vec<(&yaml::Comment, Directive)> = comments
        .iter()
        .filter_map(|c| parse_directive(&c.text).map(|d| (c, d)))
        .collect();
    if directives.iter().any(|(_, d)| *d == Directive::IgnoreFile) {
        return vec![];
    }

    let (raw, root) = match yaml::parse(src) {
        Ok(root) => (rules::check(root.as_ref(), kind, cfg, &comments), root),
        Err(e) => (vec![rules::parse_error(&e)], None),
    };

    // Build (line range, ids) suppressions from inline comments.
    let lines: Vec<&str> = src.lines().collect();
    let mut all_spans = Vec::new();
    if let Some(r) = &root {
        spans(r, &mut all_spans);
    }
    let mut suppress: Vec<(usize, usize, Vec<String>)> = Vec::new();
    for (c, d) in directives {
        let Directive::Ignore(ids) = d else { continue };
        if c.own_line {
            let next = (c.line..lines.len()).map(|i| i + 1).find(|&ln| {
                let t = lines[ln - 1].trim();
                !(t.is_empty() || t.starts_with('#'))
            });
            if let Some(ln) = next {
                let end = all_spans
                    .iter()
                    .filter(|(s, _)| *s == ln)
                    .map(|(_, e)| *e)
                    .max()
                    .unwrap_or(ln);
                suppress.push((ln, end, ids));
            }
        } else {
            suppress.push((c.line, c.line, ids));
        }
    }

    let mut out = Vec::new();
    for mut f in raw {
        if !cfg.enabled(f.rule) {
            continue;
        }
        let ignored = suppress.iter().any(|(s, e, ids)| {
            (*s..=*e).contains(&f.line) && ids.iter().any(|i| i == f.rule || i == "ALL")
        });
        if ignored {
            continue;
        }
        if let Some(sev) = cfg.severity_override(f.rule) {
            f.severity = sev;
        }
        out.push(f);
    }
    out.sort_by(|a, b| (a.line, a.column, a.rule).cmp(&(b.line, b.column, b.rule)));
    out.dedup();
    out
}

/// File names discovered when a directory is given.
pub fn is_target(name: &str) -> bool {
    matches!(
        name,
        ".pre-commit-config.yaml" | ".pre-commit-config.yml" | ".pre-commit-hooks.yaml"
    )
}

const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "vendor",
    ".venv",
    "venv",
    ".tox",
];

/// Expand paths: files are kept as-is, directories are searched recursively
/// for `.pre-commit-config.yaml` and `.pre-commit-hooks.yaml`.
pub fn discover(paths: &[PathBuf]) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for p in paths {
        if p.is_dir() {
            walk(p, &mut out)?;
        } else {
            out.push(p.clone());
        }
    }
    Ok(out)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries = std::fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let path = e.path();
        let name = e.file_name();
        let name = name.to_string_lossy();
        let ft = e.file_type()?;
        if ft.is_dir() {
            if !SKIP_DIRS.contains(&name.as_ref()) {
                walk(&path, out)?;
            }
        } else if is_target(&name) {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directive_parsing() {
        assert_eq!(
            parse_directive(" precommitguard: ignore PCG001, pcg002 because reasons"),
            Some(Directive::Ignore(vec!["PCG001".into(), "PCG002".into()]))
        );
        assert_eq!(
            parse_directive(" precommitguard: ignore"),
            Some(Directive::Ignore(vec!["ALL".into()]))
        );
        assert_eq!(
            parse_directive(" precommitguard: ignore-file"),
            Some(Directive::IgnoreFile)
        );
        assert_eq!(parse_directive(" frozen: v1.0.0"), None);
    }

    #[test]
    fn kind_detection() {
        assert_eq!(
            Kind::for_path(Path::new("x/.pre-commit-hooks.yaml")),
            Kind::Manifest
        );
        assert_eq!(
            Kind::for_path(Path::new(".pre-commit-config.yaml")),
            Kind::Config
        );
    }
}

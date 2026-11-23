//! `.precommitguard.yaml` configuration.

use crate::Severity;
use crate::yaml::{self, Node};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::Path;

/// Default config file name searched in the working directory.
pub const DEFAULT_CONFIG: &str = ".precommitguard.yaml";

/// Hosts trusted by default for `repo:` URLs (PCG004).
pub const DEFAULT_ALLOWED_HOSTS: &[&str] =
    &["github.com", "gitlab.com", "bitbucket.org", "codeberg.org"];

/// Effective configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Rule IDs to disable entirely.
    pub ignore: Vec<String>,
    /// Lowest severity that makes the process exit non-zero (default: warning).
    pub fail_on: Option<String>,
    /// Per-rule severity overrides.
    pub severity: Vec<(String, Severity)>,
    /// Glob patterns of files to skip.
    pub ignore_paths: Vec<String>,
    /// Hosts allowed in `repo:` URLs (PCG004). Replaces the defaults when set.
    pub allowed_hosts: Vec<String>,
    /// Repo URL patterns (`*` wildcard) for which tag revs are accepted (PCG001).
    pub allow_tags: Vec<String>,
    /// Flag prek-only extension keys (PCG013) for strict upstream pre-commit portability.
    pub strict_upstream: bool,
    globs: Option<GlobSet>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ignore: vec![],
            fail_on: None,
            severity: vec![],
            ignore_paths: vec![],
            allowed_hosts: DEFAULT_ALLOWED_HOSTS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            allow_tags: vec![],
            strict_upstream: false,
            globs: None,
        }
    }
}

fn strings(node: &Node, key: &str) -> Result<Vec<String>, String> {
    match &node.value {
        yaml::Value::Seq(items) => items
            .iter()
            .map(|n| {
                n.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("line {}: '{key}' items must be strings", n.line))
            })
            .collect(),
        _ if node.is_null() => Ok(vec![]),
        _ => Err(format!("line {}: '{key}' must be a list", node.line)),
    }
}

fn check_rule(id: &str, line: usize) -> Result<(), String> {
    if crate::rules::rule(id).is_none() {
        return Err(format!("line {line}: unknown rule id '{id}'"));
    }
    Ok(())
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let mut cfg = Config::default();
        let root = yaml::parse(text).map_err(|e| format!("line {}: {}", e.line, e.message))?;
        let Some(root) = root else {
            return Ok(cfg);
        };
        let map = root.as_map().ok_or("config must be a mapping")?;
        let mut paths = vec![];
        for (k, v) in map {
            let key = k.as_str().unwrap_or_default();
            match key {
                "ignore" => {
                    for id in strings(v, key)? {
                        check_rule(&id, v.line)?;
                        cfg.ignore.push(id);
                    }
                }
                "fail_on" => {
                    let s = v.as_str().ok_or("'fail_on' must be a string")?;
                    if s != "never" {
                        s.parse::<Severity>()?;
                    }
                    cfg.fail_on = Some(s.to_string());
                }
                "severity" => {
                    for (rk, rv) in v.as_map().ok_or("'severity' must be a mapping")? {
                        let id = rk.as_str().unwrap_or_default().to_string();
                        check_rule(&id, rk.line)?;
                        let sev = rv
                            .as_str()
                            .unwrap_or_default()
                            .parse::<Severity>()
                            .map_err(|e| format!("line {}: {e}", rv.line))?;
                        cfg.severity.push((id, sev));
                    }
                }
                "ignore_paths" => paths = strings(v, key)?,
                "allowed_hosts" => cfg.allowed_hosts = strings(v, key)?,
                "allow_tags" => cfg.allow_tags = strings(v, key)?,
                "strict_upstream" => {
                    cfg.strict_upstream = v
                        .as_bool()
                        .ok_or("'strict_upstream' must be true or false")?
                }
                other => return Err(format!("line {}: unknown config key '{other}'", k.line)),
            }
        }
        cfg.add_ignore_paths(paths)?;
        Ok(cfg)
    }

    /// Add glob patterns (e.g. `examples/**`) of files to skip.
    pub fn add_ignore_paths(&mut self, patterns: Vec<String>) -> Result<(), String> {
        self.ignore_paths.extend(patterns);
        let mut b = GlobSetBuilder::new();
        for p in &self.ignore_paths {
            b.add(Glob::new(p).map_err(|e| format!("invalid ignore_paths glob '{p}': {e}"))?);
        }
        self.globs = Some(b.build().map_err(|e| e.to_string())?);
        Ok(())
    }

    pub fn enabled(&self, id: &str) -> bool {
        !self.ignore.iter().any(|i| i.eq_ignore_ascii_case(id))
    }

    pub fn severity_override(&self, id: &str) -> Option<Severity> {
        self.severity
            .iter()
            .rev()
            .find(|(k, _)| k.eq_ignore_ascii_case(id))
            .map(|(_, v)| *v)
    }

    /// True if `path` matches an `ignore_paths` glob.
    pub fn path_ignored(&self, path: &str) -> bool {
        let p = path.replace('\\', "/");
        let p = p.trim_start_matches("./");
        self.globs.as_ref().is_some_and(|g| g.is_match(p))
    }

    /// True if `host` (lowercase) is an allowed host or a subdomain of one.
    pub fn host_allowed(&self, host: &str) -> bool {
        self.allowed_hosts.iter().any(|a| {
            let a = a.to_ascii_lowercase();
            host == a || host.ends_with(&format!(".{a}"))
        })
    }

    /// True if tag revs are accepted for this repo URL.
    pub fn tags_allowed(&self, repo: &str) -> bool {
        self.allow_tags.iter().any(|p| wildcard(p, repo))
    }
}

/// Minimal `*` wildcard matching (case-insensitive).
fn wildcard(pattern: &str, text: &str) -> bool {
    let p = pattern.to_ascii_lowercase();
    let t = text.to_ascii_lowercase();
    let parts: Vec<&str> = p.split('*').collect();
    if parts.len() == 1 {
        return p == t;
    }
    let mut rest = t.as_str();
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            match rest.strip_prefix(part) {
                Some(r) => rest = r,
                None => return false,
            }
        } else if i == parts.len() - 1 {
            return rest.ends_with(part);
        } else {
            match rest.find(part) {
                Some(idx) => rest = &rest[idx + part.len()..],
                None => return false,
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_config() {
        let c = Config::parse(
            "ignore: [PCG005]\nfail_on: error\nseverity:\n  PCG001: error\nignore_paths:\n  - 'examples/**'\nallowed_hosts: [git.example.com]\nallow_tags:\n  - https://github.com/pre-commit/*\nstrict_upstream: true\n",
        )
        .unwrap();
        assert!(!c.enabled("pcg005"));
        assert_eq!(c.severity_override("PCG001"), Some(Severity::Error));
        assert!(c.path_ignored("./examples/a/.pre-commit-config.yaml"));
        assert!(!c.path_ignored(".pre-commit-config.yaml"));
        assert!(c.host_allowed("git.example.com"));
        assert!(!c.host_allowed("github.com"));
        assert!(c.tags_allowed("https://github.com/pre-commit/pre-commit-hooks"));
        assert!(!c.tags_allowed("https://github.com/psf/black"));
        assert!(c.strict_upstream);
    }

    #[test]
    fn rejects_unknown_rules_and_keys() {
        assert!(Config::parse("ignore: [PCG999]").is_err());
        assert!(Config::parse("bogus: 1").is_err());
        assert!(Config::parse("fail_on: loud").is_err());
        let d = Config::parse("").unwrap();
        assert!(d.enabled("PCG001"));
        assert!(d.host_allowed("github.com"));
        assert!(d.host_allowed("gist.github.com"));
        assert!(!d.host_allowed("evilgithub.com"));
    }

    #[test]
    fn wildcard_matching() {
        assert!(wildcard("*", "anything"));
        assert!(wildcard(
            "https://github.com/*/hooks",
            "https://github.com/x/hooks"
        ));
        assert!(!wildcard(
            "https://github.com/*/hooks",
            "https://github.com/x/other"
        ));
    }
}

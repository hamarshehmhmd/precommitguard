//! Curated data: the pre-commit schema (mirrors `pre_commit/clientlib.py`,
//! pre-commit 4.7) and a list of archived or moved hook repositories.

/// Top-level keys of `.pre-commit-config.yaml`.
pub const TOP_KEYS: &[&str] = &[
    "repos",
    "default_install_hook_types",
    "default_language_version",
    "default_stages",
    "files",
    "exclude",
    "fail_fast",
    "minimum_pre_commit_version",
    "ci",
];

/// Top-level keys implemented only by prek (https://github.com/j178/prek).
pub const PREK_TOP_KEYS: &[&str] = &[
    "update",
    "default_env",
    "priorities",
    "minimum_prek_version",
    "orphan",
];

/// Hook keys implemented only by prek.
pub const PREK_HOOK_KEYS: &[&str] = &["env", "shell", "priority", "minimum_prek_version", "groups"];

/// Keys of a `repos:` entry.
pub const REPO_KEYS: &[&str] = &["repo", "rev", "hooks"];

/// Keys of a hook (manifest keys; config hooks may override any of them).
pub const HOOK_KEYS: &[&str] = &[
    "id",
    "name",
    "entry",
    "language",
    "alias",
    "files",
    "exclude",
    "types",
    "types_or",
    "exclude_types",
    "additional_dependencies",
    "args",
    "always_run",
    "fail_fast",
    "pass_filenames",
    "description",
    "language_version",
    "log_file",
    "require_serial",
    "stages",
    "verbose",
    "minimum_pre_commit_version",
];

/// Git hook types pre-commit can install.
pub const HOOK_TYPES: &[&str] = &[
    "commit-msg",
    "post-checkout",
    "post-commit",
    "post-merge",
    "post-rewrite",
    "pre-commit",
    "pre-merge-commit",
    "pre-push",
    "pre-rebase",
    "prepare-commit-msg",
];

/// Deprecated stage names and their replacements (deprecated in pre-commit 3.2.0).
pub const DEPRECATED_STAGES: &[(&str, &str)] = &[
    ("commit", "pre-commit"),
    ("merge-commit", "pre-merge-commit"),
    ("push", "pre-push"),
];

/// Languages supported by pre-commit 4.x.
pub const LANGUAGES: &[&str] = &[
    "conda",
    "coursier",
    "dart",
    "docker",
    "docker_image",
    "dotnet",
    "fail",
    "golang",
    "haskell",
    "julia",
    "lua",
    "node",
    "perl",
    "pygrep",
    "python",
    "r",
    "ruby",
    "rust",
    "swift",
    "unsupported",
    "unsupported_script",
];

/// Legacy language names still accepted as aliases (pre-commit 4.4.0+ prefers the new name).
pub const LEGACY_LANGUAGES: &[(&str, &str)] =
    &[("system", "unsupported"), ("script", "unsupported_script")];

/// Languages removed from pre-commit: (name, replacement, removed in).
pub const REMOVED_LANGUAGES: &[(&str, &str, &str)] = &[
    ("python_venv", "python", "4.0.0"),
    ("pcre", "pygrep", "2.0.0"),
];

/// Valid `repo: meta` hook ids.
pub const META_HOOKS: &[&str] = &["check-hooks-apply", "check-useless-excludes", "identity"];

/// Revs that are always branch names / moving refs.
pub const BRANCH_NAMES: &[&str] = &[
    "main", "master", "head", "develop", "dev", "trunk", "latest", "stable", "nightly", "next",
];

/// Why a repository is flagged by PCG011.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    /// The repository is archived (read-only, no further updates).
    Archived,
    /// The repository moved; the old URL only works through a redirect.
    Moved,
}

/// A known archived or moved hook repository.
#[derive(Debug, Clone, Copy)]
pub struct MovedRepo {
    /// Lowercase `host/owner/name` of the old location.
    pub from: &'static str,
    pub kind: MoveKind,
    /// Where to go instead.
    pub to: &'static str,
}

/// Curated list, verified against the GitHub API on 2026-11-23.
pub const MOVED_REPOS: &[MovedRepo] = &[
    MovedRepo {
        from: "github.com/pre-commit/mirrors-prettier",
        kind: MoveKind::Archived,
        to: "a `repo: local` hook running prettier via `language: node`, or https://github.com/rbubley/mirrors-prettier",
    },
    MovedRepo {
        from: "github.com/pre-commit/mirrors-isort",
        kind: MoveKind::Archived,
        to: "https://github.com/PyCQA/isort",
    },
    MovedRepo {
        from: "github.com/pre-commit/mirrors-autopep8",
        kind: MoveKind::Archived,
        to: "https://github.com/hhatto/autopep8",
    },
    MovedRepo {
        from: "github.com/pre-commit/mirrors-yapf",
        kind: MoveKind::Archived,
        to: "https://github.com/google/yapf",
    },
    MovedRepo {
        from: "github.com/pre-commit/mirrors-pylint",
        kind: MoveKind::Archived,
        to: "https://github.com/pylint-dev/pylint (or a `repo: local` hook)",
    },
    MovedRepo {
        from: "github.com/pre-commit/mirrors-ruby-lint",
        kind: MoveKind::Archived,
        to: "a maintained Ruby linter such as rubocop",
    },
    MovedRepo {
        from: "github.com/pre-commit/mirrors-coffeelint",
        kind: MoveKind::Archived,
        to: "a `repo: local` hook",
    },
    MovedRepo {
        from: "github.com/asottile/seed-isort-config",
        kind: MoveKind::Archived,
        to: "isort's own `known_first_party` detection (https://github.com/PyCQA/isort)",
    },
    MovedRepo {
        from: "github.com/timothycrosley/isort",
        kind: MoveKind::Moved,
        to: "https://github.com/PyCQA/isort",
    },
    MovedRepo {
        from: "github.com/ambv/black",
        kind: MoveKind::Moved,
        to: "https://github.com/psf/black-pre-commit-mirror",
    },
    MovedRepo {
        from: "github.com/charliermarsh/ruff-pre-commit",
        kind: MoveKind::Moved,
        to: "https://github.com/astral-sh/ruff-pre-commit",
    },
    MovedRepo {
        from: "github.com/myint/autoflake",
        kind: MoveKind::Moved,
        to: "https://github.com/PyCQA/autoflake",
    },
    MovedRepo {
        from: "github.com/myint/docformatter",
        kind: MoveKind::Moved,
        to: "https://github.com/PyCQA/docformatter",
    },
    MovedRepo {
        from: "github.com/asottile/blacken-docs",
        kind: MoveKind::Moved,
        to: "https://github.com/adamchainz/blacken-docs",
    },
    MovedRepo {
        from: "gitlab.com/pycqa/flake8",
        kind: MoveKind::Moved,
        to: "https://github.com/PyCQA/flake8",
    },
];

/// Look up a repo URL in [`MOVED_REPOS`].
pub fn moved(url: &str) -> Option<&'static MovedRepo> {
    let key = normalize_url(url);
    MOVED_REPOS.iter().find(|m| key == m.from)
}

/// Normalize a git URL to lowercase `host/owner/name` (scheme, credentials,
/// `.git` suffix and trailing slashes removed). SCP-style `git@host:o/r` is supported.
pub fn normalize_url(url: &str) -> String {
    let mut u = url.trim().to_ascii_lowercase();
    if let Some(idx) = u.find("://") {
        u = u[idx + 3..].to_string();
    } else if let Some(rest) = u.strip_prefix("git@") {
        u = rest.replacen(':', "/", 1);
    }
    if let Some(at) = u.find('@')
        && at < u.find('/').unwrap_or(usize::MAX)
    {
        u = u[at + 1..].to_string();
    }
    let u = u.trim_end_matches('/');
    let u = u.strip_suffix(".git").unwrap_or(u);
    u.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_urls() {
        assert_eq!(
            normalize_url("https://github.com/PSF/Black.git/"),
            "github.com/psf/black"
        );
        assert_eq!(
            normalize_url("git@github.com:psf/black.git"),
            "github.com/psf/black"
        );
        assert_eq!(
            normalize_url("https://user:tok@gitlab.com/pycqa/flake8"),
            "gitlab.com/pycqa/flake8"
        );
    }

    #[test]
    fn finds_moved_repos() {
        assert!(moved("https://github.com/pre-commit/mirrors-prettier").is_some());
        assert!(moved("https://github.com/pre-commit/mirrors-mypy").is_none());
    }
}

//! Rule catalogue and checks.

use crate::config::Config;
use crate::data::{self, MoveKind};
use crate::yaml::{Comment, Node, ParseError, Value};
use crate::{Finding, Kind, Severity};
use std::collections::HashMap;
use std::sync::LazyLock;

/// Static metadata for a rule.
#[derive(Debug, Clone, Copy)]
pub struct Rule {
    pub id: &'static str,
    pub name: &'static str,
    pub severity: Severity,
    pub category: &'static str,
    pub description: &'static str,
    pub fix: &'static str,
}

macro_rules! rule {
    ($id:literal, $name:literal, $sev:ident, $cat:literal, $desc:literal, $fix:literal) => {
        Rule {
            id: $id,
            name: $name,
            severity: Severity::$sev,
            category: $cat,
            description: $desc,
            fix: $fix,
        }
    };
}

/// Every rule, in ID order.
pub const RULES: &[Rule] = &[
    rule!(
        "PCG001",
        "mutable-tag-rev",
        Warning,
        "supply-chain",
        "`rev` is a tag (or abbreviated SHA) rather than a full 40-character commit SHA; tags can be moved or re-pushed",
        "Pin to the full commit SHA with `pre-commit autoupdate --freeze`, which also writes a `# frozen: vX.Y.Z` comment"
    ),
    rule!(
        "PCG002",
        "branch-rev",
        Error,
        "supply-chain",
        "`rev` is a branch-like moving reference (main, master, HEAD, ...); pre-commit never updates it after first install and anyone who can push to the branch controls your hook",
        "Run `pre-commit autoupdate --freeze` to pin a release commit"
    ),
    rule!(
        "PCG003",
        "insecure-repo-url",
        Error,
        "supply-chain",
        "`repo` uses an unauthenticated transport (`http://` or `git://`), so hook code can be tampered with in transit",
        "Use the `https://` URL of the repository"
    ),
    rule!(
        "PCG004",
        "untrusted-repo-host",
        Warning,
        "supply-chain",
        "`repo` is hosted outside the allowed hosts (default: github.com, gitlab.com, bitbucket.org, codeberg.org)",
        "Move the hook to a trusted host, or add the host to `allowed_hosts` in .precommitguard.yaml"
    ),
    rule!(
        "PCG005",
        "sha-without-frozen-comment",
        Info,
        "supply-chain",
        "`rev` is pinned to a SHA but has no `# frozen: vX.Y.Z` comment, so reviewers cannot tell which release it is",
        "Re-run `pre-commit autoupdate --freeze` or add `# frozen: <tag>` after the SHA"
    ),
    rule!(
        "PCG006",
        "frozen-comment-on-mutable-rev",
        Warning,
        "supply-chain",
        "A `# frozen:` comment is attached to a `rev` that is not a full SHA, which falsely suggests the hook is pinned",
        "Replace the rev with the commit SHA (`pre-commit autoupdate --freeze`) or drop the comment"
    ),
    rule!(
        "PCG007",
        "remote-code-in-entry",
        Error,
        "supply-chain",
        "Hook `entry` downloads code and executes it (e.g. `curl ... | sh`, `iwr ... | iex`), bypassing pinning entirely",
        "Vendor the script into the repository or install the tool through a pinned hook repo / additional_dependencies"
    ),
    rule!(
        "PCG008",
        "suspicious-unsupported-entry",
        Warning,
        "supply-chain",
        "An `unsupported`/`system`/`script` hook entry uses risky shell constructs (network fetches, eval, base64 decoding, sudo, chmod 777, /dev/tcp, rm -rf /)",
        "Move the logic into a reviewed script in the repository, or into a pinned hook repository"
    ),
    rule!(
        "PCG009",
        "unpinned-additional-dependency",
        Warning,
        "supply-chain",
        "An `additional_dependencies` entry has no exact version (python `==`, node `@x.y.z`, go `@vX.Y.Z`), so every fresh install may pull different code",
        "Pin exact versions, e.g. `types-requests==2.32.0.20240914` or `eslint@9.14.0`"
    ),
    rule!(
        "PCG010",
        "unpinned-docker-image",
        Warning,
        "supply-chain",
        "A `docker_image` hook runs an image by tag rather than by `@sha256:` digest",
        "Reference the image by digest, e.g. `koalaman/shellcheck:v0.10.0@sha256:...`"
    ),
    rule!(
        "PCG011",
        "archived-or-moved-repo",
        Warning,
        "supply-chain",
        "`repo` points at a hook repository that is archived or has moved (only a redirect keeps it working; a re-registered name could serve different code)",
        "Switch to the maintained location shown in the message"
    ),
    rule!(
        "PCG012",
        "yaml-syntax-error",
        Error,
        "correctness",
        "The file is not valid YAML",
        "Fix the syntax error at the reported line and column"
    ),
    rule!(
        "PCG013",
        "unknown-key",
        Warning,
        "correctness",
        "Unknown or misplaced key (usually a typo such as `additional_dependency` or `exlude`); pre-commit only warns and ignores it",
        "Fix the spelling or remove the key; see https://pre-commit.com/#plugins for valid keys"
    ),
    rule!(
        "PCG014",
        "duplicate-repo",
        Warning,
        "hygiene",
        "The same remote repository appears more than once, so it is installed and pinned twice and the revs can drift apart",
        "Merge the hooks into a single `repos:` entry"
    ),
    rule!(
        "PCG015",
        "duplicate-hook",
        Info,
        "hygiene",
        "The same hook id is listed twice in one repository with the same alias and name, so the two runs cannot be told apart in output or with `pre-commit run <alias>`",
        "Merge the entries, or give each one a distinct `alias` and `name`"
    ),
    rule!(
        "PCG016",
        "deprecated-stage",
        Warning,
        "correctness",
        "Deprecated stage name (`commit`, `push`, `merge-commit`; deprecated in pre-commit 3.2.0)",
        "Use `pre-commit`, `pre-push`, `pre-merge-commit`, or run `pre-commit migrate-config`"
    ),
    rule!(
        "PCG017",
        "invalid-stage",
        Error,
        "correctness",
        "Unknown stage or hook type in `stages`, `default_stages` or `default_install_hook_types`",
        "Use one of: commit-msg, post-checkout, post-commit, post-merge, post-rewrite, pre-commit, pre-merge-commit, pre-push, pre-rebase, prepare-commit-msg, manual"
    ),
    rule!(
        "PCG018",
        "invalid-language",
        Error,
        "correctness",
        "Unknown or removed `language` (e.g. `python_venv` was removed in pre-commit 4.0.0, `pcre` in 2.0.0)",
        "Use a supported language such as `python`, `pygrep`, `node`, `unsupported`"
    ),
    rule!(
        "PCG019",
        "legacy-language-alias",
        Info,
        "hygiene",
        "`language: system` / `language: script` are legacy aliases of `unsupported` / `unsupported_script` (pre-commit 4.4.0) and are slated for deprecation",
        "Rename to `unsupported` / `unsupported_script` and set `minimum_pre_commit_version: '4.4.0'`"
    ),
    rule!(
        "PCG020",
        "deprecated-sha-key",
        Error,
        "correctness",
        "`sha:` was replaced by `rev:` in pre-commit 1.7.0 and is no longer accepted",
        "Rename `sha:` to `rev:` (or run `pre-commit migrate-config`)"
    ),
    rule!(
        "PCG021",
        "invalid-regex",
        Error,
        "correctness",
        "`files` / `exclude` is not a valid regular expression (they are Python regexes, not globs)",
        "Write a regex, e.g. `\\.py$` instead of `*.py`"
    ),
    rule!(
        "PCG022",
        "glob-like-regex",
        Warning,
        "correctness",
        "`files` / `exclude` looks like a glob (`/*`, `**`) or uses `[\\\\/]`; pre-commit matches regexes against forward-slash paths",
        "Use regex syntax: `^docs/` instead of `docs/*`, and `/` instead of `[\\\\/]`"
    ),
    rule!(
        "PCG023",
        "schema-error",
        Error,
        "correctness",
        "A required key is missing or a value has the wrong type (e.g. `repo: local` hook without `entry`, `rev: 1.0` parsed as a number, unknown `meta` hook)",
        "Add the missing key or quote the value; local hooks need id, name, entry and language"
    ),
    rule!(
        "PCG024",
        "empty-hooks",
        Warning,
        "hygiene",
        "A repository has no hooks, or the config has no repos, so nothing runs",
        "Add hooks or remove the entry"
    ),
    rule!(
        "PCG025",
        "always-run-with-filters",
        Info,
        "hygiene",
        "`always_run: true` is combined with `files`/`types` filters: the hook runs on every commit anyway and the filters only narrow the filenames passed",
        "Drop the filters, or drop `always_run` and let the filters decide when the hook runs"
    ),
    rule!(
        "PCG026",
        "minimum-version-missing",
        Info,
        "hygiene",
        "The config uses features that need a newer pre-commit (e.g. `unsupported` language 4.4.0, `pre-commit` stage names 3.2.0) without declaring `minimum_pre_commit_version`",
        "Add `minimum_pre_commit_version: '<version>'` so older installs fail with a clear message"
    ),
];

/// Look up a rule by ID (case-insensitive).
pub fn rule(id: &str) -> Option<&'static Rule> {
    RULES.iter().find(|r| r.id.eq_ignore_ascii_case(id))
}

/// Finding for a YAML syntax error.
pub fn parse_error(e: &ParseError) -> Finding {
    Finding {
        rule: "PCG012",
        severity: Severity::Error,
        message: format!("invalid YAML: {}", e.message),
        line: e.line,
        column: e.col,
    }
}

/// (version, reason, line, column) of a feature that needs a newer pre-commit.
type Need = ((u32, u32, u32), String, usize, usize);

struct Ctx<'a> {
    cfg: &'a Config,
    frozen: HashMap<usize, String>,
    out: Vec<Finding>,
    /// Highest (version, reason, line, col) requirement seen for PCG026.
    needs: Option<Need>,
}

impl Ctx<'_> {
    fn push(&mut self, id: &'static str, at: &Node, message: String) {
        self.push_at(id, at.line, at.col, message);
    }

    fn push_at(&mut self, id: &'static str, line: usize, column: usize, message: String) {
        let severity = rule(id).map(|r| r.severity).unwrap_or(Severity::Warning);
        self.out.push(Finding {
            rule: id,
            severity,
            message,
            line,
            column,
        });
    }

    fn need(&mut self, version: (u32, u32, u32), why: &str, at: &Node) {
        if self.needs.as_ref().is_none_or(|(v, ..)| version > *v) {
            self.needs = Some((version, why.to_string(), at.line, at.col));
        }
    }
}

/// Run every check over a parsed document.
pub fn check(root: Option<&Node>, kind: Kind, cfg: &Config, comments: &[Comment]) -> Vec<Finding> {
    let frozen = comments
        .iter()
        .filter(|c| !c.own_line)
        .filter_map(|c| {
            let t = c.text.trim();
            t.strip_prefix("frozen:")
                .map(|v| (c.line, v.trim().to_string()))
        })
        .collect();
    let mut cx = Ctx {
        cfg,
        frozen,
        out: Vec::new(),
        needs: None,
    };
    match kind {
        Kind::Config => check_config(&mut cx, root),
        Kind::Manifest => check_manifest(&mut cx, root),
    }
    cx.out
}

fn key_of(k: &Node) -> &str {
    k.as_str().unwrap_or("")
}

fn version_tuple(s: &str) -> Option<(u32, u32, u32)> {
    let mut it = s.trim().split('.').map(|p| p.parse::<u32>().ok());
    let a = it.next()??;
    let b = it.next().flatten().unwrap_or(0);
    let c = it.next().flatten().unwrap_or(0);
    Some((a, b, c))
}

fn check_config(cx: &mut Ctx, root: Option<&Node>) {
    let Some(root) = root else {
        cx.push_at(
            "PCG024",
            1,
            1,
            "config is empty: no `repos:` defined".into(),
        );
        return;
    };
    let Some(map) = root.as_map() else {
        cx.push(
            "PCG023",
            root,
            "top level must be a mapping with a `repos:` key (list-style configs are no longer supported)".into(),
        );
        return;
    };
    let mut min_version = None;
    for (k, v) in map {
        let key = key_of(k);
        match key {
            "repos" => {}
            "files" | "exclude" => check_regex(cx, key, v),
            "default_stages" => check_stages(cx, v, true),
            "default_install_hook_types" => {
                cx.need((2, 18, 0), "`default_install_hook_types`", k);
                for item in v.str_items() {
                    let s = item.as_str().unwrap_or_default();
                    if !data::HOOK_TYPES.contains(&s) {
                        cx.push(
                            "PCG017",
                            item,
                            format!("unknown hook type `{s}` in `default_install_hook_types`"),
                        );
                    }
                }
            }
            "default_language_version" => {
                for (lk, _) in v.as_map().unwrap_or_default() {
                    let l = key_of(lk);
                    if !data::LANGUAGES.contains(&l)
                        && !data::LEGACY_LANGUAGES.iter().any(|(o, _)| *o == l)
                    {
                        cx.push(
                            "PCG018",
                            lk,
                            format!("unknown language `{l}` in `default_language_version`"),
                        );
                    }
                }
            }
            "minimum_pre_commit_version" => {
                min_version = v.as_str().and_then(version_tuple);
                if min_version.is_none() {
                    cx.push(
                        "PCG023",
                        v,
                        "`minimum_pre_commit_version` must be a quoted version string like '3.2.0'"
                            .into(),
                    );
                }
            }
            "fail_fast" => check_bool(cx, key, v),
            k2 if data::TOP_KEYS.contains(&k2) || k2 == "<<" => {}
            k2 if data::PREK_TOP_KEYS.contains(&k2) => {
                if cx.cfg.strict_upstream {
                    cx.push(
                        "PCG013",
                        k,
                        format!("`{k2}` is a prek-only key; upstream pre-commit ignores it"),
                    );
                }
            }
            "sha" => cx.push(
                "PCG020",
                k,
                "`sha:` is not valid at the top level; did you mean `rev:` inside a repo?".into(),
            ),
            other => cx.push(
                "PCG013",
                k,
                format!(
                    "unknown top-level key `{other}`{}",
                    suggest(other, data::TOP_KEYS)
                ),
            ),
        }
    }
    match root.get("repos") {
        None => cx.push("PCG023", root, "missing required key `repos`".into()),
        Some(repos) => match repos.as_seq() {
            Some([]) => cx.push(
                "PCG024",
                repos,
                "`repos` is empty: no hooks will run".into(),
            ),
            Some(items) => {
                let mut seen: HashMap<String, usize> = HashMap::new();
                for r in items {
                    check_repo(cx, r, &mut seen);
                }
            }
            None if repos.is_null() => cx.push(
                "PCG024",
                repos,
                "`repos` is empty: no hooks will run".into(),
            ),
            None => cx.push("PCG023", repos, "`repos` must be a list".into()),
        },
    }
    if let Some((need, why, line, col)) = cx.needs.clone()
        && min_version.is_none_or(|m| m < need)
    {
        let v = format!("{}.{}.{}", need.0, need.1, need.2);
        let msg = match min_version {
            Some(m) => format!(
                "{why} requires pre-commit {v} but `minimum_pre_commit_version` is {}.{}.{}",
                m.0, m.1, m.2
            ),
            None => format!(
                "{why} requires pre-commit {v}; declare `minimum_pre_commit_version: '{v}'`"
            ),
        };
        cx.push_at("PCG026", line, col, msg);
    }
}

fn check_repo(cx: &mut Ctx, r: &Node, seen: &mut HashMap<String, usize>) {
    let Some(map) = r.as_map() else {
        cx.push("PCG023", r, "each `repos` entry must be a mapping".into());
        return;
    };
    let repo = r.get("repo").and_then(Node::as_str).unwrap_or("");
    let special = matches!(repo, "local" | "meta" | "builtin");
    for (k, v) in map {
        match key_of(k) {
            "repo" | "hooks" | "<<" => {}
            "rev" if special => cx.push(
                "PCG013",
                k,
                format!("`rev` is not allowed for `repo: {repo}`"),
            ),
            "rev" => {}
            "sha" => {
                let _ = v;
                cx.push(
                    "PCG020",
                    k,
                    "`sha:` was renamed to `rev:` in pre-commit 1.7.0".into(),
                )
            }
            other => cx.push(
                "PCG013",
                k,
                format!(
                    "unknown key `{other}` in repo entry{}",
                    suggest(other, data::REPO_KEYS)
                ),
            ),
        }
    }
    match r.get("repo") {
        None => cx.push(
            "PCG023",
            r,
            "repo entry is missing required key `repo`".into(),
        ),
        Some(n) if n.as_str().is_none() => cx.push("PCG023", n, "`repo` must be a string".into()),
        _ => {}
    }
    let repo_node = r.get("repo");
    if !special && let Some(rn) = repo_node.filter(|n| n.as_str().is_some()) {
        check_repo_url(cx, rn, repo);
        let norm = data::normalize_url(repo);
        if let Some(first) = seen.get(&norm) {
            cx.push(
                "PCG014",
                rn,
                format!("repository `{repo}` is already listed on line {first}"),
            );
        } else {
            seen.insert(norm, rn.line);
        }
        match r.get("rev") {
            Some(rev) => check_rev(cx, rev, repo),
            None if r.get("sha").is_none() => cx.push(
                "PCG023",
                rn,
                format!("repo `{repo}` is missing required key `rev`"),
            ),
            None => {}
        }
    }

    let hooks = r.get("hooks");
    match hooks.map(|h| (h, h.as_seq())) {
        None => cx.push(
            "PCG024",
            repo_node.unwrap_or(r),
            format!("repo `{repo}` has no `hooks`"),
        ),
        Some((h, Some([]))) => cx.push(
            "PCG024",
            h,
            format!("repo `{repo}` has an empty `hooks` list"),
        ),
        Some((h, None)) if h.is_null() => cx.push(
            "PCG024",
            h,
            format!("repo `{repo}` has an empty `hooks` list"),
        ),
        Some((h, None)) => cx.push("PCG023", h, "`hooks` must be a list".into()),
        Some((_, Some(items))) => {
            let mut ids: HashMap<HookKey, usize> = HashMap::new();
            for hook in items {
                check_hook(cx, hook, HookCtx::from_repo(repo));
                dup_hook(cx, hook, &mut ids);
            }
        }
    }
}

type HookKey = (String, String, String);

fn dup_hook(cx: &mut Ctx, hook: &Node, ids: &mut HashMap<HookKey, usize>) {
    let Some(idn) = hook.get("id").filter(|n| n.as_str().is_some()) else {
        return;
    };
    let id = idn.as_str().unwrap_or_default().to_string();
    let alias = hook
        .get("alias")
        .and_then(Node::as_str)
        .unwrap_or("")
        .to_string();
    let name = hook
        .get("name")
        .and_then(Node::as_str)
        .unwrap_or("")
        .to_string();
    if let Some(first) = ids.get(&(id.clone(), alias.clone(), name.clone())) {
        cx.push(
            "PCG015",
            idn,
            format!("hook `{id}` is already defined on line {first}"),
        );
    } else {
        ids.insert((id, alias, name), idn.line);
    }
}

fn check_repo_url(cx: &mut Ctx, rn: &Node, repo: &str) {
    let lower = repo.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("git://") {
        cx.push(
            "PCG003",
            rn,
            format!("`{repo}` uses an unencrypted, unauthenticated transport"),
        );
    }
    if let Some(host) = host_of(&lower)
        && !cx.cfg.host_allowed(&host)
    {
        cx.push(
            "PCG004",
            rn,
            format!("repository host `{host}` is not in the allowed hosts"),
        );
    }
    if let Some(m) = data::moved(repo) {
        let what = match m.kind {
            MoveKind::Archived => "is archived",
            MoveKind::Moved => "has moved",
        };
        cx.push("PCG011", rn, format!("`{repo}` {what}; use {}", m.to));
    }
}

/// Host of a remote URL, or `None` for local paths / file URLs.
fn host_of(url: &str) -> Option<String> {
    if let Some(r) = url.strip_prefix("git@") {
        return r.split(':').next().map(str::to_string);
    }
    let (scheme, rest) = url.split_once("://")?;
    if scheme == "file" {
        return None;
    }
    let authority = rest.split('/').next()?;
    let host = authority.rsplit('@').next()?;
    let host = host.split(':').next()?;
    Some(host.to_string())
}

fn is_full_sha(s: &str) -> bool {
    (s.len() == 40 || s.len() == 64) && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn check_rev(cx: &mut Ctx, rev: &Node, repo: &str) {
    let Some(s) = rev.as_str() else {
        cx.push("PCG023", rev, "`rev` must be a string".into());
        return;
    };
    if matches!(&rev.value, Value::Scalar(_, true)) && s.parse::<f64>().is_ok() && !is_full_sha(s) {
        cx.push(
            "PCG023",
            rev,
            format!("`rev: {s}` is parsed by YAML as a number; quote it"),
        );
    }
    let frozen = cx.frozen.get(&rev.line).cloned();
    if is_full_sha(s) {
        if frozen.is_none() {
            cx.push(
                "PCG005",
                rev,
                format!(
                    "`{}` has no `# frozen: <tag>` comment",
                    &s[..12.min(s.len())]
                ),
            );
        }
        return;
    }
    if frozen.is_some() {
        cx.push(
            "PCG006",
            rev,
            format!("`# frozen:` comment on non-SHA rev `{s}`"),
        );
    }
    let lower = s.to_ascii_lowercase();
    let hex = !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit());
    let branch_like = data::BRANCH_NAMES.contains(&lower.as_str())
        || lower.starts_with("refs/heads/")
        || (!s.contains('.') && !hex && !s.chars().any(|c| c.is_ascii_digit()));
    if branch_like {
        cx.push(
            "PCG002",
            rev,
            format!("`rev: {s}` looks like a branch (mutable reference)"),
        );
    } else if cx.cfg.tags_allowed(repo) {
    } else if hex {
        cx.push(
            "PCG001",
            rev,
            format!("`rev: {s}` is an abbreviated SHA; use the full 40-character SHA"),
        );
    } else {
        cx.push(
            "PCG001",
            rev,
            format!("`rev: {s}` is a mutable tag, not a commit SHA"),
        );
    }
}

/// Where a hook lives, which decides which keys are required.
#[derive(Clone, Copy, PartialEq)]
enum HookCtx {
    Remote,
    Local,
    Meta,
    Builtin,
    Manifest,
}

impl HookCtx {
    fn from_repo(repo: &str) -> Self {
        match repo {
            "local" => HookCtx::Local,
            "meta" => HookCtx::Meta,
            // prek's `repo: builtin` hooks only need an id, like meta hooks.
            "builtin" => HookCtx::Builtin,
            _ => HookCtx::Remote,
        }
    }
}

const BOOL_KEYS: &[&str] = &[
    "always_run",
    "fail_fast",
    "pass_filenames",
    "require_serial",
    "verbose",
];

fn check_hook(cx: &mut Ctx, hook: &Node, hc: HookCtx) {
    let Some(map) = hook.as_map() else {
        cx.push("PCG023", hook, "each hook must be a mapping".into());
        return;
    };
    let id = hook.get("id").and_then(Node::as_str).unwrap_or("?");
    for (k, v) in map {
        let key = key_of(k);
        if key == "<<" {
            continue;
        }
        if data::PREK_HOOK_KEYS.contains(&key) {
            if cx.cfg.strict_upstream {
                cx.push(
                    "PCG013",
                    k,
                    format!(
                        "`{key}` in hook `{id}` is a prek-only key; upstream pre-commit ignores it"
                    ),
                );
            }
            continue;
        }
        if !data::HOOK_KEYS.contains(&key) {
            cx.push(
                "PCG013",
                k,
                format!(
                    "unknown key `{key}` in hook `{id}`{}",
                    suggest(key, data::HOOK_KEYS)
                ),
            );
            continue;
        }
        if hc == HookCtx::Meta && key == "entry" {
            cx.push(
                "PCG013",
                k,
                "`entry` is not allowed for `repo: meta` hooks".into(),
            );
        }
        let prek_int =
            key == "pass_filenames" && v.as_str().is_some_and(|s| s.parse::<u32>().is_ok());
        if BOOL_KEYS.contains(&key) && !prek_int {
            check_bool(cx, key, v);
        }
        match key {
            "files" | "exclude" => check_regex(cx, key, v),
            "stages" => check_stages(cx, v, false),
            "types_or" => cx.need((2, 9, 0), "`types_or`", k),
            "minimum_pre_commit_version" if v.as_str().and_then(version_tuple).is_none() => cx
                .push(
                    "PCG023",
                    v,
                    "`minimum_pre_commit_version` must be a quoted version string".into(),
                ),
            "args" | "additional_dependencies" | "types" | "exclude_types"
                if v.as_seq().is_none() && !v.is_null() =>
            {
                cx.push("PCG023", v, format!("`{key}` must be a list"))
            }
            _ => {}
        }
    }
    let required: &[&str] = match hc {
        HookCtx::Remote | HookCtx::Meta | HookCtx::Builtin => &["id"],
        HookCtx::Local | HookCtx::Manifest => &["id", "name", "entry", "language"],
    };
    let missing: Vec<&str> = required
        .iter()
        .copied()
        .filter(|k| hook.get(k).is_none())
        .collect();
    if !missing.is_empty() {
        let at = hook.get("id").unwrap_or(hook);
        let wherein = match hc {
            HookCtx::Local => "`repo: local` hook",
            HookCtx::Manifest => "manifest hook",
            _ => "hook",
        };
        cx.push(
            "PCG023",
            at,
            format!(
                "{wherein} `{id}` is missing required key(s): {}",
                missing.join(", ")
            ),
        );
    }
    if hc == HookCtx::Meta
        && let Some(idn) = hook.get("id")
        && !data::META_HOOKS.contains(&id)
    {
        cx.push(
            "PCG023",
            idn,
            format!(
                "unknown meta hook `{id}` (expected one of: {})",
                data::META_HOOKS.join(", ")
            ),
        );
    }

    let language = hook.get("language").and_then(Node::as_str);
    if let Some(ln) = hook.get("language") {
        check_language(cx, ln, hc);
    }
    if let Some(en) = hook.get("entry")
        && let Some(entry) = en.as_str()
    {
        check_entry(cx, en, entry, language);
    }
    if let Some(deps) = hook.get("additional_dependencies") {
        for d in deps.as_seq().unwrap_or_default() {
            if let Some(spec) = d.as_str()
                && let Some(why) = unpinned(spec, language)
            {
                cx.push("PCG009", d, format!("additional dependency `{spec}` {why}"));
            }
        }
    }
    if hook.get("always_run").and_then(Node::as_bool) == Some(true)
        && let Some(f) = ["files", "types", "types_or"]
            .iter()
            .find_map(|k| hook.get(k))
    {
        let _ = f;
        let at = hook.entry("always_run").map(|(k, _)| k).unwrap_or(hook);
        cx.push(
            "PCG025",
            at,
            format!("hook `{id}` sets `always_run: true` together with file filters"),
        );
    }
}

fn check_bool(cx: &mut Ctx, key: &str, v: &Node) {
    if v.as_bool().is_none() {
        cx.push("PCG023", v, format!("`{key}` must be true or false"));
    }
}

fn check_language(cx: &mut Ctx, ln: &Node, hc: HookCtx) {
    let Some(l) = ln.as_str() else {
        cx.push("PCG023", ln, "`language` must be a string".into());
        return;
    };
    if let Some((_, new, ver)) = data::REMOVED_LANGUAGES.iter().find(|(o, ..)| *o == l) {
        cx.push(
            "PCG018",
            ln,
            format!("`language: {l}` was removed in pre-commit {ver}; use `{new}`"),
        );
    } else if let Some((_, new)) = data::LEGACY_LANGUAGES.iter().find(|(o, _)| *o == l) {
        cx.push(
            "PCG019",
            ln,
            format!("`language: {l}` is a legacy alias; use `{new}`"),
        );
    } else if !data::LANGUAGES.contains(&l) {
        cx.push(
            "PCG018",
            ln,
            format!("unknown language `{l}`{}", suggest(l, data::LANGUAGES)),
        );
    } else if matches!(l, "unsupported" | "unsupported_script") {
        cx.need((4, 4, 0), &format!("`language: {l}`"), ln);
    }
    if hc == HookCtx::Meta && !matches!(l, "unsupported" | "system") {
        cx.push(
            "PCG023",
            ln,
            "`repo: meta` hooks only support `language: unsupported`".into(),
        );
    }
}

fn check_stages(cx: &mut Ctx, v: &Node, top: bool) {
    let field = if top { "default_stages" } else { "stages" };
    if v.as_seq().is_none() && !v.is_null() {
        cx.push("PCG023", v, format!("`{field}` must be a list"));
        return;
    }
    for item in v.str_items() {
        let s = item.as_str().unwrap_or_default();
        if let Some((_, new)) = data::DEPRECATED_STAGES.iter().find(|(o, _)| *o == s) {
            cx.push(
                "PCG016",
                item,
                format!("stage `{s}` is deprecated; use `{new}`"),
            );
        } else if s == "manual" || data::HOOK_TYPES.contains(&s) {
            if matches!(s, "pre-commit" | "pre-push" | "pre-merge-commit") {
                cx.need((3, 2, 0), &format!("stage name `{s}`"), item);
            }
        } else {
            cx.push("PCG017", item, format!("unknown stage `{s}` in `{field}`"));
        }
    }
}

/// Translate Python-only regex syntax to something `fancy-regex` accepts.
fn py_to_rust_regex(p: &str) -> String {
    let mut out = String::with_capacity(p.len());
    let mut chars = p.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('Z') => out.push_str("\\z"),
                Some(n) => {
                    out.push('\\');
                    out.push(n);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Validate a Python regex. Returns the error message if it does not compile.
pub fn regex_error(pattern: &str) -> Option<String> {
    fancy_regex::Regex::new(&py_to_rust_regex(pattern))
        .err()
        .map(|e| e.to_string())
}

fn check_regex(cx: &mut Ctx, key: &str, v: &Node) {
    let Some(p) = v.as_str() else {
        // prek accepts `{glob: ...}` mappings here.
        if v.as_map().is_some() && cx.cfg.strict_upstream {
            cx.push(
                "PCG013",
                v,
                format!("`{key}` glob mappings are prek-only; upstream pre-commit expects a regex"),
            );
        } else if !v.is_null() && v.as_map().is_none() {
            cx.push("PCG023", v, format!("`{key}` must be a string regex"));
        }
        return;
    };
    if let Some(err) = regex_error(p) {
        let hint = if p.starts_with('*') || p.contains("**") {
            " (this looks like a glob; pre-commit expects a regex)"
        } else {
            ""
        };
        let err = err.lines().next().unwrap_or("").to_string();
        cx.push(
            "PCG021",
            v,
            format!("`{key}` is not a valid regex: {err}{hint}"),
        );
        return;
    }
    if p.contains("/*") || p.contains("**") {
        cx.push(
            "PCG022",
            v,
            format!(
                "`{key}: {p}` looks like a glob; `/*` in a regex means \"zero or more slashes\""
            ),
        );
    } else if [r"[\\/]", r"[\/]", r"[/\\]"].iter().any(|s| p.contains(s)) {
        cx.push("PCG022", v, format!("`{key}`: pre-commit normalizes paths to forward slashes, so use `/` instead of a slash class"));
    }
}

static REMOTE_EXEC: LazyLock<Vec<regex_lite::Regex>> = LazyLock::new(|| {
    [
        r"(?i)\b(curl|wget|fetch)\b[^|;&]*\|\s*(sudo\s+)?(env\s+\S*\s*)?(ba|z|da|k|fi)?sh\b",
        r"(?i)\b(curl|wget)\b[^|;&]*\|\s*(sudo\s+)?(python[0-9.]*|perl|ruby|node|php|bun|deno)\b",
        r"(?i)\b(ba|z|da|k)?sh\s+(-c\s+)?[\x22']?(<\(|\$\(|`)\s*(curl|wget)\b",
        r"(?i)\b(iex|invoke-expression)\b.*\b(iwr|irm|invoke-webrequest|invoke-restmethod|downloadstring|net\.webclient)\b",
        r"(?i)\b(iwr|irm|invoke-webrequest|invoke-restmethod)\b[^|]*\|\s*(iex|invoke-expression)\b",
        r"(?i)\b(curl|wget)\b[^;&|]*\s(-o|-O|--output|--output-document)\b[^;&|]*(&&|;)\s*(sudo\s+)?((ba|z)?sh|chmod\s+\+x|\./)",
        r"(?i)\b(python[0-9.]*)\s+-c\s+.*\b(urlopen|urllib|requests\.get)\b.*\bexec\b",
    ]
    .iter()
    .map(|p| regex_lite::Regex::new(p).expect("valid builtin regex"))
    .collect()
});

static SUSPICIOUS: LazyLock<Vec<(regex_lite::Regex, &'static str)>> = LazyLock::new(|| {
    [
        (
            r"(?i)\b(curl|wget|iwr|invoke-webrequest)\b",
            "fetches from the network",
        ),
        (r"base64\s+(-d|--decode|-D)\b", "decodes base64 payloads"),
        (r"(^|[\s;&|(])eval\b", "uses eval"),
        (r"(^|[\s;&|(])sudo\b", "uses sudo"),
        (
            r"chmod\s+(-R\s+)?(0?777|a\+w|o\+w)\b",
            "makes files world-writable",
        ),
        (r"/dev/(tcp|udp)/", "opens raw network sockets"),
        (r"(^|[\s;&|(])(nc|ncat|netcat)\s+-", "uses netcat"),
        (
            r"rm\s+-(rf|fr)\s+(/|~|\$HOME)(\s|$|\*)",
            "deletes the root or home directory",
        ),
    ]
    .iter()
    .map(|(p, why)| {
        (
            regex_lite::Regex::new(p).expect("valid builtin regex"),
            *why,
        )
    })
    .collect()
});

fn check_entry(cx: &mut Ctx, en: &Node, entry: &str, language: Option<&str>) {
    if REMOTE_EXEC.iter().any(|r| r.is_match(entry)) {
        cx.push(
            "PCG007",
            en,
            "entry downloads and executes remote code".into(),
        );
        return;
    }
    if matches!(
        language,
        Some("unsupported" | "unsupported_script" | "system" | "script")
    ) && let Some((_, why)) = SUSPICIOUS.iter().find(|(r, _)| r.is_match(entry))
    {
        cx.push(
            "PCG008",
            en,
            format!("`{}` hook entry {why}", language.unwrap_or_default()),
        );
    }
    if language == Some("docker_image") {
        let mut toks = entry.split_whitespace();
        let mut image = None;
        while let Some(t) = toks.next() {
            if t == "--entrypoint" || t == "-u" || t == "--user" || t == "-e" || t == "--env" {
                toks.next();
            } else if !t.starts_with('-') {
                image = Some(t);
                break;
            }
        }
        if let Some(img) = image
            && !img.contains("@sha256:")
        {
            cx.push(
                "PCG010",
                en,
                format!("docker image `{img}` is not pinned by digest"),
            );
        }
    }
}

static SEMVER: LazyLock<regex_lite::Regex> =
    LazyLock::new(|| regex_lite::Regex::new(r"^v?\d+\.\d+\.\d+([-+][0-9A-Za-z.\-+]*)?$").unwrap());

/// Why an additional dependency is not pinned, or `None` if it is.
fn unpinned(spec: &str, language: Option<&str>) -> Option<&'static str> {
    let s = spec.trim();
    if s.is_empty() || s.starts_with('.') || s.starts_with('/') || s.starts_with("-") {
        return None;
    }
    let lang = match language {
        Some(l) => l,
        None if s.contains('@') && !s.contains("://") => "node",
        None if s.contains("://") => "python",
        None => "python",
    };
    match lang {
        "python" | "conda" => {
            if s.contains("://") {
                let at_sha = s.rsplit_once('@').is_some_and(|(_, r)| {
                    is_full_sha(r.split(['#', ' ', ';']).next().unwrap_or(""))
                });
                return (!at_sha).then_some("is a URL without a pinned commit SHA");
            }
            let base = s.split(';').next().unwrap_or(s);
            if base.contains("===")
                || base.contains("==") && !base.contains("==*") && !base.ends_with(".*")
            {
                None
            } else if base.contains(['>', '<', '~', '!']) || base.contains("==") {
                Some("uses a version range; pin an exact version with `==`")
            } else {
                Some("has no version pin; use `name==x.y.z`")
            }
        }
        "node" => {
            let rest = s
                .strip_prefix('@')
                .map(|r| r.split_once('/').map(|(_, n)| n).unwrap_or(r))
                .unwrap_or(s);
            match rest.rsplit_once('@') {
                Some((_, v)) if SEMVER.is_match(v) => None,
                Some(_) => Some("uses a range or dist-tag; pin an exact `@x.y.z`"),
                None => Some("has no version pin; use `name@x.y.z`"),
            }
        }
        "golang" => match s.rsplit_once('@') {
            Some((_, v))
                if SEMVER.is_match(v)
                    || is_full_sha(v)
                    || v.len() >= 12 && v.chars().all(|c| c.is_ascii_hexdigit()) =>
            {
                None
            }
            Some(_) => Some("uses `@latest` or a branch; pin `@vX.Y.Z`"),
            None => Some("has no version; use `module@vX.Y.Z`"),
        },
        "rust" => match s.rsplit_once(':') {
            Some((_, v))
                if SEMVER.is_match(v) || v.split('.').all(|p| p.parse::<u32>().is_ok()) =>
            {
                None
            }
            _ => Some("has no version; use `name:x.y.z` or `cli:name:x.y.z`"),
        },
        _ => None,
    }
}

fn check_manifest(cx: &mut Ctx, root: Option<&Node>) {
    let Some(root) = root else {
        cx.push_at("PCG024", 1, 1, "manifest is empty: no hooks defined".into());
        return;
    };
    let Some(items) = root.as_seq() else {
        cx.push(
            "PCG023",
            root,
            "`.pre-commit-hooks.yaml` must be a list of hooks".into(),
        );
        return;
    };
    if items.is_empty() {
        cx.push("PCG024", root, "manifest defines no hooks".into());
    }
    let mut ids = HashMap::new();
    for hook in items {
        check_hook(cx, hook, HookCtx::Manifest);
        dup_hook(cx, hook, &mut ids);
    }
}

/// " (did you mean `x`?)" when a close match exists.
fn suggest(word: &str, candidates: &[&str]) -> String {
    let best = candidates
        .iter()
        .map(|c| (levenshtein(word, c), *c))
        .min_by_key(|(d, _)| *d);
    match best {
        Some((d, c)) if d <= 2.max(word.len() / 4) => format!(" (did you mean `{c}`?)"),
        _ => String::new(),
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != *cb);
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_ids_are_unique_and_sequential() {
        for (i, r) in RULES.iter().enumerate() {
            assert_eq!(r.id, format!("PCG{:03}", i + 1));
            assert!(!r.fix.is_empty() && !r.description.is_empty());
        }
    }

    #[test]
    fn python_regex_compat() {
        assert!(regex_error(r"^(docs/|tests/fixtures/).*\.py\Z").is_none());
        assert!(regex_error(r"(?x)^(\n  a/.*|\n  b/.*\n)$").is_none());
        assert!(regex_error(r"(?P<n>a)(?P=n)").is_none());
        assert!(regex_error(r"^(?!vendor/).*\.js$").is_none());
        assert!(regex_error(r"\-\/\#").is_none());
        assert!(regex_error("*.py").is_some());
        assert!(regex_error("(unclosed").is_some());
    }

    #[test]
    fn pinning() {
        assert_eq!(unpinned("types-requests==2.32.0", Some("python")), None);
        assert!(unpinned("types-requests", Some("python")).is_some());
        assert!(unpinned("flake8>=7", Some("python")).is_some());
        assert_eq!(
            unpinned("foo==1.0; python_version<'3.12'", Some("python")),
            None
        );
        assert_eq!(unpinned("eslint@9.14.0", Some("node")), None);
        assert_eq!(unpinned("@types/node@22.1.0", Some("node")), None);
        assert!(unpinned("@types/node", Some("node")).is_some());
        assert!(unpinned("eslint@^9", Some("node")).is_some());
        assert_eq!(
            unpinned("golang.org/x/tools/cmd/goimports@v0.27.0", Some("golang")),
            None
        );
        assert!(unpinned("golang.org/x/tools/cmd/goimports@latest", Some("golang")).is_some());
        assert_eq!(unpinned("cli:cargo-deny:0.16.2", Some("rust")), None);
        assert!(unpinned("git+https://github.com/a/b@main", Some("python")).is_some());
        assert_eq!(
            unpinned(
                "git+https://github.com/a/b@0123456789012345678901234567890123456789",
                Some("python")
            ),
            None
        );
        assert_eq!(unpinned("rubocop", Some("ruby")), None);
    }

    #[test]
    fn hosts() {
        assert_eq!(host_of("https://github.com/a/b"), Some("github.com".into()));
        assert_eq!(
            host_of("https://u:p@git.example.com:8443/a"),
            Some("git.example.com".into())
        );
        assert_eq!(host_of("git@gitlab.com:a/b.git"), Some("gitlab.com".into()));
        assert_eq!(host_of("/abs/path"), None);
        assert_eq!(host_of("file:///tmp/x"), None);
    }

    #[test]
    fn suggestions() {
        assert_eq!(
            suggest("exlude", data::HOOK_KEYS),
            " (did you mean `exclude`?)"
        );
        assert_eq!(suggest("zzzzzzzzzz", data::HOOK_KEYS), "");
    }

    #[test]
    fn remote_exec_patterns() {
        let hit = |e: &str| REMOTE_EXEC.iter().any(|r| r.is_match(e));
        assert!(hit("curl -fsSL https://x.sh | bash"));
        assert!(hit("bash -c \"$(curl -fsSL https://x)\""));
        assert!(hit("wget -qO- https://x | sudo sh -s"));
        assert!(hit("powershell -c \"iwr https://x | iex\""));
        assert!(hit("curl -o /tmp/i.sh https://x && sh /tmp/i.sh"));
        assert!(!hit("cargo fmt --check"));
        assert!(!hit("bash scripts/check.sh"));
    }
}

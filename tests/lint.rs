use precommitguard::{Config, Kind, lint_str};
use std::collections::BTreeSet;

fn fixture(p: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/fixtures/{p}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn rules(src: &str, kind: Kind) -> BTreeSet<&'static str> {
    lint_str(src, kind, &Config::default())
        .iter()
        .map(|f| f.rule)
        .collect()
}

fn ids(list: &[&'static str]) -> BTreeSet<&'static str> {
    list.iter().copied().collect()
}

#[test]
fn secure_config_is_clean() {
    let f = lint_str(
        &fixture("secure/.pre-commit-config.yaml"),
        Kind::Config,
        &Config::default(),
    );
    assert!(f.is_empty(), "{f:#?}");
}

#[test]
fn insecure_config_hits_supply_chain_rules() {
    let got = rules(&fixture("insecure/.pre-commit-config.yaml"), Kind::Config);
    let want = ids(&[
        "PCG001", "PCG002", "PCG003", "PCG004", "PCG005", "PCG006", "PCG007", "PCG008", "PCG009",
        "PCG010", "PCG011", "PCG019", "PCG026",
    ]);
    assert_eq!(got, want);
}

#[test]
fn hygiene_config_hits_correctness_rules() {
    let got = rules(&fixture("hygiene/.pre-commit-config.yaml"), Kind::Config);
    let want = ids(&[
        "PCG001", "PCG013", "PCG014", "PCG015", "PCG016", "PCG017", "PCG018", "PCG020", "PCG022",
        "PCG023", "PCG024", "PCG025",
    ]);
    assert_eq!(got, want);
}

#[test]
fn manifest_rules() {
    let got = rules(&fixture("manifest/.pre-commit-hooks.yaml"), Kind::Manifest);
    assert_eq!(
        got,
        ids(&["PCG007", "PCG015", "PCG016", "PCG019", "PCG023"])
    );
}

#[test]
fn parse_error_reports_location() {
    let f = lint_str(
        &fixture("broken/.pre-commit-config.yaml"),
        Kind::Config,
        &Config::default(),
    );
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].rule, "PCG012");
    assert_eq!(f[0].line, 4);
}

#[test]
fn inline_ignores() {
    let f = lint_str(
        &fixture("ignored/.pre-commit-config.yaml"),
        Kind::Config,
        &Config::default(),
    );
    assert_eq!(f.len(), 1, "{f:#?}");
    assert_eq!((f[0].rule, f[0].line), ("PCG001", 16));
    assert!(
        lint_str(
            &fixture("skipfile/.pre-commit-config.yaml"),
            Kind::Config,
            &Config::default()
        )
        .is_empty()
    );
}

#[test]
fn config_overrides() {
    let cfg = Config::parse(&fixture("strict.yaml")).unwrap();
    let f = lint_str(
        &fixture("insecure/.pre-commit-config.yaml"),
        Kind::Config,
        &cfg,
    );
    // pre-commit/* tags are allowed, git.example.net is allowed.
    assert!(!f.iter().any(|x| x.rule == "PCG004"));
    assert!(!f.iter().any(|x| x.rule == "PCG001" && x.line == 3));
    // Remaining tag findings are promoted to error.
    let tag = f.iter().find(|x| x.rule == "PCG001").unwrap();
    assert_eq!(tag.severity, precommitguard::Severity::Error);
}

fn one(src: &str) -> Vec<(&'static str, usize)> {
    lint_str(src, Kind::Config, &Config::default())
        .into_iter()
        .map(|f| (f.rule, f.line))
        .collect()
}

#[test]
fn rev_classification() {
    let base = |rev: &str| {
        format!(
            "repos:\n  - repo: https://github.com/a/b\n    rev: {rev}\n    hooks:\n      - id: x\n"
        )
    };
    assert_eq!(one(&base("HEAD")), vec![("PCG002", 3)]);
    assert_eq!(one(&base("refs/heads/release")), vec![("PCG002", 3)]);
    assert_eq!(one(&base("v1.2.3")), vec![("PCG001", 3)]);
    assert_eq!(one(&base("abc1234")), vec![("PCG001", 3)]);
    assert_eq!(
        one(&base("0123456789abcdef0123456789abcdef01234567")),
        vec![("PCG005", 3)]
    );
    assert_eq!(
        one(&base(
            "0123456789abcdef0123456789abcdef01234567  # frozen: v1.0.0"
        )),
        vec![]
    );
}

#[test]
fn empty_and_invalid_documents() {
    assert_eq!(one(""), vec![("PCG024", 1)]);
    assert_eq!(one("repos: []\n"), vec![("PCG024", 1)]);
    assert_eq!(one("- repo: x\n"), vec![("PCG023", 1)]);
    assert_eq!(one("files: x\n"), vec![("PCG023", 1)]);
}

#[test]
fn invalid_regex_and_glob_hint() {
    let src = "repos:\n  - repo: local\n    hooks:\n      - id: a\n        name: a\n        entry: a\n        language: python\n        files: '*.py'\n";
    let f = lint_str(src, Kind::Config, &Config::default());
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].rule, "PCG021");
    assert!(f[0].message.contains("glob"));
}

#[test]
fn minimum_version_satisfied() {
    let src = "minimum_pre_commit_version: '3.2.0'\nrepos:\n  - repo: local\n    hooks:\n      - id: a\n        name: a\n        entry: a\n        language: python\n        stages: [pre-push]\n";
    assert_eq!(one(src), vec![]);
    let older = src.replace("3.2.0", "2.20.0");
    assert_eq!(one(&older), vec![("PCG026", 9)]);
}

#[test]
fn prek_extensions_accepted_unless_strict() {
    let src = "minimum_prek_version: '0.2.0'\nrepos:\n  - repo: builtin\n    hooks:\n      - id: trailing-whitespace\n        priority: 10\n        env: {A: b}\n";
    assert_eq!(one(src), vec![]);
    let strict = Config::parse("strict_upstream: true").unwrap();
    let f: Vec<_> = lint_str(src, Kind::Config, &strict)
        .into_iter()
        .map(|f| (f.rule, f.line))
        .collect();
    assert_eq!(f, vec![("PCG013", 1), ("PCG013", 6), ("PCG013", 7)]);
}

#[test]
fn yaml_aliases_are_resolved() {
    let src = "repos:\n  - repo: local\n    hooks:\n      - id: a\n        name: a\n        entry: a\n        language: python\n        additional_dependencies: &deps [requests]\n      - id: b\n        name: b\n        entry: b\n        language: python\n        additional_dependencies: *deps\n";
    let rules: Vec<_> = one(src).into_iter().map(|(r, _)| r).collect();
    assert_eq!(rules, vec!["PCG009", "PCG009"]);
}

#[test]
fn every_rule_is_documented() {
    let docs =
        std::fs::read_to_string(format!("{}/docs/rules.md", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let readme =
        std::fs::read_to_string(format!("{}/README.md", env!("CARGO_MANIFEST_DIR"))).unwrap();
    for r in precommitguard::rules::RULES {
        assert!(
            docs.contains(&format!("## {}", r.id)),
            "{} missing from docs/rules.md",
            r.id
        );
        assert!(readme.contains(r.id), "{} missing from README", r.id);
    }
}

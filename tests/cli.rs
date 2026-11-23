use std::process::Command;

fn bin() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_precommitguard"));
    c.current_dir(env!("CARGO_MANIFEST_DIR"));
    c
}

fn run(args: &[&str]) -> (i32, String, String) {
    let mut c = bin();
    c.args(args);
    // The repository's own .precommitguard.yaml ignores the fixtures.
    if !matches!(args.first(), Some(&"rules" | &"explain")) && !args.contains(&"-c") {
        c.arg("--no-config");
    }
    let o = c.output().unwrap();
    (
        o.status.code().unwrap(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

#[test]
fn clean_file_exits_zero() {
    let (code, out, _) = run(&["tests/fixtures/secure", "--color", "never"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("0 problem(s) in 1 file(s)"));
}

#[test]
fn findings_exit_one_and_fail_on_controls_it() {
    assert_eq!(run(&["tests/fixtures/insecure"]).0, 1);
    assert_eq!(run(&["tests/fixtures/insecure", "--fail-on", "never"]).0, 0);
    // The ignored fixture has only a warning left.
    assert_eq!(run(&["tests/fixtures/ignored", "--fail-on", "error"]).0, 0);
    assert_eq!(
        run(&["check", "tests/fixtures/ignored", "--fail-on", "warning"]).0,
        1
    );
}

#[test]
fn usage_errors_exit_two() {
    assert_eq!(run(&["does/not/exist"]).0, 2);
    assert_eq!(run(&["--ignore", "PCG999", "tests/fixtures/secure"]).0, 2);
    assert_eq!(run(&["--fail-on", "loud", "tests/fixtures/secure"]).0, 2);
    assert_eq!(run(&["explain", "PCG999"]).0, 2);
}

#[test]
fn json_output() {
    let (_, out, _) = run(&["tests/fixtures/insecure", "--format", "json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let f = &v[0]["findings"][0];
    assert_eq!(f["rule"], "PCG001");
    assert_eq!(f["name"], "mutable-tag-rev");
    assert_eq!(f["line"], 3);
    assert!(f["fix"].as_str().unwrap().contains("--freeze"));
}

#[test]
fn sarif_output() {
    let (_, out, _) = run(&["tests/fixtures/insecure", "--format", "sarif"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["version"], "2.1.0");
    let run0 = &v["runs"][0];
    assert_eq!(run0["tool"]["driver"]["name"], "precommitguard");
    assert_eq!(
        run0["tool"]["driver"]["rules"].as_array().unwrap().len(),
        precommitguard::rules::RULES.len()
    );
    let r = &run0["results"][0];
    assert_eq!(r["level"], "warning");
    assert_eq!(
        r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
        "tests/fixtures/insecure/.pre-commit-config.yaml"
    );
}

#[test]
fn github_annotations() {
    let (_, out, _) = run(&["tests/fixtures/insecure", "--format", "github"]);
    assert!(out.lines().any(|l| {
        l.starts_with("::error file=tests/fixtures/insecure/.pre-commit-config.yaml,line=7,")
    }));
    assert!(out.contains("title=PCG002 branch-rev::"));
}

#[test]
fn ignore_flags_and_config() {
    let (code, out, _) = run(&[
        "tests/fixtures/ignored",
        "--ignore",
        "PCG001",
        "--color",
        "never",
    ]);
    assert_eq!(code, 0, "{out}");
    let (code, _, _) = run(&[
        "tests/fixtures",
        "--ignore-path",
        "tests/fixtures/{insecure,hygiene,manifest,broken,ignored}/**",
    ]);
    assert_eq!(code, 0);
    let (code, out, _) = run(&[
        "tests/fixtures/insecure",
        "-c",
        "tests/fixtures/strict.yaml",
        "--color",
        "never",
    ]);
    assert_eq!(code, 1);
    assert!(!out.contains("PCG004"));
}

#[test]
fn manifest_flag_and_discovery() {
    let (_, out, _) = run(&["tests/fixtures/manifest", "--format", "json"]);
    assert!(out.contains("manifest hook `noentry`"));
    let (_, out, _) = run(&[
        "tests/fixtures/secure/.pre-commit-config.yaml",
        "--manifest",
        "--format",
        "json",
    ]);
    assert!(out.contains("must be a list of hooks"));
}

#[test]
fn rules_and_explain() {
    let (code, out, _) = run(&["rules"]);
    assert_eq!(code, 0);
    assert_eq!(out.lines().count(), precommitguard::rules::RULES.len());
    let (code, out, _) = run(&["explain", "pcg002"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("PCG002 branch-rev"));
    assert!(out.contains("Fix:"));
}

#[test]
fn colors() {
    let (_, out, _) = run(&["tests/fixtures/insecure", "--color", "always"]);
    assert!(out.contains("\x1b["));
    let (_, out, _) = run(&["tests/fixtures/insecure", "--color", "never"]);
    assert!(!out.contains("\x1b["));
}

#[test]
fn repository_self_check_is_clean() {
    // Uses the repository's own .precommitguard.yaml (fixtures ignored, fail_on: info).
    let o = bin().arg("--color").arg("never").output().unwrap();
    let out = String::from_utf8_lossy(&o.stdout);
    assert_eq!(o.status.code(), Some(0), "{out}");
    assert!(out.contains("in 2 file(s)"), "{out}");
}

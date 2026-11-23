//! Output formatters: human, json, sarif, github.

use crate::rules::{RULES, rule};
use crate::{FileReport, Severity};
use serde_json::json;

pub const DOCS: &str = "https://github.com/hamarshehmhmd/precommitguard/blob/main/docs/rules.md";

fn paint(s: &str, code: &str, color: bool) -> String {
    if color {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn human(reports: &[FileReport], color: bool) -> String {
    let mut out = String::new();
    let mut total = 0;
    for r in reports {
        for f in &r.findings {
            total += 1;
            let loc = format!("{}:{}:{}", r.path, f.line, f.column);
            let sev = format!("{:<7}", f.severity.as_str());
            let sev = match f.severity {
                Severity::Error => paint(&sev, "1;31", color),
                Severity::Warning => paint(&sev, "1;33", color),
                Severity::Info => paint(&sev, "1;36", color),
            };
            out.push_str(&format!(
                "{}  {sev}  {}  {}\n",
                paint(&loc, "1", color),
                paint(f.rule, "2", color),
                f.message
            ));
        }
    }
    let count = |s: Severity| {
        reports
            .iter()
            .flat_map(|r| &r.findings)
            .filter(|f| f.severity == s)
            .count()
    };
    if total > 0 {
        out.push('\n');
    }
    out.push_str(&format!(
        "{} problem(s) in {} file(s): {} error, {} warning, {} info\n",
        total,
        reports.len(),
        count(Severity::Error),
        count(Severity::Warning),
        count(Severity::Info)
    ));
    out
}

pub fn json(reports: &[FileReport]) -> String {
    let files: Vec<_> = reports
        .iter()
        .map(|r| {
            json!({
                "path": r.path,
                "findings": r.findings.iter().map(|f| json!({
                    "rule": f.rule,
                    "name": rule(f.rule).map(|x| x.name),
                    "severity": f.severity,
                    "message": f.message,
                    "line": f.line,
                    "column": f.column,
                    "fix": rule(f.rule).map(|x| x.fix),
                })).collect::<Vec<_>>()
            })
        })
        .collect();
    serde_json::to_string_pretty(&files).expect("serializable")
}

fn level(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "note",
    }
}

pub fn sarif(reports: &[FileReport]) -> String {
    let rules: Vec<_> = RULES
        .iter()
        .map(|r| {
            let mut tags = vec!["pre-commit", r.category];
            if r.category == "supply-chain" {
                tags.push("security");
            }
            json!({
                "id": r.id,
                "name": r.name,
                "shortDescription": { "text": r.description },
                "fullDescription": { "text": r.description },
                "help": { "text": r.fix, "markdown": format!("**Fix:** {}", r.fix) },
                "helpUri": format!("{DOCS}#{}", r.id.to_lowercase()),
                "defaultConfiguration": { "level": level(r.severity) },
                "properties": { "tags": tags }
            })
        })
        .collect();
    let results: Vec<_> = reports
        .iter()
        .flat_map(|r| {
            r.findings.iter().map(move |f| {
                json!({
                    "ruleId": f.rule,
                    "ruleIndex": RULES.iter().position(|x| x.id == f.rule).unwrap_or(0),
                    "level": level(f.severity),
                    "message": { "text": f.message },
                    "locations": [{
                        "physicalLocation": {
                            "artifactLocation": { "uri": r.path.replace('\\', "/").trim_start_matches("./") },
                            "region": { "startLine": f.line.max(1), "startColumn": f.column.max(1) }
                        }
                    }]
                })
            })
        })
        .collect();
    let doc = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": { "driver": {
                "name": "precommitguard",
                "version": env!("CARGO_PKG_VERSION"),
                "semanticVersion": env!("CARGO_PKG_VERSION"),
                "informationUri": "https://github.com/hamarshehmhmd/precommitguard",
                "rules": rules
            }},
            "results": results
        }]
    });
    serde_json::to_string_pretty(&doc).expect("serializable")
}

fn escape_data(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

fn escape_prop(s: &str) -> String {
    escape_data(s).replace(':', "%3A").replace(',', "%2C")
}

pub fn github(reports: &[FileReport]) -> String {
    let mut out = String::new();
    for r in reports {
        for f in &r.findings {
            let kind = match f.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "notice",
            };
            let name = rule(f.rule).map(|x| x.name).unwrap_or("");
            out.push_str(&format!(
                "::{kind} file={},line={},col={},title={}::{}\n",
                escape_prop(&r.path.replace('\\', "/")),
                f.line.max(1),
                f.column.max(1),
                escape_prop(&format!("{} {name}", f.rule)),
                escape_data(&f.message)
            ));
        }
    }
    out
}

/// `precommitguard rules` table.
pub fn rules_table() -> String {
    let mut out = String::new();
    for r in RULES {
        out.push_str(&format!(
            "{}  {:<8} {:<13} {:<32} {}\n",
            r.id,
            r.severity.as_str(),
            r.category,
            r.name,
            r.description
        ));
    }
    out
}

/// `precommitguard explain <ID>` text.
pub fn explain(id: &str) -> Option<String> {
    let r = rule(id)?;
    Some(format!(
        "{} {} ({}, {})\n\n{}\n\nFix: {}\n\nDocs: {DOCS}#{}\nSuppress: add `# precommitguard: ignore {}` on the line, or `ignore: [{}]` in .precommitguard.yaml\n",
        r.id,
        r.name,
        r.severity.as_str(),
        r.category,
        r.description,
        r.fix,
        r.id.to_lowercase(),
        r.id,
        r.id
    ))
}

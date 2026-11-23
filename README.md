# precommitguard

[![CI](https://github.com/hamarshehmhmd/precommitguard/actions/workflows/ci.yml/badge.svg)](https://github.com/hamarshehmhmd/precommitguard/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/hamarshehmhmd/precommitguard)](https://github.com/hamarshehmhmd/precommitguard/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![SARIF](https://img.shields.io/badge/output-SARIF%202.1.0-informational)](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html)

**A fast, offline supply-chain and hygiene linter for `.pre-commit-config.yaml` and `.pre-commit-hooks.yaml`.**

Every hook in a pre-commit config is third-party code that runs on every developer laptop and in CI. Most configs pin hooks to *tags* (which can be moved), some to *branches*, and `repo: local` hooks sometimes `curl | sh` an installer. `pre-commit` itself only logs a warning for some of this, and nothing checks it in CI.

precommitguard is a single static binary that catches these problems in milliseconds, without network access or a Python install, and reports them as human-readable text, JSON, **SARIF** (GitHub code scanning) or **GitHub annotations**.

```console
$ precommitguard
.pre-commit-config.yaml:3:10   warning  PCG001  `rev: v4.6.0` is a mutable tag, not a commit SHA
.pre-commit-config.yaml:7:10   error    PCG002  `rev: main` looks like a branch (mutable reference)
.pre-commit-config.yaml:10:11  error    PCG003  `http://github.com/PyCQA/flake8` uses an unencrypted, unauthenticated transport
.pre-commit-config.yaml:14:35  warning  PCG009  additional dependency `flake8-bugbear` has no version pin; use `name==x.y.z`
.pre-commit-config.yaml:19:11  warning  PCG011  `https://github.com/pre-commit/mirrors-isort` is archived; use https://github.com/PyCQA/isort
.pre-commit-config.yaml:27:16  error    PCG007  entry downloads and executes remote code

6 problem(s) in 1 file(s): 3 error, 3 warning, 0 info
```

## Features

- **26 rules** with stable IDs ([docs/rules.md](docs/rules.md)): mutable tag/branch revs, `# frozen:` comment checks, insecure transports, untrusted hosts, `curl | sh` entries, unpinned `additional_dependencies` and docker images, archived/moved hook repos, plus schema checks (unknown keys with typo suggestions, deprecated stages and languages, `sha:`, invalid regexes, missing required fields, duplicates).
- Understands **`pre-commit autoupdate --freeze`**: SHA revs with `# frozen: vX.Y.Z` comments are the clean state.
- Lints hook **manifests** (`.pre-commit-hooks.yaml`) for hook authors too.
- Exact `file:line:column` for every finding, including YAML syntax errors.
- Output: `human` (colored), `json`, `sarif` (2.1.0), `github` (workflow annotations).
- Config file (`.precommitguard.yaml`) and inline `# precommitguard: ignore PCG001` comments.
- Works with [prek](https://github.com/j178/prek) configs (prek-only keys accepted; `strict_upstream: true` flags them).
- Offline and fast: no network, no Python; one binary for Linux, macOS and Windows.

## Install

**Prebuilt binaries** (Linux x86_64/aarch64 gnu + x86_64 musl, macOS x86_64/arm64, Windows x86_64) are on the [releases page](https://github.com/hamarshehmhmd/precommitguard/releases), with `SHA256SUMS`:

```sh
# Linux x86_64 example
curl -fsSLO https://github.com/hamarshehmhmd/precommitguard/releases/download/v0.1.0/precommitguard-v0.1.0-x86_64-unknown-linux-musl.tar.gz
tar xzf precommitguard-v0.1.0-x86_64-unknown-linux-musl.tar.gz
./precommitguard-v0.1.0-x86_64-unknown-linux-musl/precommitguard --version
```

**From source** (Rust 1.88+):

```sh
cargo install --git https://github.com/hamarshehmhmd/precommitguard --tag v0.1.0
```

## Quickstart

```sh
precommitguard                          # lint ./.pre-commit-config.yaml (and any nested ones)
precommitguard path/to/.pre-commit-config.yaml
precommitguard --format sarif > precommitguard.sarif
precommitguard --fail-on error          # only errors fail the run
precommitguard rules                    # list all rules
precommitguard explain PCG001           # rule documentation and fix
```

The usual fix for most supply-chain findings is one command:

```sh
pre-commit autoupdate --freeze   # pins every repo to a commit SHA and adds "# frozen: vX.Y.Z"
```

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | No findings at or above `--fail-on` (default `warning`) |
| `1` | Findings at or above `--fail-on` (YAML syntax errors are reported as PCG012 errors) |
| `2` | Usage error, invalid config, missing path or unreadable file |

## GitHub Action

```yaml
name: pre-commit config lint
on: [push, pull_request]
permissions:
  contents: read
  security-events: write   # only needed for SARIF upload
jobs:
  precommitguard:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: hamarshehmhmd/precommitguard@v0.1.0
        with:
          fail-on: warning
          sarif-file: precommitguard.sarif   # optional
      - uses: github/codeql-action/upload-sarif@v4
        if: always()
        with:
          sarif_file: precommitguard.sarif
```

Inputs: `paths` (default `.`), `format` (default `github` annotations), `fail-on` (default `warning`), `version` (default `latest`), `sarif-file` (optional), `upload-sarif` (`true` to upload SARIF to code scanning from the action itself).

## pre-commit hook

precommitguard can lint your pre-commit config from pre-commit itself (it is built with `language: rust`, so the first run compiles it):

```yaml
repos:
  - repo: https://github.com/hamarshehmhmd/precommitguard
    rev: v0.1.0  # better: run `pre-commit autoupdate --freeze` to pin the SHA
    hooks:
      - id: precommitguard
```

## Configuration

`.precommitguard.yaml` in the working directory (or `--config path`; `--no-config` to skip it):

```yaml
ignore: [PCG005]                 # disable rules
fail_on: warning                 # info | warning | error | never
severity:                        # per-rule overrides
  PCG001: error
ignore_paths:                    # globs of files to skip
  - "examples/**"
allowed_hosts:                   # replaces the default list for PCG004
  - github.com
  - git.example.com
allow_tags:                      # repos where tag revs are acceptable (PCG001), * wildcard
  - https://github.com/my-org/*
strict_upstream: false           # true: flag prek-only keys
```

Inline suppressions:

```yaml
repos:
  # precommitguard: ignore PCG001      <- applies to the whole next node (this repo entry)
  - repo: https://github.com/my-org/hooks
    rev: v1.2.0
    hooks:
      - id: lint
        additional_dependencies: [requests]  # precommitguard: ignore PCG009
# precommitguard: ignore-file             <- anywhere: skip this file
```

## Rules

| ID | Name | Severity | Category |
|----|------|----------|----------|
| [PCG001](docs/rules.md#pcg001) | `mutable-tag-rev` | warning | supply-chain |
| [PCG002](docs/rules.md#pcg002) | `branch-rev` | error | supply-chain |
| [PCG003](docs/rules.md#pcg003) | `insecure-repo-url` | error | supply-chain |
| [PCG004](docs/rules.md#pcg004) | `untrusted-repo-host` | warning | supply-chain |
| [PCG005](docs/rules.md#pcg005) | `sha-without-frozen-comment` | info | supply-chain |
| [PCG006](docs/rules.md#pcg006) | `frozen-comment-on-mutable-rev` | warning | supply-chain |
| [PCG007](docs/rules.md#pcg007) | `remote-code-in-entry` | error | supply-chain |
| [PCG008](docs/rules.md#pcg008) | `suspicious-unsupported-entry` | warning | supply-chain |
| [PCG009](docs/rules.md#pcg009) | `unpinned-additional-dependency` | warning | supply-chain |
| [PCG010](docs/rules.md#pcg010) | `unpinned-docker-image` | warning | supply-chain |
| [PCG011](docs/rules.md#pcg011) | `archived-or-moved-repo` | warning | supply-chain |
| [PCG012](docs/rules.md#pcg012) | `yaml-syntax-error` | error | correctness |
| [PCG013](docs/rules.md#pcg013) | `unknown-key` | warning | correctness |
| [PCG014](docs/rules.md#pcg014) | `duplicate-repo` | warning | hygiene |
| [PCG015](docs/rules.md#pcg015) | `duplicate-hook` | info | hygiene |
| [PCG016](docs/rules.md#pcg016) | `deprecated-stage` | warning | correctness |
| [PCG017](docs/rules.md#pcg017) | `invalid-stage` | error | correctness |
| [PCG018](docs/rules.md#pcg018) | `invalid-language` | error | correctness |
| [PCG019](docs/rules.md#pcg019) | `legacy-language-alias` | info | hygiene |
| [PCG020](docs/rules.md#pcg020) | `deprecated-sha-key` | error | correctness |
| [PCG021](docs/rules.md#pcg021) | `invalid-regex` | error | correctness |
| [PCG022](docs/rules.md#pcg022) | `glob-like-regex` | warning | correctness |
| [PCG023](docs/rules.md#pcg023) | `schema-error` | error | correctness |
| [PCG024](docs/rules.md#pcg024) | `empty-hooks` | warning | hygiene |
| [PCG025](docs/rules.md#pcg025) | `always-run-with-filters` | info | hygiene |
| [PCG026](docs/rules.md#pcg026) | `minimum-version-missing` | info | hygiene |

Full descriptions, examples and fixes: **[docs/rules.md](docs/rules.md)**.

## How it compares

| | precommitguard | `pre-commit validate-config` | [pre-commit-ci-config](https://github.com/pre-commit-ci/pre-commit-ci-config) | [pre-commit-sort](https://github.com/nim65s/pre-commit-sort) | zizmor / actionlint |
|---|:-:|:-:|:-:|:-:|:-:|
| Lints `.pre-commit-config.yaml` | ✅ | ✅ (schema only) | `ci:` block only | sorts only | ❌ (GitHub workflows) |
| Mutable tag / branch `rev` | ✅ PCG001/002 | branch warning only | ❌ | ❌ | n/a |
| `# frozen:` comment awareness | ✅ | ❌ | ❌ | ❌ | n/a |
| `curl \| sh` / risky `entry` | ✅ | ❌ | ❌ | ❌ | n/a |
| Unpinned `additional_dependencies` / images | ✅ | ❌ | ❌ | ❌ | n/a |
| Archived / moved hook repos | ✅ | ❌ | ❌ | ❌ | n/a |
| Typos with suggestions, deprecated stages/languages | ✅ | warnings in logs | ❌ | ❌ | n/a |
| SARIF / GitHub annotations | ✅ | ❌ | ❌ | ❌ | ✅ |
| Needs Python / network | No | Python | Python | Python | No |

precommitguard complements zizmor/actionlint: they secure your GitHub workflows, precommitguard secures the hooks those workflows (and your laptop) run.

## Contributing

New rules, false-positive reports and moved-repo updates are very welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) and the [good first issues](https://github.com/hamarshehmhmd/precommitguard/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22). Please follow the [Code of Conduct](CODE_OF_CONDUCT.md); report vulnerabilities as described in [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE) © Mohammad Hamarsheh

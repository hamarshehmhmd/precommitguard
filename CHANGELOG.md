# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-11-23

### Added
- 26 rules (PCG001–PCG026) for `.pre-commit-config.yaml` and `.pre-commit-hooks.yaml`:
  - supply chain: mutable tag revs, branch revs, `http://`/`git://` repos, untrusted hosts, `# frozen:` comment checks, `curl | sh`-style entries, suspicious `unsupported`/`system` entries, unpinned `additional_dependencies` (python, node, golang, rust), docker images without digests, archived or moved hook repositories;
  - correctness and hygiene: YAML syntax errors with line/column, unknown keys with typo suggestions, duplicate repos and hooks, deprecated and invalid stages, removed/unknown languages, legacy `system`/`script` aliases, `sha:`, invalid or glob-like `files`/`exclude` regexes, schema errors (missing `entry`, numeric `rev`, unknown `meta` hooks), empty hooks, `always_run` with filters, missing `minimum_pre_commit_version`.
- YAML parser layer with exact positions, anchor/alias resolution and comment scanning.
- Output formats: human (colored), JSON, SARIF 2.1.0, GitHub annotations; `rules` and `explain` subcommands.
- `.precommitguard.yaml` (ignore, fail_on, severity, ignore_paths, allowed_hosts, allow_tags, strict_upstream) and inline `# precommitguard: ignore` / `ignore-file` comments.
- prek compatibility (prek-only keys accepted unless `strict_upstream: true`).
- Composite GitHub Action with checksum verification and optional SARIF upload; `.pre-commit-hooks.yaml` so precommitguard runs as a pre-commit hook.
- Prebuilt binaries for Linux (x86_64 gnu/musl, aarch64), macOS (x86_64, arm64) and Windows (x86_64) with `SHA256SUMS`.

[0.1.0]: https://github.com/hamarshehmhmd/precommitguard/releases/tag/v0.1.0

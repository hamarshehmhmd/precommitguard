# Contributing

Thanks for helping! Issues and PRs are welcome, especially new rules, false-positive reports and updates to the archived/moved repository list.

## Development

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -- --no-config tests/fixtures      # try it on the fixtures
cargo run -- explain PCG001
```

All three checks must pass; CI runs them on Linux, macOS and Windows, checks the MSRV (Rust 1.88) and lints this repository's own `.pre-commit-config.yaml` with the freshly built binary.

## Project layout

| Path | What |
|------|------|
| `src/yaml.rs` | YAML tree with line/column positions (on `yaml-rust2` events) and a comment scanner |
| `src/rules.rs` | Rule catalogue (`RULES`) and all checks |
| `src/data.rs` | pre-commit schema constants and the curated archived/moved repo list |
| `src/lib.rs` | `lint_str`, inline ignores, file discovery |
| `src/config.rs` | `.precommitguard.yaml` |
| `src/output.rs` | human / json / sarif / github formatters, `rules` and `explain` |
| `tests/fixtures/` | sample configs: `secure/` (must stay clean), `insecure/`, `hygiene/`, `manifest/`, ignores, broken YAML |

## Adding a rule

1. Open a "New rule" issue first if the rule is non-obvious, so we can agree on scope and severity.
2. Append an entry to `RULES` in `src/rules.rs` with the next free ID (`PCG0xx`). IDs are stable and are never reused or renumbered.
3. Implement the check. Report the position of the most specific node (the offending value or list item) so `line:column` and inline ignores are precise.
4. Make a fixture trigger it, add a focused test in `tests/lint.rs`, and keep `tests/fixtures/secure/` clean.
5. Document it in `docs/rules.md` (with bad/good examples), the README rules table and `CHANGELOG.md`. A test fails if a rule is missing from the docs or README.

Keep false positives low: when in doubt, prefer `info` severity. Base schema facts on pre-commit's [`clientlib.py`](https://github.com/pre-commit/pre-commit/blob/main/pre_commit/clientlib.py) and link the source in the PR.

### Updating the archived/moved repo list

Edit `MOVED_REPOS` in `src/data.rs`. Please include evidence in the PR (the archive banner or the redirect), e.g. `gh api repos/OWNER/NAME --jq '.full_name, .archived'`.

## Commits and PRs

- Small, focused PRs with a clear description; fill in the PR template.
- Conventional-ish commit subjects (`feat:`, `fix:`, `docs:`, `ci:`) are appreciated.
- By contributing you agree your work is licensed under the MIT license.

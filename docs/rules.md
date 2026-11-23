# precommitguard rules

Every rule has a stable ID that is never renumbered or reused. Severities are defaults; override them with `severity:` in `.precommitguard.yaml`.

Suppress a finding with a YAML comment on the same line (`# precommitguard: ignore PCG001`), on the line above a node to cover the whole node (e.g. a full `- repo:` entry), or `# precommitguard: ignore-file` anywhere in the file. Disable a rule globally with `ignore: [PCG001]` or `--ignore PCG001`.

| ID | Name | Severity | Category |
|----|------|----------|----------|
| [PCG001](#pcg001) | `mutable-tag-rev` | warning | supply-chain |
| [PCG002](#pcg002) | `branch-rev` | error | supply-chain |
| [PCG003](#pcg003) | `insecure-repo-url` | error | supply-chain |
| [PCG004](#pcg004) | `untrusted-repo-host` | warning | supply-chain |
| [PCG005](#pcg005) | `sha-without-frozen-comment` | info | supply-chain |
| [PCG006](#pcg006) | `frozen-comment-on-mutable-rev` | warning | supply-chain |
| [PCG007](#pcg007) | `remote-code-in-entry` | error | supply-chain |
| [PCG008](#pcg008) | `suspicious-unsupported-entry` | warning | supply-chain |
| [PCG009](#pcg009) | `unpinned-additional-dependency` | warning | supply-chain |
| [PCG010](#pcg010) | `unpinned-docker-image` | warning | supply-chain |
| [PCG011](#pcg011) | `archived-or-moved-repo` | warning | supply-chain |
| [PCG012](#pcg012) | `yaml-syntax-error` | error | correctness |
| [PCG013](#pcg013) | `unknown-key` | warning | correctness |
| [PCG014](#pcg014) | `duplicate-repo` | warning | hygiene |
| [PCG015](#pcg015) | `duplicate-hook` | info | hygiene |
| [PCG016](#pcg016) | `deprecated-stage` | warning | correctness |
| [PCG017](#pcg017) | `invalid-stage` | error | correctness |
| [PCG018](#pcg018) | `invalid-language` | error | correctness |
| [PCG019](#pcg019) | `legacy-language-alias` | info | hygiene |
| [PCG020](#pcg020) | `deprecated-sha-key` | error | correctness |
| [PCG021](#pcg021) | `invalid-regex` | error | correctness |
| [PCG022](#pcg022) | `glob-like-regex` | warning | correctness |
| [PCG023](#pcg023) | `schema-error` | error | correctness |
| [PCG024](#pcg024) | `empty-hooks` | warning | hygiene |
| [PCG025](#pcg025) | `always-run-with-filters` | info | hygiene |
| [PCG026](#pcg026) | `minimum-version-missing` | info | hygiene |

## PCG001

**`mutable-tag-rev`** · warning · supply-chain

`rev` is a tag (or abbreviated SHA) rather than a full 40-character commit SHA; tags can be moved or re-pushed.

Bad:

```yaml
rev: v6.0.0
```

Good:

```yaml
rev: 3e8a8703264a2f4a69428a0aa4dcb512790b2c8c  # frozen: v6.0.0
```

**Fix:** Pin to the full commit SHA with `pre-commit autoupdate --freeze`, which also writes a `# frozen: vX.Y.Z` comment

## PCG002

**`branch-rev`** · error · supply-chain

`rev` is a branch-like moving reference (main, master, HEAD, ...); pre-commit never updates it after first install and anyone who can push to the branch controls your hook.

Bad:

```yaml
rev: main
```

Good:

```yaml
rev: 3e8a8703264a2f4a69428a0aa4dcb512790b2c8c  # frozen: v6.0.0
```

**Fix:** Run `pre-commit autoupdate --freeze` to pin a release commit

## PCG003

**`insecure-repo-url`** · error · supply-chain

`repo` uses an unauthenticated transport (`http://` or `git://`), so hook code can be tampered with in transit.

Bad:

```yaml
- repo: http://github.com/PyCQA/flake8
```

Good:

```yaml
- repo: https://github.com/PyCQA/flake8
```

**Fix:** Use the `https://` URL of the repository

## PCG004

**`untrusted-repo-host`** · warning · supply-chain

`repo` is hosted outside the allowed hosts (default: github.com, gitlab.com, bitbucket.org, codeberg.org).

Bad:

```yaml
- repo: https://git.example.net/acme/hooks
```

Good:

```yaml
# .precommitguard.yaml
allowed_hosts: [github.com, git.example.net]
```

**Fix:** Move the hook to a trusted host, or add the host to `allowed_hosts` in .precommitguard.yaml

## PCG005

**`sha-without-frozen-comment`** · info · supply-chain

`rev` is pinned to a SHA but has no `# frozen: vX.Y.Z` comment, so reviewers cannot tell which release it is.

Bad:

```yaml
rev: 3e8a8703264a2f4a69428a0aa4dcb512790b2c8c
```

Good:

```yaml
rev: 3e8a8703264a2f4a69428a0aa4dcb512790b2c8c  # frozen: v6.0.0
```

**Fix:** Re-run `pre-commit autoupdate --freeze` or add `# frozen: <tag>` after the SHA

## PCG006

**`frozen-comment-on-mutable-rev`** · warning · supply-chain

A `# frozen:` comment is attached to a `rev` that is not a full SHA, which falsely suggests the hook is pinned.

Bad:

```yaml
rev: v6.0.0  # frozen: v6.0.0
```

Good:

```yaml
rev: 3e8a8703264a2f4a69428a0aa4dcb512790b2c8c  # frozen: v6.0.0
```

**Fix:** Replace the rev with the commit SHA (`pre-commit autoupdate --freeze`) or drop the comment

## PCG007

**`remote-code-in-entry`** · error · supply-chain

Hook `entry` downloads code and executes it (e.g. `curl ... | sh`, `iwr ... | iex`), bypassing pinning entirely.

Bad:

```yaml
entry: bash -c "curl -fsSL https://example.com/install.sh | bash"
```

Good:

```yaml
entry: ./scripts/check.sh   # vendored and reviewed
```

**Fix:** Vendor the script into the repository or install the tool through a pinned hook repo / additional_dependencies

## PCG008

**`suspicious-unsupported-entry`** · warning · supply-chain

An `unsupported`/`system`/`script` hook entry uses risky shell constructs (network fetches, eval, base64 decoding, sudo, chmod 777, /dev/tcp, rm -rf /).

Bad:

```yaml
language: unsupported
entry: sh -c 'echo $PAYLOAD | base64 -d | sh'
```

Good:

```yaml
language: unsupported
entry: ./scripts/check.sh
```

**Fix:** Move the logic into a reviewed script in the repository, or into a pinned hook repository

## PCG009

**`unpinned-additional-dependency`** · warning · supply-chain

An `additional_dependencies` entry has no exact version (python `==`, node `@x.y.z`, go `@vX.Y.Z`), so every fresh install may pull different code.

Bad:

```yaml
additional_dependencies: [types-requests, "eslint@^9"]
```

Good:

```yaml
additional_dependencies: [types-requests==2.32.0.20240914]
```

**Fix:** Pin exact versions, e.g. `types-requests==2.32.0.20240914` or `eslint@9.14.0`

## PCG010

**`unpinned-docker-image`** · warning · supply-chain

A `docker_image` hook runs an image by tag rather than by `@sha256:` digest.

Bad:

```yaml
language: docker_image
entry: koalaman/shellcheck:v0.10.0
```

Good:

```yaml
language: docker_image
entry: koalaman/shellcheck:v0.10.0@sha256:<digest>
```

**Fix:** Reference the image by digest, e.g. `koalaman/shellcheck:v0.10.0@sha256:...`

## PCG011

**`archived-or-moved-repo`** · warning · supply-chain

`repo` points at a hook repository that is archived or has moved (only a redirect keeps it working; a re-registered name could serve different code).

Bad:

```yaml
- repo: https://github.com/pre-commit/mirrors-isort
```

Good:

```yaml
- repo: https://github.com/PyCQA/isort
```

**Fix:** Switch to the maintained location shown in the message

## PCG012

**`yaml-syntax-error`** · error · correctness

The file is not valid YAML.

Bad:

```yaml
repos:
  - repo: x
   hooks: [
```

Good:

```yaml
repos:
  - repo: x
    hooks: []
```

**Fix:** Fix the syntax error at the reported line and column

## PCG013

**`unknown-key`** · warning · correctness

Unknown or misplaced key (usually a typo such as `additional_dependency` or `exlude`); pre-commit only warns and ignores it.

Bad:

```yaml
- id: check-json
  exlude: ^vendor/
```

Good:

```yaml
- id: check-json
  exclude: ^vendor/
```

**Fix:** Fix the spelling or remove the key; see https://pre-commit.com/#plugins for valid keys

## PCG014

**`duplicate-repo`** · warning · hygiene

The same remote repository appears more than once, so it is installed and pinned twice and the revs can drift apart.

Bad:

```yaml
- repo: https://github.com/pre-commit/pre-commit-hooks
  ...
- repo: https://github.com/pre-commit/pre-commit-hooks
```

Good:

```yaml
# one entry with all hooks
```

**Fix:** Merge the hooks into a single `repos:` entry

## PCG015

**`duplicate-hook`** · info · hygiene

The same hook id is listed twice in one repository with the same alias and name, so the two runs cannot be told apart in output or with `pre-commit run <alias>`.

Bad:

```yaml
- id: check-json
- id: check-json
```

Good:

```yaml
- id: check-json
- id: check-json
  alias: check-json-strict
  name: check json (strict)
```

**Fix:** Merge the entries, or give each one a distinct `alias` and `name`

## PCG016

**`deprecated-stage`** · warning · correctness

Deprecated stage name (`commit`, `push`, `merge-commit`; deprecated in pre-commit 3.2.0).

Bad:

```yaml
stages: [commit, push]
```

Good:

```yaml
stages: [pre-commit, pre-push]
```

**Fix:** Use `pre-commit`, `pre-push`, `pre-merge-commit`, or run `pre-commit migrate-config`

## PCG017

**`invalid-stage`** · error · correctness

Unknown stage or hook type in `stages`, `default_stages` or `default_install_hook_types`.

Bad:

```yaml
stages: [precommit]
```

Good:

```yaml
stages: [pre-commit]
```

**Fix:** Use one of: commit-msg, post-checkout, post-commit, post-merge, post-rewrite, pre-commit, pre-merge-commit, pre-push, pre-rebase, prepare-commit-msg, manual

## PCG018

**`invalid-language`** · error · correctness

Unknown or removed `language` (e.g. `python_venv` was removed in pre-commit 4.0.0, `pcre` in 2.0.0).

Bad:

```yaml
language: python_venv
```

Good:

```yaml
language: python
```

**Fix:** Use a supported language such as `python`, `pygrep`, `node`, `unsupported`

## PCG019

**`legacy-language-alias`** · info · hygiene

`language: system` / `language: script` are legacy aliases of `unsupported` / `unsupported_script` (pre-commit 4.4.0) and are slated for deprecation.

Bad:

```yaml
language: system
```

Good:

```yaml
language: unsupported
```

**Fix:** Rename to `unsupported` / `unsupported_script` and set `minimum_pre_commit_version: '4.4.0'`

## PCG020

**`deprecated-sha-key`** · error · correctness

`sha:` was replaced by `rev:` in pre-commit 1.7.0 and is no longer accepted.

Bad:

```yaml
sha: v1.0.0
```

Good:

```yaml
rev: <40-char sha>  # frozen: v1.0.0
```

**Fix:** Rename `sha:` to `rev:` (or run `pre-commit migrate-config`)

## PCG021

**`invalid-regex`** · error · correctness

`files` / `exclude` is not a valid regular expression (they are Python regexes, not globs).

Bad:

```yaml
files: '*.py'
```

Good:

```yaml
files: \.py$
```

**Fix:** Write a regex, e.g. `\.py$` instead of `*.py`

## PCG022

**`glob-like-regex`** · warning · correctness

`files` / `exclude` looks like a glob (`/*`, `**`) or uses `[\\/]`; pre-commit matches regexes against forward-slash paths.

Bad:

```yaml
files: docs/*
```

Good:

```yaml
files: ^docs/
```

**Fix:** Use regex syntax: `^docs/` instead of `docs/*`, and `/` instead of `[\\/]`

## PCG023

**`schema-error`** · error · correctness

A required key is missing or a value has the wrong type (e.g. `repo: local` hook without `entry`, `rev: 1.0` parsed as a number, unknown `meta` hook).

Bad:

```yaml
- repo: local
  hooks:
    - id: mypy
      name: mypy
      language: python
```

Good:

```yaml
- repo: local
  hooks:
    - id: mypy
      name: mypy
      entry: mypy
      language: python
```

**Fix:** Add the missing key or quote the value; local hooks need id, name, entry and language

## PCG024

**`empty-hooks`** · warning · hygiene

A repository has no hooks, or the config has no repos, so nothing runs.

Bad:

```yaml
- repo: https://github.com/asottile/pyupgrade
  rev: <sha>
  hooks: []
```

Good:

```yaml
# remove the entry or add hooks
```

**Fix:** Add hooks or remove the entry

## PCG025

**`always-run-with-filters`** · info · hygiene

`always_run: true` is combined with `files`/`types` filters: the hook runs on every commit anyway and the filters only narrow the filenames passed.

Bad:

```yaml
always_run: true
files: \.py$
```

Good:

```yaml
files: \.py$
```

**Fix:** Drop the filters, or drop `always_run` and let the filters decide when the hook runs

## PCG026

**`minimum-version-missing`** · info · hygiene

The config uses features that need a newer pre-commit (e.g. `unsupported` language 4.4.0, `pre-commit` stage names 3.2.0) without declaring `minimum_pre_commit_version`.

Bad:

```yaml
repos:
  - repo: local
    hooks:
      - id: x
        language: unsupported
```

Good:

```yaml
minimum_pre_commit_version: "4.4.0"
repos: ...
```

**Fix:** Add `minimum_pre_commit_version: '<version>'` so older installs fail with a clear message

## Notes on regex compatibility

pre-commit evaluates `files` / `exclude` with Python's `re` module. precommitguard compiles them with the Rust [`fancy-regex`](https://crates.io/crates/fancy-regex) engine, which supports look-around, backreferences, named groups `(?P<name>...)` and inline flags such as `(?x)`; Python's `\Z` is translated to `\z`. A small number of exotic Python-only constructs may be reported by PCG021 even though Python accepts them. If you hit one, suppress it inline and please open an issue.

## prek compatibility

Keys implemented only by [prek](https://github.com/j178/prek) (`priority`, `env`, `shell`, `groups`, `minimum_prek_version`, `default_env`, `priorities`, `orphan`, `update`, `repo: builtin`, glob mappings for `files`/`exclude`) are accepted by default. Set `strict_upstream: true` in `.precommitguard.yaml` to report them under PCG013 when your config must also work with upstream pre-commit.

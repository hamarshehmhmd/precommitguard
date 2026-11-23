# Security Policy

## Supported versions

The latest release receives fixes.

## Reporting a vulnerability

Please report privately via GitHub: **Security → Report a vulnerability** on this repository. Do not open a public issue. You should get a response within 7 days.

False negatives (a risky pre-commit configuration that precommitguard misses) are welcome as normal issues using the "New rule" template.

## Verifying downloads

Every release ships a `SHA256SUMS` file. The composite GitHub Action verifies the downloaded archive against it before running.

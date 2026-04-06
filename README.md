# potto

[![CI](https://github.com/iamkorun/potto/actions/workflows/ci.yml/badge.svg)](https://github.com/iamkorun/potto/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/potto)](https://crates.io/crates/potto)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/iamkorun/potto?style=social)](https://github.com/iamkorun/potto)
  <a href="https://buymeacoffee.com/iamkorun"><img src="https://img.shields.io/badge/Buy%20Me%20a%20Coffee-ffdd00?logo=buy-me-a-coffee&logoColor=black" alt="Buy Me a Coffee"></a>

**Ferret out missing .env keys before they bite you in production.**

potto is a small, meticulous CLI tool — named after the nocturnal primate that never misses a thing — that keeps your `.env` and `.env.example` files in sync. Catch missing keys locally, in CI, or as a pre-commit hook.

<!-- TODO: Add demo GIF -->

---

## The Problem

You add a new secret to `.env`. You forget to add it to `.env.example`. A teammate pulls your branch, runs the app, and gets a cryptic crash at runtime. Or worse, it breaks in CI three minutes before a deploy.

Sound familiar?

## The Solution

potto compares your `.env` against `.env.example` in one command, shows you exactly which keys are missing with color-coded output, and can auto-fix `.env.example` for you.

```
$ potto

-> Checking .env against .env.example

FAIL  2 key(s) in .env are MISSING from .env.example  (teammates can't run the app)
  - STRIPE_SECRET_KEY
  - REDIS_URL

WARN  1 key(s) in .env.example are MISSING from .env  (you may need to set these)
  - FEATURE_FLAG_NEW_UI

Run `potto sync` to fix .env.example automatically.
```

---

## Quick Start

```bash
cargo install potto
```

Then run it in any project:

```bash
potto
```

---

## Installation

### Via Cargo (recommended)

```bash
cargo install potto
```

### From Source

```bash
git clone https://github.com/iamkorun/potto
cd potto
cargo install --path .
```

### Binary Releases

Pre-built binaries for Linux, macOS, and Windows are available on the [Releases page](https://github.com/iamkorun/potto/releases).

```bash
# Linux (x86_64)
curl -L https://github.com/iamkorun/potto/releases/latest/download/potto-linux-x86_64.tar.gz | tar xz
sudo mv potto /usr/local/bin/

# macOS (Apple Silicon)
curl -L https://github.com/iamkorun/potto/releases/latest/download/potto-darwin-aarch64.tar.gz | tar xz
sudo mv potto /usr/local/bin/
```

---

## Usage

### `potto` / `potto check`

Compare `.env` against `.env.example`. Files are auto-discovered in the current directory.

```bash
potto
# or
potto check
```

With explicit paths:

```bash
potto check --env apps/api/.env --example apps/api/.env.example
```

**Output — in sync:**

```
-> Checking .env against .env.example

OK  All 12 key(s) are in sync.
```

**Output — out of sync:**

```
-> Checking .env against .env.example

FAIL  2 key(s) in .env are MISSING from .env.example  (teammates can't run the app)
  - STRIPE_SECRET_KEY
  - REDIS_URL

WARN  1 key(s) in .env.example are MISSING from .env  (you may need to set these)
  - FEATURE_FLAG_NEW_UI

Run `potto sync` to fix .env.example automatically.
```

---

### `potto sync`

Append missing keys from `.env` to `.env.example` — values are stripped, only key names are written.

```bash
potto sync
```

```
-> Adding 2 key(s) to .env.example
  + STRIPE_SECRET_KEY=
  + REDIS_URL=

OK  2 key(s) added to .env.example
```

If `.env.example` doesn't exist yet, potto creates it from scratch.

---

### `potto compare <file-a> <file-b>`

Compare any two env files directly — useful for multi-environment setups.

```bash
potto compare .env.staging .env.production
```

```
Comparing .env.staging vs .env.production

+  1 key(s) only in .env.staging:
  + STAGING_DEBUG_MODE

+  2 key(s) only in .env.production:
  + CDN_URL
  + SENTRY_DSN
```

---

## Exit Codes

| Code | Meaning              |
|------|----------------------|
| `0`  | Files are in sync    |
| `1`  | Files are out of sync |
| `2`  | File read/write error |

Exit code `1` on out-of-sync makes potto perfect for CI gates and pre-commit hooks.

---

## Features

- **Zero config** — auto-discovers `.env` and `.env.example` in the current directory
- **Color-coded output** — FAIL (red), WARN (yellow), OK (green) at a glance
- **Auto-sync** — `potto sync` appends missing keys with blank values, never touches existing entries
- **Arbitrary file comparison** — `potto compare` works on any two env files
- **CI-friendly exit codes** — gates pipelines without extra scripting
- **Quiet mode** — `--quiet` / `-q` suppresses output, relying only on exit codes
- **Safe** — never reads or exposes secret values, only key names
- **Fast** — single Rust binary, no runtime, no dependencies to install
- **Explicit paths** — `--env` and `--example` flags override auto-discovery

---

## Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--quiet` | `-q` | Suppress all output, exit code only (great for CI) |
| `--verbose` | `-v` | Show file discovery paths and extra detail |
| `--env <path>` | | Path to .env file (overrides auto-discovery) |
| `--example <path>` | | Path to .env.example file (overrides auto-discovery) |
| `--help` | `-h` | Show help |
| `--version` | `-V` | Show version |

---

## Pre-commit Hook

Add potto to `.git/hooks/pre-commit` to block commits when `.env.example` is out of sync:

```bash
#!/bin/sh
potto check --quiet
if [ $? -ne 0 ]; then
  echo ""
  echo "Commit blocked: .env is out of sync with .env.example"
  echo "Run 'potto sync' to fix, then re-commit."
  exit 1
fi
```

Make it executable:

```bash
chmod +x .git/hooks/pre-commit
```

Or use with [pre-commit](https://pre-commit.com/) by adding to `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: local
    hooks:
      - id: potto
        name: potto — check .env sync
        entry: potto
        language: system
        pass_filenames: false
```

---

## GitHub Actions CI

Block merges when `.env.example` falls behind:

```yaml
name: CI

on: [push, pull_request]

jobs:
  env-sync:
    name: Check .env sync
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install potto
        run: cargo install potto

      - name: Check .env.example is up to date
        run: |
          cp .env.example .env   # use example as the reference env in CI
          potto check
```

For monorepos, run potto per package:

```yaml
      - name: Check env sync (api)
        run: potto check --env apps/api/.env --example apps/api/.env.example
```

---

## Contributing

Contributions are welcome.

1. Fork the repo
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Make your changes and add tests
4. Run `cargo test` — all tests must pass
5. Submit a pull request

Please follow [Conventional Commits](https://www.conventionalcommits.org/) for commit messages.

---

## License

MIT — see [LICENSE](LICENSE).

---

## Star History

<a href="https://star-history.com/#iamkorun/potto&Date">
  <img src="https://api.star-history.com/svg?repos=iamkorun/potto&type=Date" alt="Star History Chart" width="600">
</a>

---

<p align="center">
  <a href="https://buymeacoffee.com/iamkorun"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me a Coffee" width="200"></a>
</p>

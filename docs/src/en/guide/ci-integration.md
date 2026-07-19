---
title: CI Integration Guide
description: Integrate yaoxiang check and yaoxiang format into CI/CD pipelines
---

# CI Integration Guide

Integrate YaoXiang's static check and formatting tools into CI/CD pipelines to ensure code quality.

## GitHub Actions

```yaml
name: YaoXiang CI

on:
  push:
    branches: [main, dev]
  pull_request:
    branches: [main]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install YaoXiang
        run: |
          curl -fsSL https://yaoxiang.dev/install.sh | sh
          echo "$HOME/.yaoxiang/bin" >> $GITHUB_PATH

      - name: Type check
        run: yaoxiang check --color never --no-progress

      - name: Format check
        run: yaoxiang format --dry-run .
```

## GitLab CI

```yaml
yaoxiang-check:
  image: rust:latest
  script:
    - curl -fsSL https://yaoxiang.dev/install.sh | sh
    - export PATH="$HOME/.yaoxiang/bin:$PATH"
    - yaoxiang check --color never --no-progress
    - yaoxiang format --dry-run .
  rules:
    - if: $CI_MERGE_REQUEST_IID
    - if: $CI_COMMIT_BRANCH == "main"
    - if: $CI_COMMIT_BRANCH == "dev"
```

## Exit Codes

| Exit Code | Meaning | CI Behavior |
|-----------|---------|-------------|
| `0` | No errors | Pass |
| `1` | Check found errors | Fail |
| `2` | No `.yx` files found | Depends on configuration |

## Parsing JSON Output

Use `--json` to get machine-readable output:

```bash
yaoxiang check --json | jq '.error_count'
```

## Best Practices

1. **Path argument**: `yaoxiang check` checks the current directory by default, but you can also specify a path: `yaoxiang check src/`
2. **Separate check and format**: Run `check` and `format --dry-run` separately to make it easier to locate issues
3. **Use `--no-progress`**: CI environments do not need a progress bar
4. **Use `--color never`**: Avoid ANSI color codes polluting logs
5. **Cache dependencies**: Leverage CI caching mechanisms to speed up builds
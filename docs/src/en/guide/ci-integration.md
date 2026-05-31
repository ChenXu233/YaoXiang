---
title: CI Integration Guide
description: Integrate yaoxiang check and yaoxiang fmt into CI/CD pipelines
---

# CI Integration Guide

Integrate YaoXiang's static checking and formatting tools into CI/CD pipelines to ensure code quality.

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
        run: yaoxiang fmt --check
```

## GitLab CI

```yaml
yaoxiang-check:
  image: rust:latest
  script:
    - curl -fsSL https://yaoxiang.dev/install.sh | sh
    - export PATH="$HOME/.yaoxiang/bin:$PATH"
    - yaoxiang check --color never --no-progress
    - yaoxiang fmt --check
  rules:
    - if: $CI_MERGE_REQUEST_IID
    - if: $CI_COMMIT_BRANCH == "main"
    - if: $CI_COMMIT_BRANCH == "dev"
```

## Exit Codes

| Exit Code | Meaning | CI Behavior |
|-----------|---------|-------------|
| `0` | No errors | Pass |
| `1` | Errors found during check | Fail |
| `2` | No `.yx` files found | Depends on configuration |

## JSON Output Parsing

Use `--json` to get machine-readable output:

```bash
yaoxiang check --json | jq '.error_count'
```

## Best Practices

1. **Separate check and format**: Run `check` and `fmt --check` separately to make it easier to locate issues
2. **Use `--no-progress`**: CI environments don't need progress bars
3. **Use `--color never`**: Avoid ANSI color codes polluting logs
4. **Cache dependencies**: Leverage CI caching to speed up builds
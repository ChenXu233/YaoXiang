---
title: 'CI Integration Guide'
description: 'Integrate yx check and yx format into CI/CD pipelines'
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
        run: yx check --color never --no-progress

      - name: Format check
        run: yx format --dry-run .
```

## GitLab CI

```yaml
yaoxiang-check:
  image: rust:latest
  script:
    - curl -fsSL https://yaoxiang.dev/install.sh | sh
    - export PATH="$HOME/.yaoxiang/bin:$PATH"
    - yx check --color never --no-progress
    - yx format --dry-run .
  rules:
    - if: $CI_MERGE_REQUEST_IID
    - if: $CI_COMMIT_BRANCH == "main"
    - if: $CI_COMMIT_BRANCH == "dev"
```

## Exit Codes

| Exit Code | Meaning             | CI Behavior              |
| --------- | ------------------- | ------------------------ |
| `0`       | No errors           | Pass                     |
| `1`       | Check found errors  | Fail                     |
| `2`       | No `.yx` file found | Depends on configuration |

## JSON Output Parsing

Use `--json` to get machine-readable output:

```bash
yx check --json | jq '.error_count'
```

## Best Practices

1. **Path parameter**: `yx check` checks the current directory by default, you can also specify a
   path: `yx check src/`
2. **Separate check and format**: run `check` and `format --dry-run` separately for easier problem
   localization
3. **Use `--no-progress`**: CI environments don't need progress bars
4. **Use `--color never`**: avoid ANSI color codes polluting logs
5. **Cache dependencies**: leverage the CI caching mechanism to speed up builds

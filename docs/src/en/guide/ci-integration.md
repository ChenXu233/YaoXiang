---
title: 'CI Integration Guide'
description: 'Integrate yx check and yx format into your CI/CD pipeline'
---

# CI Integration Guide

Integrate YaoXiang's static checking and formatting tools into your CI/CD pipeline to ensure code
quality.

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
          curl -fsSL https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh
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
    - curl -fsSL
      https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh
    - export PATH="$HOME/.yaoxiang/bin:$PATH"
    - yx check --color never --no-progress
    - yx format --dry-run .
  rules:
    - if: $CI_MERGE_REQUEST_IID
    - if: $CI_COMMIT_BRANCH == "main"
    - if: $CI_COMMIT_BRANCH == "dev"
```

## Exit Codes

| Exit Code | Meaning                                                                        | CI Behavior       |
| --------- | ------------------------------------------------------------------------------ | ----------------- |
| `0`       | No errors                                                                      | Pass              |
| `1`       | Errors found during checking; or warnings present when using `--deny-warnings` | Fail              |
| `2`       | No `.yx` file found                                                            | Depends on config |

## JSON Output Parsing

Use `--json` to get machine-readable output:

```bash
yx check --json | jq '.error_count'
```

## Best Practices

1. **Path argument**: `yx check` defaults to checking the current directory, but you can also
   specify a path: `yx check src/`
2. **Separate check and format**: Run `check` and `format --dry-run` separately to help locate
   issues
3. **Use `--no-progress`**: CI environments don't need progress bars
4. **Use `--color never`**: Avoid ANSI color codes polluting logs
5. **Strict mode**: Append `--deny-warnings` so that warnings also cause CI to fail
6. **Cache dependencies**: Leverage CI caching mechanisms to speed up builds

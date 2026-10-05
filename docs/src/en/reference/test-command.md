# yx test

Run YaoXiang test files. Test files are regular `.yx` source files that declare assertions and
expectations through the `std.test` standard testing module.

## Usage

```
yx test [OPTIONS] [PATH]...
```

## Test Discovery

When `PATH` is not specified, the test scope is determined in the following order:

1. The `patterns` configuration in `[tool.test]` in `./yaoxiang.toml`
2. When not configured, the default discovery is `tests/**/*.yx`

When `PATH` is specified, only the explicitly given paths are run (the `patterns` config is not
read); the exclude patterns and `--filter` from the config are still applied afterwards.

Each test file is divided into four categories by its declared expectations (stable schema, for CI
consumption):

| Category        | Pass Criteria                                                                                           |
| --------------- | ------------------------------------------------------------------------------------------------------- |
| `behavior`      | Run the file; exit code 0 means pass                                                                    |
| `compile-error` | `check` exits with non-zero and all declared error codes appear (file is not run)                       |
| `runtime-error` | `check` must pass and run must fail; all declared error codes appear                                    |
| `invalid`       | File declaration is invalid (e.g., expected codes contradict the category); not counted as pass or fail |

The expectation declaration grammar is described in
[RFC-036](../rfc/accepted/036-test-framework.md).

## Options

| Option            | Description                                                                             | Default |
| ----------------- | --------------------------------------------------------------------------------------- | ------- |
| `--filter <NAME>` | Only run test files whose filename contains the substring                               | None    |
| `--fail-fast`     | Stop after the first failing test file completes                                        | No      |
| `-v`, `--verbose` | Show captured stdout/stderr for each test file                                          | No      |
| `--list`          | Only list discovered test files, do not run them                                        | No      |
| `--no-progress`   | Suppress progress output (titles and PASS lines); failures and summary are always shown | No      |
| `--json`          | Output a JSON report instead of human-readable text                                     | No      |
| `--parallel`      | Run test files in parallel (one worker per CPU core)                                    | No      |

## Exit Codes

| Exit Code | Description                                 |
| --------- | ------------------------------------------- |
| `0`       | All pass (or no test files discovered)      |
| `1`       | Failures exist, or a runtime error occurred |

## JSON Output Format

When using `--json`, the output format is:

```json
{
  "summary": {
    "total": 3,
    "passed": 2,
    "failed": 1,
    "skipped": 0,
    "by_kind": { "behavior": 2, "compile-error": 0, "runtime-error": 1, "invalid": 0 },
    "time_secs": 0.512
  },
  "files": [
    {
      "file": "tests/div_zero_err.yx",
      "kind": "runtime-error",
      "passed": false,
      "time_secs": 0.103,
      "exit_code": 1,
      "stderr": "..."
    }
  ]
}
```

- Failed files include `exit_code` and `stderr` (for CI forensics); with `--verbose` all files
  include `stdout`/`stderr`
- `files` is sorted by path; output is stable
- `by_kind` has fixed four keys (`behavior` / `compile-error` / `runtime-error` / `invalid`); zero
  counts are also output

## Examples

```bash
# Run all project tests
yx test

# Run a specified directory
yx test tests/yaoxiang/

# Only run test files whose filename contains parser
yx test --filter parser

# Stop after the first failure, and show captured output
yx test --fail-fast -v

# Only list discovered test files
yx test --list

# CI mode: parallel, no progress
yx test --parallel --no-progress

# Output a JSON report
yx test --json > report.json
```

## CI Integration

```yaml
# GitHub Actions
- name: Test
  run: yx test --parallel --no-progress
```

For detailed CI configuration, see [CI Integration Guide](../guide/ci-integration.md).

## See also

- [`yx check`](check-command.md) -- Static check
- [`yx format`](format-command.md) -- Code formatting
- [RFC-036: std.test Test Framework](../rfc/accepted/036-test-framework.md) -- Test framework design
- [CI Integration Guide](../guide/ci-integration.md) -- CI/CD integration

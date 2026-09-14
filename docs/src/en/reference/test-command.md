# yx test

Run YaoXiang test files. Test files are ordinary `.yx` source files that declare assertions and
expectations through the `std.test` standard test module.

## Usage

```
yx test [OPTIONS] [PATH]...
```

## Test Discovery

When no `PATH` is specified, the test scope is determined in the following order:

1. The `patterns` configuration under `[tool.test]` in `./yaoxiang.toml`
2. When unconfigured, `tests/**/*.yx` is discovered by default

When `PATH` is specified, only the explicitly given paths are run (the patterns configuration is not
read), and the exclude patterns and `--filter` from the configuration still apply.

Each test file is divided into four categories based on its declared expectations (the schema is
stable, intended for CI consumption):

| Category        | Judgment Criteria                                                                                                |
| --------------- | ---------------------------------------------------------------------------------------------------------------- |
| `behavior`      | Run the file, pass if exit code is 0                                                                             |
| `compile-error` | `check` exits with non-zero and all declared error codes appear, pass (do not run the file)                      |
| `runtime-error` | `check` must pass and the run must fail, pass if all declared error codes appear                                 |
| `invalid`       | The file declaration is invalid (e.g., expected codes contradict the category), neither counted as pass nor fail |

The expectation declaration grammar is described in
[RFC-036](../design/rfc/accepted/036-test-framework.md).

## Options

| Option            | Description                                                                            | Default |
| ----------------- | -------------------------------------------------------------------------------------- | ------- |
| `--filter <NAME>` | Only run test files whose filename contains the substring                              | None    |
| `--fail-fast`     | Stop after the first failing test file completes                                       | No      |
| `-v`, `--verbose` | Show the captured stdout/stderr for each test file                                     | No      |
| `--list`          | Only list discovered test files, do not run                                            | No      |
| `--no-progress`   | Suppress progress output (title and PASS lines); failures and summary are always shown | No      |
| `--json`          | Output a JSON report instead of human-readable text                                    | No      |
| `--parallel`      | Run test files in parallel (one worker per CPU core)                                   | No      |

## Exit Codes

| Exit Code | Description                              |
| --------- | ---------------------------------------- |
| `0`       | All passed (or no test files discovered) |
| `1`       | One or more failures, or a runtime error |

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

- Failed files include `exit_code` and `stderr` (for CI forensics); with `--verbose`, all files
  include `stdout`/`stderr`
- `files` is sorted by path, output is stable
- `by_kind` always has four fixed keys (`behavior` / `compile-error` / `runtime-error` / `invalid`),
  zero counts are still emitted

## Examples

```bash
# Run all project tests
yx test

# Run a specified directory
yx test tests/yaoxiang/

# Only run test files whose filename contains "parser"
yx test --filter parser

# Stop after the first failure and show captured output
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

For detailed CI configuration, see the [CI Integration Guide](../guide/ci-integration.md).

## See Also

- [`yx check`](./check-command.md) -- Static check
- [`yx format`](./format-command.md) -- Code formatter
- [RFC-036: std.test testing framework](../design/rfc/accepted/036-test-framework.md) -- Testing
  framework design
- [CI Integration Guide](../guide/ci-integration.md) -- CI/CD integration

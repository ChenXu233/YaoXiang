# yx check

Performs static checking (type checking, ownership checking) on YaoXiang source code, without
generating any code.

## Usage

```
yx check [OPTIONS] [PATH]...
```

## Arguments

| Argument | Description                                                                        |
| -------- | ---------------------------------------------------------------------------------- |
| `PATH`   | One or more file or directory paths. If not specified, checks the current project. |

## Options

| Option             | Description                                                               | Default |
| ------------------ | ------------------------------------------------------------------------- | ------- |
| `--json`           | Output diagnostic information in JSON format                              | No      |
| `--color <MODE>`   | Color output mode: `auto`, `always`, `never`                              | `auto`  |
| `--exclude <PATH>` | Exclude the specified path (can be used multiple times)                   | None    |
| `--no-progress`    | Suppress progress and summary messages                                    | No      |
| `--deny-warnings`  | Treat warnings as errors: exit with a non-zero code if any warnings exist | No      |

## Exit Codes

| Exit Code | Description                                                                    |
| --------- | ------------------------------------------------------------------------------ |
| `0`       | No errors                                                                      |
| `1`       | Errors found during checking; or warnings exist when `--deny-warnings` is used |
| `2`       | No `.yx` files found                                                           |

## Cross-File Analysis

`yx check` supports cross-file type checking. When checking multiple files:

1. Parse all `.yx` files in parallel
2. Build the module dependency graph
3. Detect circular dependencies (report as errors)
4. Check in topological sort order
5. Use a shared type environment to correctly detect cross-file references

```bash
# Check the entire project (automatically detect cross-file references)
yx check src/

# Check specific files
yx check src/main.yx src/lib.yx
```

## JSON Output Format

When using `--json`, the output format is:

```json
{
  "error_count": 0,
  "warning_count": 0,
  "diagnostics": [
    {
      "file": "src/main.yx",
      "severity": "error",
      "code": "E1001",
      "message": "Unknown variable: 'x'",
      "line": 5,
      "column": 3,
      "end_line": 5,
      "end_column": 4,
      "lsp": { ... }
    }
  ]
}
```

## Examples

```bash
# Check the current project
yx check

# Check specific files
yx check src/main.yx

# Check a directory and output JSON
yx check src/ --json

# CI mode (no color, no progress)
yx check --color never --no-progress

# CI strict mode (warnings also cause failure)
yx check --deny-warnings

# Exclude the test directory
yx check src/ --exclude tests/
```

## CI Integration

```yaml
# GitHub Actions
- name: Type check
  run: yx check --color never --no-progress
```

For detailed CI configuration, see the [CI Integration Guide](../guide/ci-integration.md).

## See Also

- [`yx format`](./format-command.md) -- Code formatting
- [`yx test`](./test-command.md) -- Run tests
- [Error Code Reference](./error-codes.md) -- Complete list of error codes
- [CI Integration Guide](../guide/ci-integration.md) -- CI/CD integration
- [Diagnostic System Design](../design/check/diagnostic-system.md) -- Architecture design document

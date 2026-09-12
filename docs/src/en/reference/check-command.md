# yx check

Performs static checks (type checking, ownership checking) on YaoXiang source code, without
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

| Option             | Description                                             | Default |
| ------------------ | ------------------------------------------------------- | ------- |
| `--json`           | Output diagnostics in JSON format                       | No      |
| `-w`, `--watch`    | Watch for file changes and re-check automatically       | No      |
| `--color <MODE>`   | Color output mode: `auto`, `always`, `never`            | `auto`  |
| `--exclude <PATH>` | Exclude the specified path (can be used multiple times) | None    |
| `--no-progress`    | Suppress progress and summary messages                  | No      |

## Exit Codes

| Exit Code | Description          |
| --------- | -------------------- |
| `0`       | No errors            |
| `1`       | Check found errors   |
| `2`       | No `.yx` files found |

## Cross-File Analysis

`yx check` supports cross-file type checking. When checking multiple files:

1. Parse all `.yx` files in parallel
2. Build the module dependency graph
3. Detect circular dependencies (reported as errors)
4. Check in topological order
5. Use a shared type environment to correctly detect cross-file references

```bash
# Check the entire project (automatically detects cross-file references)
yx check src/

# Check specified files
yx check src/main.yx src/lib.yx
```

## Incremental Checking (watch mode)

Use `-w` or `--watch` to enable file watch mode. When files change, it automatically re-checks.

```bash
yx check --watch
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

# Check specified files
yx check src/main.yx

# Check a directory and output JSON
yx check src/ --json

# Watch mode
yx check --watch

# CI mode (no color, no progress)
yx check --color never --no-progress

# Exclude the tests directory
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
- [Error Code Reference](./error-codes.md) -- Complete list of error codes
- [CI Integration Guide](../guide/ci-integration.md) -- CI/CD integration
- [Diagnostic System Design](../design/check/diagnostic-system.md) -- Architecture design document

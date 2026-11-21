# yx check

Performs static checks on YaoXiang source code (type checking and ownership checking) without
generating any code.

## Usage

```
yx check [OPTIONS] [PATH]...
```

## Arguments

| Argument | Description                                                                     |
| -------- | ------------------------------------------------------------------------------- |
| `PATH`   | One or more file or directory paths. Checks the current project if unspecified. |

## Options

| Option             | Description                                                                                                                                                                        | Default |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- |
| `--json`           | Output diagnostics in JSON format                                                                                                                                                  | No      |
| `--color <MODE>`   | Color output mode: `auto`, `always`, `never`                                                                                                                                       | `auto`  |
| `--exclude <PATH>` | Exclude the specified path (may be used multiple times)                                                                                                                            | None    |
| `--no-progress`    | Suppress progress and summary messages                                                                                                                                             | No      |
| `--allow <CODE>`   | Suppress the specified warning code: it is neither output nor included in the warning count (repeatable; unknown codes and codes outside the W family result in an argument error) | None    |
| `--deny-warnings`  | Treat warnings as errors: exit with a non-zero code if warnings exist                                                                                                              | No      |
| `--deny-shadowing` | Exit with a non-zero code when a local module shadows a vendor dependency package (W1006)                                                                                          | No      |

## Exit Codes

| Exit Code | Description                                                                 |
| --------- | --------------------------------------------------------------------------- |
| `0`       | No errors                                                                   |
| `1`       | Errors were found during checking; or warnings exist with `--deny-warnings` |
| `2`       | No `.yx` files were found                                                   |

## Cross-File Analysis

`yx check` supports cross-file type checking. When checking multiple files, it:

1. Parses all `.yx` files in parallel
2. Builds the module dependency graph
3. Detects circular dependencies (reports an error)
4. Checks in topological order
5. Uses a shared type environment to correctly detect cross-file references

```bash
# Check the entire project (automatically detects cross-file references)
yx check src/

# Check specified files
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

# Check specified files
yx check src/main.yx

# Check a directory and output JSON
yx check src/ --json

# CI mode (no color, no progress)
yx check --color never --no-progress

# CI strict mode (warnings also cause failure)
yx check --deny-warnings

# Suppress the specified warning code (it is neither output nor counted)
yx check src/ --allow W1007

# Fail when a local module shadows a vendor dependency package
yx check src/ --deny-shadowing

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

- [`yx format`](format-command.md) -- Code formatting
- [`yx test`](test-command.md) -- Run tests
- [Error Code Reference](error-code/index.md) -- Complete error code list
- [CI Integration Guide](../guide/ci-integration.md) -- CI/CD integration
- [Diagnostic System Design](../dev/design/check/diagnostic-system.md) -- Architecture design
  document

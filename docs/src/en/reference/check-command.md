# yaoxiang check

Perform static checks on YaoXiang source code (type checking, ownership checking), do not generate any code.

## Usage

```
yaoxiang check [OPTIONS] [PATH]...
```

## Arguments

| Argument | Description |
|----------|-------------|
| `PATH` | One or more file or directory paths. When not specified, checks the current project. |

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `--json` | Output diagnostics in JSON format | No |
| `-w`, `--watch` | Watch for file changes and automatically re-check | No |
| `--color <MODE>` | Color output mode: `auto`, `always`, `never` | `auto` |
| `--exclude <PATH>` | Exclude the specified path (can be used multiple times) | None |
| `--no-progress` | Suppress progress and summary messages | No |

## Exit Codes

| Exit Code | Description |
|-----------|-------------|
| `0` | No errors |
| `1` | Check found errors |
| `2` | No `.yx` files found |

## Cross-file Analysis

`yaoxiang check` supports cross-file type checking. When checking multiple files:

1. Parse all `.yx` files in parallel
2. Build module dependency graph
3. Detect circular dependencies (report error)
4. Check in topological sort order
5. Use shared type environment, correctly detect cross-file references

```bash
# Check entire project (automatically detect cross-file references)
yaoxiang check src/

# Check specified files
yaoxiang check src/main.yx src/lib.yx
```

## Incremental Checking (watch mode)

Use `-w` or `--watch` to enable file watching mode. Automatically re-check when files change.

```bash
yaoxiang check --watch
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
# Check current project
yaoxiang check

# Check specified files
yaoxiang check src/main.yx

# Check directory and output JSON
yaoxiang check src/ --json

# Watch mode
yaoxiang check --watch

# CI mode (no colors, no progress)
yaoxiang check --color never --no-progress

# Exclude test directory
yaoxiang check src/ --exclude tests/
```

## CI Integration

```yaml
# GitHub Actions
- name: Type check
  run: yaoxiang check --color never --no-progress
```

For detailed CI configuration, see [CI Integration Guide](../guide/ci-integration.md).

## See Also

- [`yaoxiang fmt`](./format-command.md) -- Code formatting
- [Error Codes Reference](./error-codes.md) -- Complete error code list
- [CI Integration Guide](../guide/ci-integration.md) -- CI/CD integration
- [Diagnostic System Design](../design/check/diagnostic-system.md) -- Architecture design documentation
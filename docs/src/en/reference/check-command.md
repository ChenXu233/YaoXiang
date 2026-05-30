# yaoxiang check

Performs static checks (type checking, ownership checking) on YaoXiang source code without generating any code.

## Usage

```bash
yaoxiang check [OPTIONS] [PATH]...
```

## Arguments

| Argument | Description |
|----------|-------------|
| `PATH` | One or more file or directory paths. When not specified, checks the current project. |

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `--json` | Output diagnostic information in JSON format | No |
| `-w`, `--watch` | Watch for file changes and automatically re-check | No |
| `--color <MODE>` | Color output mode: `auto`, `always`, `never` | `auto` |
| `--exclude <PATH>` | Exclude specified path (can be used multiple times) | None |
| `--no-progress` | Suppress progress and summary messages | No |

## Exit Codes

| Exit Code | Description |
|-----------|-------------|
| `0` | No errors |
| `1` | Check found errors |
| `2` | No `.yx` files found |

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

# Check specified file
yaoxiang check src/main.yx

# Check directory and output JSON
yaoxiang check src/ --json

# Watch mode
yaoxiang check --watch

# CI mode (no colors, no progress)
yaoxiang check --color never --no-progress

# Exclude tests directory
yaoxiang check src/ --exclude tests/
```

## CI Integration

```yaml
# GitHub Actions
- name: Type check
  run: yaoxiang check --color never --no-progress
```

## See Also

- [`yaoxiang fmt`](./format-command.md) -- Code formatting
- [Error Codes Reference](./error-codes.md) -- Complete error code list
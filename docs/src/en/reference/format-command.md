# yx format

Format YaoXiang source code, unifying code style.

## Usage

```
yx format [OPTIONS] <PATH>
```

## Arguments

| Argument | Description                        |
| -------- | ---------------------------------- |
| `PATH`   | Source file or directory to format |

## Options

| Option             | Description                                                      | Default      |
| ------------------ | ---------------------------------------------------------------- | ------------ |
| `-n`, `--dry-run`  | Dry run: show formatting diff, do not write back                 | No           |
| `-w`, `--write`    | Write formatting result in place                                 | No           |
| `--stdout`         | Output to standard output (default when `-n`/`-w` not specified) | --           |
| `--no-verify`      | Skip verification after formatting (performance optimization)    | No           |
| `--indent <N>`     | Override indent width                                            | Config value |
| `--line-width <N>` | Override max line width                                          | Config value |
| `--use-tabs`       | Use Tab indentation                                              | Config value |
| `--single-quote`   | Use single quotes for strings                                    | Config value |

## Configuration Priority

Formatting options take effect in the following priority order (highest first):

1. Command line flags
2. Formatting configuration in project config `yaoxiang.toml`
3. User-level configuration file

For configuration items and default values, see
[Formatter Configuration Design](../design/formatter/configuration.md).

## Exit Codes

| Exit code | Description                                                               |
| --------- | ------------------------------------------------------------------------- |
| `0`       | Success (including cases where no formatting is needed)                   |
| `2`       | Files needing formatting found under `--dry-run`, or no `.yx` files found |
| `1`       | Other errors                                                              |

## Examples

```bash
# Format and output to standard output (default)
yx format src/main.yx

# Write in place
yx format -w src/

# CI check: fail with exit code 2 when files need formatting
yx format -n .

# Temporarily override indent and line width
yx format --indent 4 --line-width 100 main.yx
```

## CI Integration

```yaml
# GitHub Actions
- name: Format check
  run: yx format --dry-run .
```

Exit code `2` indicates files that need formatting. For detailed CI configuration, see the
[CI Integration Guide](../guide/ci-integration.md).

## See Also

- [`yx check`](./check-command.md) -- Static check
- [`yx test`](./test-command.md) -- Run tests
- [Formatting Rules Overview](../design/formatter/formatting-rules/index.md) -- Individual
  formatting rules
- [CI Integration Guide](../guide/ci-integration.md) -- CI/CD integration

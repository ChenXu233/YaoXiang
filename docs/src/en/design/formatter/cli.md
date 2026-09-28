---
title: 'yaoxiang format CLI Usage'
description: Command-line arguments and usage for the formatting tool
---

# CLI Usage

---

## A. CLI Usage

```bash
# Format a file (output to stdout)
yaoxiang format file.yx

# Check if a file is already formatted
yaoxiang format --dry-run file.yx

# Format and write to file
yaoxiang format -w file.yx

# Format all .yx files in a directory
yaoxiang format -w src/
```

---

## B. CLI Parameters

| Argument         | Description                       | Default |
| ---------------- | --------------------------------- | ------- |
| `--dry-run`      | Check mode, does not modify files | false   |
| `-w`, `--write`  | Write mode, modifies files        | false   |
| `--stdout`       | Output to stdout                  | false   |
| `--indent-width` | Indent width                      | 4       |
| `--line-width`   | Max line width                    | 120     |
| `--use-tabs`     | Use tab indentation               | false   |
| `--single-quote` | Use single quotes                 | false   |

---

## C. References

- [Issue #13: Implement yaoxiang format code formatter](https://github.com/ChenXu233/YaoXiang/issues/13)
- [Rustfmt Style Guide](https://rust-lang.github.io/rustfmt/)
- [Testing Specification](../../dev/test-specification.md)

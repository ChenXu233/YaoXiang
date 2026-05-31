---
title: "yaoxiang fmt Command Line Usage"
description: Command line arguments and usage methods for the formatting tool
---

# Command Line Usage

---

## A. Command Line Usage

```bash
# Format a file (output to stdout)
yaoxiang fmt file.yx

# Check if a file is already formatted
yaoxiang fmt --check file.yx

# Format and write to file
yaoxiang fmt --write file.yx

# Format all .yx files in a directory
yaoxiang fmt --write src/
```

---

## B. CLI Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `--check` | Check mode, does not modify files | false |
| `--write` | Write mode, modifies files | false |
| `--stdout` | Output to stdout | false |
| `--indent-width` | Indent width | 4 |
| `--line-width` | Maximum line width | 120 |
| `--use-tabs` | Use tab indentation | false |
| `--single-quote` | Use single quotes | false |

---

## C. References

- [Issue #13: Implement yaoxiang fmt code formatting tool](https://github.com/ChenXu233/YaoXiang/issues/13)
- [Rustfmt Style Guide](https://rust-lang.github.io/rustfmt/)
- [Test Writing Specification](../test-specification.md)
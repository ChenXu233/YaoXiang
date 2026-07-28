---
title: "yaoxiang format Command Line Usage"
description: Command line arguments and usage for the formatting tool
---

# Command Line Usage

---

## A. Command Line Usage

```bash
# Format file (output to stdout)
yaoxiang format file.yx

# Check if file is already formatted
yaoxiang format --dry-run file.yx

# Format and write to file
yaoxiang format -w file.yx

# Format all .yx files in directory
yaoxiang format -w src/
```

---

## B. CLI Arguments

| Argument          | Description                    | Default |
| ----------------- | ------------------------------ | ------- |
| `--dry-run`       | Check mode, do not modify files | false   |
| `-w`, `--write`   | Write mode, modify files       | false   |
| `--stdout`        | Output to stdout               | false   |
| `--indent-width`  | Indent width                   | 4       |
| `--line-width`    | Maximum line width             | 120     |
| `--use-tabs`      | Use tab for indentation        | false   |
| `--single-quote`  | Use single quotes              | false   |

---

## C. References

- [Issue #13: Implement yaoxiang format code formatting tool](https://github.com/ChenXu233/YaoXiang/issues/13)
- [Rustfmt Style Guide](https://rust-lang.github.io/rustfmt/)
- [Test Specification](../test-specification.md)

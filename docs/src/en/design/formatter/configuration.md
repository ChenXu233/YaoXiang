---
title: 'Formatting Configuration Options'
description: 'Configuration file format, priority, and default values for yaoxiang fmt'
---

# Configuration Options

---

## Configuration File Format

The configuration file uses TOML format, and the file name is `yaoxiang.toml`.

```toml
[fmt]
# 行宽限制（默认 120）
line_width = 120

# 缩进宽度（默认 4）
indent_width = 4

# 是否使用 tab 缩进（默认 false）
use_tabs = false

# 是否使用单引号（默认 false）
single_quote = false

# 是否排序导入语句（默认 true）
sort_imports = true
```

---

## Configuration Priority

Configuration priority chain (from high to low):

1. **CLI Parameters** — Command-line parameters have the highest priority
2. **Project-level Configuration** — `yaoxiang.toml` in the current directory
3. **User-level Configuration** — `~/.config/yaoxiang/config.toml`
4. **Default Values** — Built-in default values

---

## Default Values

| Option         | Default Value | Description                  |
| -------------- | ------------- | ---------------------------- |
| `line_width`   | 120           | Maximum line width           |
| `indent_width` | 4             | Number of indent spaces      |
| `use_tabs`     | false         | Whether to use tabs          |
| `single_quote` | false         | Whether to use single quotes |
| `sort_imports` | true          | Whether to sort imports      |

---
title: "Formatting Configuration Options"
description: yaoxiang fmt configuration file format, priority, and defaults
---

# Configuration Options

---

## Configuration File Format

The configuration file uses TOML format, with the filename `yaoxiang.toml`.

```toml
[fmt]
# Line width limit (default 120)
line_width = 120

# Indent width (default 4)
indent_width = 4

# Whether to use tab indentation (default false)
use_tabs = false

# Whether to use single quotes (default false)
single_quote = false

# Whether to sort import statements (default true)
sort_imports = true
```

---

## Configuration Priority

Configuration priority chain (from highest to lowest):

1. **CLI Arguments** — Command-line arguments have the highest priority
2. **Project-level Configuration** — `yaoxiang.toml` in the current directory
3. **User-level Configuration** — `~/.config/yaoxiang/config.toml`
4. **Default Values** — Built-in defaults

---

## Default Values

| Option | Default | Description |
|--------|---------|-------------|
| `line_width` | 120 | Maximum line width |
| `indent_width` | 4 | Number of indent spaces |
| `use_tabs` | false | Whether to use tabs |
| `single_quote` | false | Whether to use single quotes |
| `sort_imports` | true | Whether to sort imports |
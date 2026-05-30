---
title: "格式化配置选项"
description: yaoxiang fmt 的配置文件格式、优先级和默认值
---

# 配置选项

---

## 配置文件格式

配置文件使用 TOML 格式，文件名为 `yaoxiang.toml`。

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

## 配置优先级

配置优先级链（从高到低）：

1. **CLI 参数** — 命令行参数具有最高优先级
2. **项目级配置** — 当前目录的 `yaoxiang.toml`
3. **用户级配置** — `~/.config/yaoxiang/config.toml`
4. **默认值** — 内置默认值

---

## 默认值

| 选项 | 默认值 | 说明 |
|------|--------|------|
| `line_width` | 120 | 最大行宽 |
| `indent_width` | 4 | 缩进空格数 |
| `use_tabs` | false | 是否使用 tab |
| `single_quote` | false | 是否使用单引号 |
| `sort_imports` | true | 是否排序导入 |

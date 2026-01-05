# Phase 2: 语法分析器

> **模块路径**: `src/frontend/parser/`
> **状态**: ⏳ 待实现

## 概述

语法分析器将 Token 序列转换为抽象语法树（AST）。采用**递归下降解析**与**运算符优先级解析**相结合的方式。

## 文件结构

```
phase-02-parser/
├── README.md                      # 本文档
├── task-02-01-basic-parsing.md    # 基础解析（表达式、语句）
├── task-02-02-expressions.md      # 表达式解析
├── task-02-03-statements.md       # 语句解析
├── task-02-04-functions.md        # 函数解析
├── task-02-05-types.md            # 类型解析
├── task-02-06-pattern-matching.md # 模式匹配解析
└── task-02-07-modules.md          # 模块解析
```

## 完成状态

| Task | 名称 | 状态 |
|------|------|------|
| task-02-01 | 基础解析 | ⏳ 待实现 |
| task-02-02 | 表达式解析 | ⏳ 待实现 |
| task-02-03 | 语句解析 | ⏳ 待实现 |
| task-02-04 | 函数解析 | ⏳ 待实现 |
| task-02-05 | 类型解析 | ⏳ 待实现 |
| task-02-06 | 模式匹配解析 | ⏳ 待实现 |
| task-02-07 | 模块解析 | ⏳ 待实现 |

## 相关文件

- **mod.rs**: Parser 主实现
- **ast.rs**: AST 节点定义
- **tests/**: 测试用例

# Phase 1: 词法分析器

> **模块路径**: `src/frontend/lexer/`
> **状态**: ✅ 已完成

## 概述

词法分析器是编译器的前端第一阶段，负责将源代码字符串转换为 Token 序列。

## 文件结构

```
phase-01-lexer/
├── README.md                    # 本文档
├── task-01-01-basic-tokens.md   # 关键字和标识符
├── task-01-02-literals.md       # 字面量（数字、字符串、字符）
├── task-01-03-operators.md      # 运算符和分隔符
└── task-01-04-comments.md       # 注释处理
```

## 完成状态

| Task | 名称 | 状态 |
|------|------|------|
| task-01-01 | 关键字和标识符 | ✅ 完成 |
| task-01-02 | 字面量 | ✅ 完成 |
| task-01-03 | 运算符和分隔符 | ✅ 完成 |
| task-01-04 | 注释处理 | ✅ 完成 |

## 相关文件

- **tokens.rs**: Token 类型定义
- **mod.rs**: Lexer 主实现
- **tests/**: 测试用例

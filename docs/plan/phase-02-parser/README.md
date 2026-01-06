# Phase 2: 语法分析器

> **模块路径**: `src/frontend/parser/`
> **状态**: ✅ 已实现

## 概述

语法分析器将 Token 序列转换为抽象语法树（AST）。采用 **Pratt Parser**（运算符优先级解析）实现完整的表达式解析，结合递归下降解析处理语句和类型。

## 文件结构

```
phase-02-parser/
├── README.md                      # 本文档
├── task-02-01-basic-parsing.md    # 基础解析（表达式、语句）
├── task-02-02-expressions.md      # 表达式解析（含列表推导式）
├── task-02-03-statements.md       # 语句解析
├── task-02-04-functions.md        # 函数解析
├── task-02-05-types.md            # 类型解析
├── task-02-06-pattern-matching.md # 模式匹配解析
└── task-02-07-modules.md          # 模块解析
```

## 完成状态

| Task | 名称 | 状态 |
|------|------|------|
| task-02-01 | 基础解析 | ✅ 已实现 |
| task-02-02 | 表达式解析 | ✅ 已实现 |
| task-02-03 | 语句解析 | ✅ 已实现 |
| task-02-04 | 函数解析 | ✅ 已实现 |
| task-02-05 | 类型解析 | ✅ 已实现 |
| task-02-06 | 模式匹配解析 | ✅ 已实现 |
| task-02-07 | 模块解析 | ✅ 已实现 |

## 解析器架构

```
src/frontend/parser/
├── mod.rs          # Parser 主入口，parse() 函数
├── state.rs        # ParserState, BP_* 绑定功率常量
├── ast.rs          # Expr, Stmt, Type, Pattern 定义
├── expr.rs         # Pratt parser 主循环
├── nud.rs          # 前缀表达式解析
├── led.rs          # 中缀表达式解析
├── stmt.rs         # 语句解析
├── type_parser.rs  # 类型解析
└── tests/          # 测试用例
```

## 核心特性

- **Pratt Parser**: 支持完整的运算符优先级和结合性
- **10 级优先级**: 从赋值到函数调用
- **表达式**: 字面量、算术、逻辑、比较、函数调用、Lambda、列表推导等
- **语句**: 变量声明、函数定义、控制流、模块导入等
- **类型系统**: 基本类型、泛型、函数类型、变体类型等
- **模式匹配**: 通配符、字面量、构造器、元组、守卫等

## 相关文件

- **mod.rs**: Parser 主实现
- **ast.rs**: AST 节点定义
- **tests/**: 测试用例

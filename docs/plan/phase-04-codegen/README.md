# Phase 4: 字节码生成器

> **模块路径**: `src/middle/codegen/`
> **状态**: ⏳ 待实现

## 概述

字节码生成器将类型检查后的 AST 转换为中间表示（IR），然后生成目标平台的字节码指令序列。

## 文件结构

```
phase-04-codegen/
├── README.md                           # 本文档
├── task-04-01-arithmetic.md            # 算术运算字节码
├── task-04-02-logic.md                 # 逻辑运算字节码
├── task-04-03-control-flow.md          # 控制流字节码
├── task-04-04-function-call.md         # 函数调用字节码
├── task-04-05-data-structure.md        # 数据结构字节码
├── task-04-06-pattern-matching.md      # 模式匹配字节码
├── task-04-07-concurrency.md           # 并发原语字节码
├── task-04-08-error-handling.md        # 错误处理字节码
└── task-04-09-escape-analysis.md       # 逃逸分析集成
```

## 完成状态

| Task | 名称 | 状态 |
|------|------|------|
| task-04-01 | 算术运算字节码 | ⏳ 待实现 |
| task-04-02 | 逻辑运算字节码 | ⏳ 待实现 |
| task-04-03 | 控制流字节码 | ⏳ 待实现 |
| task-04-04 | 函数调用字节码 | ⏳ 待实现 |
| task-04-05 | 数据结构字节码 | ⏳ 待实现 |
| task-04-06 | 模式匹配字节码 | ⏳ 待实现 |
| task-04-07 | 并发原语字节码 | ⏳ 待实现 |
| task-04-08 | 错误处理字节码 | ⏳ 待实现 |
| task-04-09 | 逃逸分析集成 | ⏳ 待实现 |

## 相关文件

- **ir.rs**: 中间表示定义
- **bytecode.rs**: 字节码指令定义
- **generator.rs**: 代码生成器主实现
- **emit.rs**: 字节码发射器

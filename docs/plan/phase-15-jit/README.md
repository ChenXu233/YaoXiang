# Phase 13: JIT 编译器

> **模块路径**: `src/vm/jit/`
> **状态**: ⏳ 待实现

## 概述

JIT 编译器将热点字节码编译为机器码，提升执行性能。

## 文件结构

```
phase-13-jit/
├── README.md                   # 本文档
├── task-13-01-profiler.md      # 热点分析
├── task-13-02-code-gen.md      # 机器码生成
└── task-13-03-codegen-cache.md # 代码缓存
```

## 完成状态

| Task | 名称 | 状态 |
|------|------|------|
| task-13-01 | 热点分析 | ⏳ 待实现 |
| task-13-02 | 机器码生成 | ⏳ 待实现 |
| task-13-03 | 代码缓存 | ⏳ 待实现 |

## 相关文件

- **mod.rs**: JIT 编译器主模块
- **profiler.rs**: 热点分析器
- **machine_code.rs**: 机器码生成器

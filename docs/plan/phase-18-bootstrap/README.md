# Phase 18: Bootstrap（自举）

> **模块路径**: `src/bootstrap/`
> **状态**: ⏳ 待实现
> **依赖**: P1-P17（所有前期阶段）

## 概述

Bootstrap 实现 YaoXiang 语言的自举——使用 YaoXiang 编写的代码来编译 YaoXiang 自身。

## 文件结构

```
phase-18-bootstrap/
├── README.md                    # 本文档
├── task-18-01-compiler.md       # 自举编译器
├── task-18-02-standard-lib.md   # 标准库自举
├── task-18-03-bootstrapping.md  # 自举流程
└── task-18-04-migration.md      # 迁移策略
```

## 完成状态

| Task | 名称 | 状态 | 依赖 |
|------|------|------|------|
| task-18-01 | 自举编译器 | ⏳ 待实现 | P17 |
| task-18-02 | 标准库自举 | ⏳ 待实现 | task-18-01 |
| task-18-03 | 自举流程 | ⏳ 待实现 | task-18-01 |
| task-18-04 | 迁移策略 | ⏳ 待实现 | task-18-03 |

## 自举架构

```
┌─────────────────────────────────────────────────────────┐
│                    Bootstrap 架构                        │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Stage 0: 现有 Rust 编译器                               │
│  └── 编译 YaoXiang 源码                                  │
│                                                         │
│  ↓                                                       │
│                                                         │
│  Stage 1: YaoXiang 编译器（第1版）                       │
│  └── 用 YaoXiang 重写部分组件                            │
│                                                         │
│  ↓                                                       │
│                                                         │
│  Stage 2: YaoXiang 编译器（第2版）                       │
│  └── 更大比例的 YaoXiang 代码                            │
│                                                         │
│  ↓                                                       │
│                                                         │
│  Stage 3: 完全自举                                       │
│  └── 100% YaoXiang 代码                                  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## 自举策略

### 阶段 1：核心组件重写

优先重写稳定的组件：

```yaoxiang
# 优先级：高
# 原因：稳定、依赖少

# 1. 实用工具函数
module std::util

# 2. 字符串处理
module std::string

# 3. 集合类型
module std::collections::list
module std::collections::dict
```

### 阶段 2：编译流程组件

```yaoxiang
# 优先级：中
# 原因：需要类型系统支持

# 1. 词法分析器（相对独立）
module compiler::lexer

# 2. 语法分析器
module compiler::parser
```

### 阶段 3：完整编译器

```yaoxiang
# 优先级：低
# 原因：复杂、依赖多

# 完整编译器
module compiler::main
module compiler::typecheck
module compiler::codegen
```

## 自举流程

```bash
# Stage 0: 使用 Rust 编译器
cd stage-0
cargo build --release
./yaoxiangc ../src/main.yx -o yaoxiang

# Stage 1: 使用 Stage 0 编译器
cd stage-1
../stage-0/yaoxiangc compile main.yx
./yaoxiang compile ../compiler/lexer.yx -o lexer

# Stage 2: 使用 Stage 1 编译器
cd stage-2
../stage-1/yaoxiang compile ../compiler/parser.yx -o parser

# Stage 3: 验证自举
cd stage-3
../stage-2/yaoxiang compile ../src/main.yx -o yaoxiang
diff stage-3/yaoxiang stage-2/yaoxiang
```

## 迁移策略

### 渐进式迁移

```yaoxiang
# 文件级别迁移
# 每个文件标记迁移状态

module compiler::lexer  # 状态: 已迁移
module compiler::parser # 状态: 已迁移
module compiler::typecheck # 状态: 待迁移

# 迁移优先级
# 1. 独立模块（无复杂依赖）
# 2. 稳定模块（API 不变）
# 3. 核心模块（被广泛引用）
```

### API 兼容性

```yaoxiang
# 保持 Rust 和 YaoXiang 版本 API 一致
# 避免破坏性变更

pub fn tokenize(code: String) -> Array[Token] {
    # 无论 Rust 还是 YaoXiang 实现，API 一致
}
```

## 验证自举

```bash
#!/bin/bash
# bootstrap.sh - 验证自举正确性

# 1. 构建 Stage 0
echo "=== Building Stage 0 ==="
cd stage-0
cargo build --release

# 2. 使用 Stage 0 编译 Stage 1
echo "=== Compiling Stage 1 ==="
./target/release/yaoxiangc ../stage-1/src/main.yx -o ../stage-1/yaoxiang

# 3. 使用 Stage 1 编译 Stage 2
echo "=== Compiling Stage 2 ==="
cd ../stage-1
./yaoxiangc ../stage-2/src/main.yx -o ../stage-2/yaoxiang

# 4. 验证结果一致
echo "=== Verifying ==="
cd ../stage-2
sha256sum yaoxiang
cd ../stage-0
sha256sum target/release/yaoxiang

echo "=== Bootstrap Complete ==="
```

## 风险与缓解

| 风险 | 缓解措施 |
|------|----------|
| 循环依赖 | 拓扑排序迁移顺序 |
| 性能回归 | 基准测试监控 |
| API 不兼容 | 保持 API 稳定 |
| 调试困难 | 保留 Rust 参考实现 |

## 相关文档

- [Phase 17: Stdlib](../phase-17-stdlib/README.md)
- [Phase 19: AOT](../phase-19-aot/README.md)

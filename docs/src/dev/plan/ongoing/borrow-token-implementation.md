---
title: 借用令牌系统实现路线图
status: ongoing
created: 2026-05-29
---

# 借用令牌系统实现路线图

## 目标

完整实现 RFC-009 v9 的借用令牌系统，替代旧的丐版借用。

## 前置依赖链

```
RFC-009 v9 (借用令牌设计) ← 已完成
    │
    ├── 1. 类型属性系统 (新 RFC)
    │      ├── Copy trait 定义与实现
    │      ├── MonoType 类型标记 (bitflags)
    │      ├── trait solver 递归 struct 检查
    │      └── auto-derive 递归字段检查
    │
    ├── 2. 闭包捕获模型 (新 RFC)
    │      ├── 变量捕获分析
    │      ├── 借用捕获 vs 移动捕获
    │      └── MakeClosure env 填充
    │
    └── 3. 借用令牌实现
           ├── MonoType::Ref { mutable, inner }
           ├── borrow checker pass (middle/passes/lifetime/)
           ├── 令牌冲突检测 (流敏感活性分析)
           └── ZST 优化 (令牌编译后消失)
```

## 阶段

### 阶段 1：类型属性系统

**状态**：设计中

**范围**：
- MonoType 新增 `TypeFlags`（Copy/Linear/Move 标记）
- `Copy` trait 注册为内置 marker trait
- 原语类型自动标记为 Copy
- struct 自动推导：所有字段 Copy → struct Copy
- trait solver 支持递归 struct 检查
- auto-derive 支持泛型容器字段

**相关文件**：
- `src/frontend/core/types/base/mono.rs`
- `src/frontend/core/typecheck/traits/std_traits.rs`
- `src/frontend/core/typecheck/traits/auto_derive.rs`
- `src/frontend/core/typecheck/traits/solver.rs`

### 阶段 2：闭包捕获模型

**状态**：待设计

**范围**：
- 类型检查时分析 lambda 引用的外部变量
- 确定每个变量的捕获方式（借用令牌 vs Move）
- IR 生成时填充 MakeClosure env
- 支持借用令牌在闭包中的传播

### 阶段 3：借用令牌实现

**状态**：待阶段 1、2 完成

**范围**：
- AST: `Type::Ref`、`Expr::Borrow`
- 词法: `&` 和 `&mut` 令牌
- MonoType: `Ref { mutable, inner }`
- IR: 借用指令（按需）
- Passes: `BorrowChecker` (流敏感活性分析)
- ZST 优化: 令牌编译后消除

## 参考

- [RFC-009 所有权模型 v9](../../design/rfc/accepted/009-ownership-model.md)
- [RFC-010 统一类型语法](../../design/rfc/accepted/010-unified-type-syntax.md)
- [RFC-011 泛型系统设计](../../design/rfc/accepted/011-generic-type-system.md)

---
title: 'RFC-039: 模式匹配完备化'
status: '草案'
author: '晨煦'
created: '2026-09-03'
updated: '2026-09-03'
---

# RFC-039: 模式匹配完备化

## 摘要

将 `match` 的模式匹配从当前的"仅字面量 + 通配符"补全为完备能力：Union 变体模式（含载荷绑定）、结构体/元组模式、或模式与守卫的 IR 落地，以及穷尽性检查（E1030/E1031 从空挂码转为真实发射）。本 RFC 是 `Error { kind: ErrorKind, message }` 演进路线（RFC-013「运行时错误值与码贯通」演进节）的前置，但动机独立——模式匹配是通用语言能力，服务于所有带变体的类型。

## 动机

### 现状实证（2026-09-03，v0.7.12 代码）

1. **IR 层只有字面量模式是真的**。AST 已定义完整模式集（`src/frontend/core/parser/ast.rs:578`：Wildcard / Identifier / Literal / Tuple / Struct / Union / Or / Guard），解析器可解析 `ok(v)`、`err(e)` 类变体模式（`Pattern::Union`）；但 IR 生成（`src/middle/core/ir_gen.rs:5330` 起）只实现 `Literal` 分支，其余模式落入 stub——加载常量 0 参与相等比较，**永远不匹配**；且 scrutinee 恰为 0 时会**误匹配**进 stub arm（潜在错误行为）。
2. **stdlib.md 的 `match ok(v)/err(e)` 示例是纸面能力**。规范文档（§1.3 Result）给出的变体解构写法实际运行不了；`?` 运算符的实现不走 match 脱糖，掩盖了这一点。测试语料中零变体解构用例（`match.yx` / `pattern_matching.yx` 仅覆盖字面量与通配符）。
3. **穷尽性检查空挂**。E1030（Pattern non-exhaustive）、E1031（Unreachable pattern）在码表中注册但无发射点——match 语义未定型。
4. **错误处理演进被阻塞**。运行时 `Error` 值目前以 `{code, message}` 字符串码为唯一可编程判定契约（RFC-013）；结构化建模（`match e.kind { file_not_found(path) => ... }`）依赖本 RFC 的变体解构与载荷绑定。

### 设计目标

- **变体解构可用**：`match r { ok(v) => ..., err(e) => ... }` 与用户自定义变体集（记录风格 sum type）的解构在三个执行路径语义一致。
- **穷尽性可依赖**：E1030/E1031 真实发射，match 漏分支是编译错误。
- **stub 清除**：删除"加载 0 永不匹配/误匹配"的占位行为，未支持模式在编译期报错而非静默走错。

## 提案（骨架级，待正文充实）

| 能力                       | 说明                                                                 |
| -------------------------- | -------------------------------------------------------------------- |
| Union 变体模式             | `VariantName` / `VariantName(binding)` 匹配 + 载荷绑定进 arm 作用域   |
| Struct / Tuple 模式        | 字段模式、元组模式落地（AST 已有，IR 补全）                           |
| Or 模式与 Guard            | `p1 \| p2` 与 `p if cond` 的求值顺序与绑定规则                        |
| 穷尽性检查                 | E1030 漏分支、E1031 不可达，作用域覆盖变体集（含内建 Result/Option）  |
| Identifier 模式语义确认    | 现语法中裸标识符是绑定还是变体比较，需定案（与通配 `_` 的关系）        |

### 开放问题

- [ ] Identifier 模式：绑定新变量还是匹配已存在值？（Rust 语义 vs 既有 YaoXiang 语料行为）
- [ ] 穷尽性的适用边界：动态导入/接口方法返回的变体集如何参与穷尽性判定？
- [ ] 是否需要 non_exhaustive 类属性（影响 std 变体集扩展对用户 match 的破坏半径）？
- [ ] stub 加载 0 误匹配在修复前的过渡期是否先报编译错？

## 关联

- RFC-013「运行时错误值与码贯通」演进节（`Error { kind, message }` 依赖本 RFC）
- RFC-036 测试模型（测试断言错误分支时的匹配写法）
- 已关个案不适用；本 RFC 为能力补全而非缺陷修复

---
title: RFC-009 v9 实现完整性审计报告
status: ongoing
created: 2026-05-29
---

# RFC-009 v9 实现完整性审计报告

## 审计范围

对照 RFC-009 v9 借用令牌系统设计文档，逐项检查编译器实现的完整性。覆盖类型系统、解析器、借用检查器、IR 生成、Dup 系统、闭包捕获六个维度。

---

## 1. 前端（类型系统 + 解析器）

| # | 检查项 | 状态 | 文件 | 说明 |
|---|--------|------|------|------|
| 1.1 | 词法 `&`/`&mut` | ✅ | `tokenizer.rs` L249-268, `tokens.rs` L75-76 | `Ampersand` 和 `MutRef` 两个 TokenKind 存在，`&&`(And) 不受影响 |
| 1.2 | AST `Type::Ref` | ✅ | `ast.rs` L474-480 | `Ref { mutable: bool, inner: Box<Type>, span: Span }` 字段齐全 |
| 1.3 | AST `Expr::Borrow` | ✅ | `ast.rs` L131-137 | `Borrow { mutable: bool, expr: Box<Expr>, span: Span }` 字段齐全 |
| 1.4 | 解析器 `&T`/`&mut T` 类型 | ✅ | `types.rs` L73-93 | `parse_type_annotation` 正确匹配 `Ampersand` 和 `MutRef` |
| 1.5 | 解析器 `&expr`/`&mut expr` | ✅ | `nud.rs` L38-39, L196-213 | `parse_borrow` 方法正确区分 mutable |
| 1.6 | MonoType `Ref { mutable, inner }` | ✅ | `mono.rs` L189-196 | 注释"编译期零大小类型，无运行时表示" |
| 1.7 | MonoType Display | ✅ | `mono.rs` L342-348 | 输出 `&T` 或 `&mut T` |
| 1.8 | From<ast::Type> 转换 | ✅ | `mono.rs` L556-559 | `Type::Ref` 正确转换为 `MonoType::Ref` |
| 1.9 | 类型检查器 Borrow 推断 | ✅ | `expressions.rs` L1096-1106 | 推断内部类型并包裹为 `MonoType::Ref` |
| 1.10 | Formatter | ✅ | `types.rs` L107-113, `expr.rs` L168-178 | `Type::Ref` 和 `Expr::Borrow` 格式化正确 |
| **1.11** | **Dup trait: `&T` Dup, `&mut T` 不 Dup** | **⚠️** | **`solver.rs` L201-233** | **`check_dup_trait` 没有显式匹配 `MonoType::Ref`，两者都落入 `_ => false`** |

---

## 2. 中端 — 借用检查器

| # | 检查项 | 状态 | 文件 | 说明 |
|---|--------|------|------|------|
| 2.1 | BorrowChecker 存在 | ✅ | `borrow_checker.rs` | 496 行，方法完整 |
| 2.2 | 令牌状态机 | ✅ | `borrow_checker.rs` | `Active`/`Frozen`/`Moved` 三种状态 |
| 2.3 | 多个 `&T` 同源允许（Dup） | ✅ | `borrow_checker.rs` L174 | 不可变+不可变组合不报错 |
| 2.4 | `&mut T` 活跃时创建 `&T` 报错 | ✅ | `borrow_checker.rs` L165-173 | 产生 `MutableBorrowConflict` |
| 2.5 | `&mut T` 活跃时创建 `&mut T` 报错 | ✅ | `borrow_checker.rs` L147-155 | 产生 `MutableBorrowConflict` |
| 2.6 | 冻结后使用报错 | ✅ | `borrow_checker.rs` L203-207 | 产生 `UseWhileFrozen` |
| 2.7 | 移动后使用报错 | ✅ | `borrow_checker.rs` L209-214 | 产生 `BorrowAfterMove` |
| 2.8 | 冻结机制 | ✅ | `borrow_checker.rs` | `&mut T` 可冻结为 `&T`，源 `&mut` 自动解冻 |
| 2.9 | OwnershipChecker 集成 | ✅ | `mod.rs` L122, L153-154 | `borrow_checker` 字段存在，`check_function` 中调用 |
| 2.10 | 错误类型 | ✅ | `error.rs` | `MutableBorrowConflict`/`BorrowAfterMove`/`UseWhileFrozen` 三变体 |
| **2.11** | **品牌机制（Brand）** | **❌** | **`borrow_checker.rs`** | **仅用变量名字符串追踪来源，无编期唯一 ID，无派生品牌链** |
| **2.12** | **`&mut T` IR 指令** | **❌** | **`ir.rs`** | **IR 中无创建 `&mut T` 的指令，`create_borrow(mutable=true)` 在 IR 分析中不可达** |

---

## 3. 中端 — IR 生成

| # | 检查项 | 状态 | 文件 | 说明 |
|---|--------|------|------|------|
| **3.1** | **IR Borrow/Release 指令** | **❌** | **`ir.rs`** | **`Instruction` 枚举无 `Borrow` 也无 `Release`，仅有 `ArcNew`/`ArcClone`/`ArcDrop`** |
| **3.2** | **ir_gen Expr::Borrow 处理** | **❌** | **`ir_gen.rs`** | **`generate_expr_ir` 完全没有 `Expr::Borrow` 分支，`&expr` 在 IR 阶段静默忽略** |
| 3.3 | MakeClosure env 填充 | ⚠️ | `ir_gen.rs` L3186-3201 | 捕获分析工作，但缺 ZST 优化（TODO 注释） |
| 3.4 | Bytecode type_id | ⚠️ | `bytecode.rs` L413 | 分配了 type_id 49，但无对应指令 |
| 3.5 | Bytecode From<MonoType> | ⚠️ | `bytecode.rs` L1418-1424 | 占位桩，所有类型映射为 `IrType::Void` |
| **3.6** | **解释器 Borrow 处理** | **❌** | **`execute.rs`** | **无 borrow 相关处理，`RuntimeValue` 无 borrow 变体** |
| **3.7** | **ZST 优化** | **❌** | **`ir_gen.rs`** | **`MonoType::Ref` 注释为 ZST，但 IR 生成无任何优化逻辑** |

---

## 4. Dup 系统（RFC-011 Section 2.4）

| # | 检查项 | 状态 | 文件 | 说明 |
|---|--------|------|------|------|
| 4.1 | Dup trait 注册 | ✅ | `std_traits.rs` L31 | `"Dup"` 在 `STD_TRAITS` 中 |
| 4.2 | is_marker 字段 | ✅ | `trait_data.rs` L31 | `pub is_marker: bool` 存在 |
| 4.3 | Dup implies Clone | ✅ | `std_traits.rs` L115 | `parent_traits: vec!["Clone"]` |
| 4.4 | 原语 Dup impl | ✅ | `std_traits.rs` L153-186 | Int/Float/Bool/Char/String/Bytes |
| 4.5 | Solver 递归检查 | ✅ | `solver.rs` L201-233 | 递归 Struct/Enum/Tuple 字段 |
| 4.6 | Auto-derive 泛型支持 | ✅ | `auto_derive.rs` | `Type::Generic`/`Type::Tuple` 递归处理 |
| 4.7 | Bounds 集成 | ✅ | `bounds.rs` L51-58 | 失败时尝试 auto-derive |
| **4.8** | **Send/Sync 残留** | **⚠️** | **`solver.rs`, `auto_derive.rs`, `send_sync.rs`** | **`check_send_trait`/`check_sync_trait` 方法残留；`BUILTIN_DERIVES` 含 Send/Sync；`send_sync.rs` 仍以 `pub mod` 导出** |

---

## 5. 闭包捕获（RFC-023）

| # | 检查项 | 状态 | 文件 | 说明 |
|---|--------|------|------|------|
| 5.1 | capture.rs 模块 | ✅ | `capture.rs` | ~1065 行，含测试 |
| 5.2 | 捕获分析 | ✅ | `capture.rs` L133-170 | 遍历 Lambda 体，区分 Read/Write |
| 5.3 | 逃逸分析 | ✅ | `capture.rs` L83-123 | Spawn/Return/赋值 → Escaping |
| 5.4 | 模式选择 | ✅ | `capture.rs` L186-206 | Dup→Copy, Escaping→Move, Inline→Borrow/BorrowMut |
| 5.5 | 集成到类型检查 | ✅ | `expressions.rs` L934-968 | Lambda 推断时调用捕获分析 |
| 5.6 | MakeClosure env 填充 | ✅ | `ir_gen.rs` L3177-3243 | `env: Vec::new()` 已替换为实际捕获变量 |

---

## 问题清单（按优先级排序）

### P0 — 阻塞项（借用令牌无法运行时工作）

| # | 问题 | 位置 | 影响 | 修复方向 |
|---|------|------|------|----------|
| P0-1 | IR 无 Borrow/Release 指令 | `ir.rs` | 借用令牌无法在 IR 中表示 | 新增 `Borrow { dst, src, mutable }` 和 `Release { src }` 指令 |
| P0-2 | `Expr::Borrow` IR 生成缺失 | `ir_gen.rs` | `&expr` 在 IR 阶段静默忽略 | 在 `generate_expr_ir` 中添加 Borrow 分支，生成 Borrow 指令 |
| P0-3 | 解释器无 Borrow 处理 | `execute.rs` | 运行时无法执行借用操作 | 添加 `BytecodeInstr::Borrow`/`Release` 处理 |

### P1 — 重要项（语义正确性）

| # | 问题 | 位置 | 影响 | 修复方向 |
|---|------|------|------|----------|
| P1-1 | `&T` 不满足 Dup | `solver.rs` L201-233 | 违背 RFC 核心语义："&T 可自由复制" | 添加 `MonoType::Ref { mutable: false, .. } => true` 分支 |
| P1-2 | 品牌机制缺失 | `borrow_checker.rs` | 无法追踪令牌派生关系 | 添加编期唯一 ID 和派生品牌链 |
| P1-3 | `&mut T` IR 指令不可达 | `borrow_checker.rs` | 可变借用冲突检测永不触发 | IR 层添加 MutRef 指令后自动解决 |

### P2 — 改进项（代码质量）

| # | 问题 | 位置 | 影响 | 修复方向 |
|---|------|------|------|----------|
| P2-1 | MakeClosure ZST 优化 | `ir_gen.rs` L3196-3198 | 令牌被捕获时产生无意义开销 | ZST 类型跳过 env |
| P2-2 | Send/Sync 残留 | `solver.rs`, `auto_derive.rs`, `send_sync.rs` | 代码残留 | 删除 `check_send_trait`/`check_sync_trait`、清理 `BUILTIN_DERIVES` |
| P2-3 | Bytecode From<MonoType> 占位 | `bytecode.rs` L1418-1424 | 所有类型映射为 Void | 实现真正的类型转换 |

---

## 实现状态总览

```
前端（类型系统 + 解析器）  ████████████████████░  91% (10/11)
中端（借用检查器）         ██████████████░░░░░░░  75% (9/12)
中端（IR 生成）            ░░░░░░░░░░░░░░░░░░░░   0% (0/3 核心)
Dup 系统                   ██████████████████░░  88% (7/8)
闭包捕获                   ████████████████████ 100% (6/6)
```

**整体完成度：~65%**。前端完整，中端借用检查器逻辑完整但 IR 层断裂。

---

## 建议实现顺序

1. **P1-1**: `solver.rs` 添加 `MonoType::Ref` Dup 匹配（1 行改动）
2. **P2-2**: 清理 Send/Sync 残留代码
3. **P0-1**: `ir.rs` 添加 Borrow/Release IR 指令
4. **P0-2**: `ir_gen.rs` 添加 `Expr::Borrow` IR 生成
5. **P0-3**: `execute.rs` 添加解释器 Borrow 处理
6. **P2-1**: MakeClosure ZST 优化
7. **P1-2**: 品牌机制（可延后，当前变量名追踪足够基本功能）

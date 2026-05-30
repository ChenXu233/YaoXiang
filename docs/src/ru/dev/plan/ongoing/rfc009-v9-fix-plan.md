```yaml
---
title: RFC-009 v9 问题修复方案
status: ongoing
created: 2026-05-29
---

# RFC-009 v9 问题修复方案

基于 [审计报告](rfc009-v9-implementation-audit.md) 的问题清单，逐项给出具体修复方案。

---

## 修复顺序总览

```
阶段 1（快速修复，1 行改动）:
  P1-1: solver.rs — &T Dup 匹配

阶段 2（清理残留）:
  P2-2: Send/Sync 残留清理

阶段 3（IR 层核心，P0 阻塞项）:
  P0-1: ir.rs — 添加 Borrow/Release IR 指令
  P0-2: ir_gen.rs — Expr::Borrow IR 生成
  P0-3: execute.rs — 解释器 Borrow 处理

阶段 4（IR 层补全）:
  P1-3: bytecode.rs — Borrow/Release 字节码指令 + Opcode
  P2-3: bytecode.rs — From<MonoType> 真正实现

阶段 5（优化项）:
  P2-1: ir_gen.rs — MakeClosure ZST 优化
  P1-2: borrow_checker.rs — 品牌机制（可延后）
```

---

## P1-1: solver.rs — `&T` Dup 匹配（1 行改动）

**问题**：`check_dup_trait` 没有显式匹配 `MonoType::Ref`，`&T` 和 `&mut T` 都落入 `_ => false`。RFC 核心语义是 "`&T` 可自由复制（Dup），`&mut T` 不可"。

**文件**：`src/frontend/core/typecheck/traits/solver.rs` L201-233

**当前代码**：
```rust
fn check_dup_trait(&self, ty: &MonoType) -> bool {
    match ty {
        MonoType::Int(_) | MonoType::Float(_) | MonoType::Bool
        | MonoType::Char | MonoType::String | MonoType::Bytes
        | MonoType::Void => true,
        MonoType::Arc(_) => true,
        MonoType::Tuple(elems) => elems.iter().all(|t| self.check_dup_trait(t)),
        MonoType::Struct(s) => s.fields.iter().all(|(_, ft)| self.check_dup_trait(ft)),
        MonoType::Enum(_) => true,
        _ => false,  // ← &T 和 &mut T 都落入这里
    }
}
```

**修复**：在 `MonoType::Arc(_)` 后添加一行：
```rust
// &T 是零大小令牌，可自由复制（Dup）；&mut T 不可
MonoType::Ref { mutable: false, .. } => true,
```

**完整修改后的 match**：
```rust
fn check_dup_trait(&self, ty: &MonoType) -> bool {
    match ty {
        MonoType::Int(_) | MonoType::Float(_) | MonoType::Bool
        | MonoType::Char | MonoType::String | MonoType::Bytes
        | MonoType::Void => true,
        MonoType::Arc(_) => true,
        // &T 是零大小令牌，可自由复制（Dup）；&mut T 不可
        MonoType::Ref { mutable: false, .. } => true,
        MonoType::Tuple(elems) => elems.iter().all(|t| self.check_dup_trait(t)),
        MonoType::Struct(s) => s.fields.iter().all(|(_, ft)| self.check_dup_trait(ft)),
        MonoType::Enum(_) => true,
        _ => false,
    }
}
```

**验证**：现有测试 + 新增测试用例：
```rust
#[test]
fn test_ref_is_dup() {
    let solver = create_test_solver();
    let ref_ty = MonoType::Ref { mutable: false, inner: Box::new(MonoType::Int(IntType::I64)) };
    assert!(solver.check_dup_trait(&ref_ty), "&T should be Dup");
}

#[test]
fn test_mut_ref_is_not_dup() {
    let solver = create_test_solver();
    let mut_ref_ty = MonoType::Ref { mutable: true, inner: Box::new(MonoType::Int(IntType::I64)) };
    assert!(!solver.check_dup_trait(&mut_ref_ty), "&mut T should NOT be Dup");
}
```

**影响范围**：纯逻辑修复，无破坏性。解决了闭包捕获分析中 `determine_capture_mode` 对 `&T` 变量的判断。

---

## P2-2: Send/Sync 残留清理

**问题**：`check_send_trait`/`check_sync_trait` 方法、`BUILTIN_DERIVES` 中的 Send/Sync、`send_sync.rs` 模块仍然存在。

**文件清单**：
1. `src/frontend/core/typecheck/traits/solver.rs` — 删除 `check_send_trait`/`check_sync_trait` 方法
2. `src/frontend/core/typecheck/traits/auto_derive.rs` — 从 `BUILTIN_DERIVES` 删除 Send/Sync 条目
3. `src/frontend/core/typecheck/traits/send_sync.rs` — 整个文件删除
4. `src/frontend/core/typecheck/traits/mod.rs` — 删除 `pub mod send_sync;`

**验证**：`cargo build` 编译通过 + 全部测试通过。

---

## P0-1: ir.rs — 添加 Borrow/Release IR 指令

**问题**：`Instruction` 枚举无 `Borrow` 也无 `Release`，借用令牌无法在 IR 中表示。

**文件**：`src/middle/core/ir.rs`

**设计方案**：参照 `ArcNew`/`ArcClone`/`ArcDrop` 的模式，添加两个指令：

```rust
// =====================
// 借用令牌指令
// =====================
/// 创建借用令牌：dst = &src（不可变）或 dst = &mut src（可变）
/// 借用令牌是零大小类型，编译后消失。
/// 此指令仅用于借用检查器的流敏感分析，运行时等价于 Mov。
Borrow {
    dst: Operand,
    src: Operand,
    mutable: bool,
},
/// 释放借用令牌，结束借用生命周期
/// 借用检查器据此更新令牌状态。
Release(Operand),
```

**插入位置**：在 `ArcDrop(Operand)` 之后（约 L268），`ShareRef` 之前。

**影响**：需要同步更新所有 `match` Instruction 的地方：
- `ir_gen.rs` 中的 IR → 字节码转换
- `borrow_checker.rs` 中的指令遍历

---

## P0-2: ir_gen.rs — `Expr::Borrow` IR 生成

**问题**：`generate_expr_ir` 完全没有 `Expr::Borrow` 分支，`&expr` 在 IR 阶段静默忽略。

**文件**：`src/middle/core/ir_gen.rs`

**设计方案**：在 `generate_expr_ir` 的 match 中添加 `Expr::Borrow` 分支。

**当前状态**：`Expr::Borrow` 仅在 `get_expr_span` 中被匹配（L1996），用于获取 span。

**添加的分支**（建议插入在 `Expr::Lambda` 分支之后）：

```rust
ast::Expr::Borrow { mutable, expr, span } => {
    // 1. 生成内部表达式的 IR
    let inner_reg = self.next_temp_reg();
    self.generate_expr_ir(expr, inner_reg, instructions, constants)?;

    // 2. 创建借用令牌指令
    // 借用令牌是零大小类型，运行时等价于 Mov。
    // 此指令的存在让借用检查器可以进行流敏感分析。
    instructions.push(Instruction::Borrow {
        dst: Operand::Local(result_reg),
        src: Operand::Local(inner_reg),
        mutable: *mutable,
    });
}
```

**关键点**：
- 借用令牌是 ZST，运行时不需要实际创建任何东西
- `Borrow` 指令在运行时退化为 `Mov`（dst = src）
- 但借用检查器在编译期通过分析 `Borrow`/`Release` 指令来检测冲突

---

## P0-3: execute.rs — 解释器 Borrow 处理

**问题**：解释器无 borrow 相关处理，`RuntimeValue` 无 borrow 变体。

**文件**：`src/backends/interpreter/executor/execute.rs`

**设计方案**：借用令牌是 ZST，运行时不需要特殊处理。`Borrow` 指令等价于 `Mov`，`Release` 指令等价于 `Nop`。

**添加的 match 分支**：

```rust
BytecodeInstr::Borrow { dst, src, mutable: _ } => {
    // 借用令牌是零大小类型，运行时等价于 Mov
    let val = frame
        .registers
        .get(src.0 as usize)
        .cloned()
        .unwrap_or(RuntimeValue::Unit);
    frame.set_register(dst.0 as usize, val);
    frame.advance();
}
BytecodeInstr::Release { src: _ } => {
    // 借用令牌释放，运行时无操作
    frame.advance();
}
```

**关键设计决策**：不需要给 `RuntimeValue` 添加 `Borrow` 变体。借用令牌的全部语义在编译期由借用检查器保证，运行时只做值传递。

---

## P1-3: bytecode.rs — Borrow/Release 字节码指令

**问题**：`BytecodeInstr` 枚举无 Borrow/Release，IR → 字节码转换无法处理这些指令。

**文件**：`src/middle/core/bytecode.rs`

**需要修改的位置**：

### 1. `BytecodeInstr` 枚举（约 L322，Arc 操作区之后）

```rust
// =====================
// Borrow Token Operations
// =====================
/// Create borrow token (ZST, runtime ≈ Mov)
Borrow {
    dst: Reg,
    src: Reg,
    mutable: bool,
},
/// Release borrow token (ZST, runtime ≈ Nop)
Release {
    src: Reg,
},
```

### 2. `Opcode` 枚举

```rust
Borrow,
Release,
```

### 3. `opcode()` 方法

```rust
BytecodeInstr::Borrow { .. } => Opcode::Borrow,
BytecodeInstr::Release { .. } => Opcode::Release,
```

### 4. `size()` 方法

```rust
BytecodeInstr::Borrow { .. } => 5,   // dst(2) + src(2) + mutable(1)
BytecodeInstr::Release { .. } => 2,  // src(2)
```

### 5. IR → 字节码转换（`ir_to_bytecode` 或等价函数）

在处理 `Instruction::Borrow` 时：
```rust
Instruction::Borrow { dst, src, mutable } => {
    BytecodeInstr::Borrow {
        dst: self.operand_to_reg(dst),
        src: self.operand_to_reg(src),
        mutable: *mutable,
    }
}
Instruction::Release(src) => {
    BytecodeInstr::Release {
        src: self.operand_to_reg(src),
    }
}
```

### 6. 字节码反序列化（`decode_instructions`）

```rust
Opcode::Borrow => {
    // Borrow: dst(2) + src(2) + mutable(1)
    if instr.operands.len() >= 5 {
        let dst = u16::from_le_bytes([instr.operands[0], instr.operands[1]]);
        let src = u16::from_le_bytes([instr.operands[2], instr.operands[3]]);
        let mutable = instr.operands[4] != 0;
        decoded_instructions.push(BytecodeInstr::Borrow {
            dst: Reg(dst),
            src: Reg(src),
            mutable,
        });
    }
}
Opcode::Release => {
    // Release: src(2)
    if instr.operands.len() >= 2 {
        let src = u16::from_le_bytes([instr.operands[0], instr.operands[1]]);
        decoded_instructions.push(BytecodeInstr::Release { src: Reg(src) });
    }
}
```

---

## P2-3: bytecode.rs — `From<MonoType>` 真正实现

**问题**：`From<MonoType> for IrType` 是占位桩，所有类型映射为 `IrType::Void`。

**文件**：`src/middle/core/bytecode.rs` L1418-1424

**当前代码**：
```rust
impl From<MonoType> for IrType {
    fn from(_: MonoType) -> Self {
        IrType::Void  // 占位
    }
}
```

**修复**：
```rust
impl From<MonoType> for IrType {
    fn from(ty: MonoType) -> Self {
        match ty {
            MonoType::Int(_) => IrType::I64,
            MonoType::Float(_) => IrType::F64,
            MonoType::Bool => IrType::Bool,
            MonoType::Char => IrType::Char,
            MonoType::String => IrType::String,
            MonoType::Bytes => IrType::Bytes,
            MonoType::Void => IrType::Void,
            MonoType::List(_) => IrType::List,
            MonoType::Tuple(_) => IrType::Tuple,
            MonoType::Struct(_) => IrType::Struct,
            MonoType::Enum(_) => IrType::Enum,
            MonoType::Fn { .. } => IrType::Function,
            MonoType::Arc(_) => IrType::Arc,
            MonoType::Weak(_) => IrType::Weak,
            MonoType::Ref { .. } => IrType::Void,  // ZST，无运行时表示
            _ => IrType::Void,
        }
    }
}
```

**注意**：需确认 `IrType` 枚举是否已定义这些变体。如果没有，需要先扩展 `IrType`。

---

## P2-1: ir_gen.rs — MakeClosure ZST 优化

**问题**：`&T` 令牌被捕获时产生无意义的 env 开销。

**文件**：`src/middle/core/ir_gen.rs` L3196-3198

**当前代码**：
```rust
for captured in &captured_vars {
    if let Some(local_idx) = self.lookup_local(&captured.name) {
        // TODO: ZST 优化 — 如果被捕获的变量是 ZST（如 &T 令牌），
        // 应跳过它，因为它没有运行时表示。
        env_vars.push(Operand::Local(local_idx));
    }
}
```

**修复**：
```rust
for captured in &captured_vars {
    if let Some(local_idx) = self.lookup_local(&captured.name) {
        // ZST 优化：借用令牌是零大小类型，跳过 env
        if let Some(type_result) = &self.type_result {
            if let Some(mono_type) = type_result.local_var_types.get(&captured.name) {
                if matches!(mono_type, MonoType::Ref { .. }) {
                    continue;  // ZST，不需要加入 env
                }
            }
        }
        env_vars.push(Operand::Local(local_idx));
    }
}
```

---

## P1-2: 品牌机制（可延后）

**问题**：仅用变量名字符串追踪来源，无编译期唯一 ID。

**影响**：当前变量名追踪足够支撑基本功能。品牌机制是防御性增强，防止：
1. 不同来源的同名变量被误判为同一来源
2. 令牌被"伪造"（编译器内部一致性）

**延后理由**：
- 当前所有变量在同一函数内是唯一的（SSA 或作用域隔离）
- 品牌机制是编译器内部优化，不影响用户可见行为
- 实现复杂度高，需要在 `BorrowToken` 中添加 `brand_id: u64` 和派生链

**建议**：在 P0/P1 完成后、进入 LLVM 后端前实现。

---

## 测试策略

### 新增测试

1. **solver.rs**：`&T` Dup / `&mut T` 非 Dup 单元测试
2. **ir_gen.rs**：`Expr::Borrow` IR 生成测试（验证生成 `Instruction::Borrow`）
3. **execute.rs**：借用令牌运行时测试（验证 `Borrow` ≈ `Mov`）
4. **borrow_checker.rs**：已有 11 个测试，验证 `Borrow`/`Release` 指令集成

### 回归测试

- `cargo test` 全量通过（当前 2125 测试）
- `cargo build` 编译无警告

---

## 文件修改清单

| 文件 | 改动类型 | 优先级 |
|------|----------|--------|
| `src/frontend/core/typecheck/traits/solver.rs` | +1 行 match 分支 | P1-1 |
| `src/frontend/core/typecheck/traits/send_sync.rs` | 删除文件 | P2-2 |
| `src/frontend/core/typecheck/traits/mod.rs` | -1 行 mod 声明 | P2-2 |
| `src/frontend/core/typecheck/traits/auto_derive.rs` | 删除 Send/Sync 条目 | P2-2 |
| `src/middle/core/ir.rs` | +10 行（Borrow/Release 指令） | P0-1 |
| `src/middle/core/ir_gen.rs` | +15 行（Expr::Borrow 分支 + ZST 优化） | P0-2, P2-1 |
| `src/middle/core/bytecode.rs` | +30 行（指令 + Opcode + 编解码） | P1-3, P2-3 |
| `src/backends/interpreter/executor/execute.rs` | +12 行（Borrow/Release 处理） | P0-3 |

**总改动量**：约 70 行新增代码 + 1 个文件删除。
```
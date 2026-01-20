# Task 12-05: VM 指令完整性与栈操作修复

> **优先级**: 🔴 高
> **状态**: ✅ 所有阶段已完成
> **预估工时**: 3-4 天

## 问题背景

经过对 VM 实现 (`src/vm/`) 的全面审查，发现以下严重问题：

1. **大量指令未实现** - 静默忽略
2. **`value_stack` 从未使用** - 架构缺陷
3. **I32/位运算/F32 等指令完全缺失**

## 实施进度

| 阶段 | 状态 | 完成内容 |
|------|------|---------|
| 阶段 1 | ✅ 已完成 | 架构修复（错误处理 + value_stack） |
| 阶段 2 | ✅ 已完成 | I32 和位运算指令（18个指令） |
| 阶段 3 | ✅ 已完成 | F32 和 F64 剩余指令（21个指令） |
| 阶段 4 | ✅ 已完成 | 字符串/闭包/异常/内存操作指令（~25个指令） |
| 阶段 5 | ✅ 已完成 | 集成测试（30个测试覆盖所有新指令） |

## 问题清单

### 🔴 高优先级问题

| # | 问题 | 文件:行号 | 严重程度 | 状态 |
|---|------|----------|---------|------|
| 1 | `value_stack` 定义但从未使用 | `executor.rs:140` | 🔴 架构缺陷 | ✅ 已修复 |
| 2 | 未实现指令静默忽略（`_ => {}`） | `executor.rs:732-735` | 🔴 静默失败 | ✅ 已修复 |
| 3 | I32 运算指令完全缺失（12个） | `opcode.rs:154-196` | 🔴 功能缺失 | ✅ 已修复 |
| 4 | I64 位运算指令完全缺失（6个） | `opcode.rs:116-132` | 🔴 功能缺失 | ✅ 已修复 |

### 🟡 中优先级问题

| # | 问题 | 文件:行号 | 严重程度 | 状态 |
|---|------|----------|---------|------|
| 5 | F32 运算指令缺失（10个） | `opcode.rs:236-264` | 🟡 功能缺失 | ✅ 已修复 |
| 6 | F64Rem 取模指令缺失 | `opcode.rs:214` | 🟡 功能缺失 | ✅ 已修复 |
| 7 | F32 比较指令缺失（6个） | `opcode.rs:266-303` | 🟡 功能缺失 | ✅ 已修复 |
| 8 | 字符串操作指令缺失（6个） | `opcode.rs:386-408` | 🟡 功能缺失 | ✅ 已修复 |
| 9 | 闭包操作指令缺失（4个） | `opcode.rs:367-381` | 🟡 功能缺失 | ✅ 已修复 |
| 10 | 异常处理指令缺失（4个） | `opcode.rs:413-425` | 🟡 功能缺失 | ✅ 已修复 |

### 🟢 低优先级问题

| # | 问题 | 文件:行号 | 严重程度 |
|---|------|----------|---------|
| 11 | SetField 编码跳号（0x74 未使用） | `opcode.rs:324` | 🟢 代码规范 |
| 12 | 缺少指令执行集成测试 | `tests/` | 🟢 测试覆盖 |
| 13 | ReturnValue 返回值未处理 | `executor.rs:386-388` | 🟢 功能缺陷 |
| 14 | TryBegin 操作数数量不匹配 | `opcode.rs:784` vs `opcode.rs:413` | 🟢 代码规范 |

## 统计数据

```
opcode.rs 定义指令数: ~110+
executor.rs 已实现数: ~25
实现率: ~23%
```

## 实施计划

### 阶段 1：架构修复（Day 1）

#### 1.1 修复未实现指令的错误处理

**文件**: `src/vm/executor.rs`

**修改位置**: `execute_instruction` 函数的 match 分支末尾

**修改内容**:
```rust
// 当前代码
_ => {
    // 未实现的指令
}

// 修改为
_ => return Err(VMError::UnimplementedOpcode(opcode)),
```

**预期结果**: 未实现指令时返回错误而不是静默忽略

#### 1.2 添加 UnimplementedOpcode 错误变体

**文件**: `src/vm/errors.rs`

**修改内容**:
```rust
#[error("Unimplemented opcode: {0}")]
UnimplementedOpcode(u8),
```

**预期结果**: VMError 枚举包含未实现指令的错误类型

#### 1.3 实现 value_stack 的基本操作

**文件**: `src/vm/executor.rs`

**修改内容**: 在以下位置使用 `value_stack`:
- `CallStatic` 执行前弹出参数
- `ReturnValue` 执行前压入返回值

**预期结果**: 值栈开始被实际使用

---

### 阶段 2：补齐 I32 和位运算指令（Day 2）

#### 2.1 实现 I32 算术运算（6个）

**文件**: `src/vm/executor.rs`

**实现指令**:
- `I32Add`
- `I32Sub`
- `I32Mul`
- `I32Div`
- `I32Rem`
- `I32Neg`

**实现模式**:
```rust
I32Add => {
    let dst = self.read_u8()?;
    let lhs = self.read_u8()?;
    let rhs = self.read_u8()?;
    let result = self.binary_op_i32(lhs, rhs, |a, b| Ok(a + b))?;
    self.regs.write(dst, RuntimeValue::Int(result));
}
```

**新增辅助方法**:
```rust
fn binary_op_i32<F>(&self, lhs_reg: u8, rhs_reg: u8, op: F) -> Result<i64, VMError>
where
    F: FnOnce(i64, i64) -> Result<i64, VMError>,
```

#### 2.2 实现 I32 位运算（6个）

**实现指令**:
- `I32And`
- `I32Or`
- `I32Xor`
- `I32Shl`
- `I32Sar`
- `I32Shr`

#### 2.3 实现 I64 位运算（6个）

**实现指令**:
- `I64And`
- `I64Or`
- `I64Xor`
- `I64Shl`
- `I64Sar`
- `I64Shr`

---

### 阶段 3：补齐 F32 和 F64 剩余指令（Day 3）

#### 3.1 实现 F32 算术运算（10个）

**实现指令**:
- `F32Add`, `F32Sub`, `F32Mul`, `F32Div`, `F32Rem`
- `F32Sqrt`, `F32Neg`
- `F32Load`, `F32Store`, `F32Const`

#### 3.2 实现 F64Rem

**实现指令**: `F64Rem`

#### 3.3 实现 F32 比较指令（6个）

**实现指令**:
- `F32Eq`, `F32Ne`, `F32Lt`, `F32Le`, `F32Gt`, `F32Ge`

---

### 阶段 4：补齐高级指令（Day 4）

#### 4.1 字符串操作指令（6个）

**实现指令**:
- `StringLength`
- `StringConcat`
- `StringEqual`
- `StringGetChar`
- `StringFromInt`
- `StringFromFloat`

#### 4.2 闭包操作指令（4个）

**实现指令**:
- `MakeClosure`
- `LoadUpvalue`
- `StoreUpvalue`
- `CloseUpvalue`

#### 4.3 异常处理指令（4个）

**实现指令**:
- `TryBegin`
- `TryEnd`
- `Throw`
- `Rethrow`

---

### 阶段 5：代码规范修复

#### 5.1 修复 SetField 编码

**文件**: `src/vm/opcode.rs`

**修改**: 将 `SetField` 从 `0x75` 改为 `0x74`，移除编码跳号

#### 5.2 修复 TryBegin 操作数数量

**文件**: `src/vm/opcode.rs`

**修改**: 确认 `TryBegin` 操作数数量与实际使用一致

#### 5.3 添加集成测试

**文件**: `src/vm/tests/opcode.rs`

**新增测试**:
- 指令执行集成测试
- 栈操作测试
- 边界情况测试

## 验收标准

### 必须满足（Must Have）

- [x] 未实现指令不再静默忽略，返回明确错误
- [x] value_stack 开始被实际使用
- [x] I32 全部 12 个指令实现完成
- [x] I64 位运算全部 6 个指令实现完成
- [x] 所有修改通过现有测试

### 最好有（Nice to Have）

- [x] F32 全部指令实现
- [x] 字符串操作指令实现
- [x] 闭包操作指令实现
- [x] 新增集成测试覆盖（30个测试）

## 风险与依赖

### 风险

1. **API 变更风险**: 修改 `VMError` 可能影响上游代码
   - **缓解**: 仅新增变体，不修改现有变体

2. **编码变更风险**: 修改 SetField 编码可能导致字节码不兼容
   - **缓解**: 此修改应在正式发布前完成

### 依赖

- 无外部依赖
- 修改仅涉及 `src/vm/` 目录

## 测试策略

1. **单元测试**: 每个新指令添加对应测试
2. **集成测试**: 端到端测试实际程序执行
3. **回归测试**: 确保现有测试通过

## 参考资料

- `src/vm/opcode.rs` - 指令定义
- `src/vm/executor.rs` - 执行器实现
- `src/vm/errors.rs` - 错误定义
- `src/runtime/value.rs` - RuntimeValue 类型

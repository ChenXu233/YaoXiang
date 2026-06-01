---
title: "代码生成状态"
---

# 代码生成（Codegen）

> **模块状态**：已完成（基本功能）
> **位置**：`src/middle/passes/codegen/`
> **最后更新**：2026-06-01

---

## 模块概述

代码生成模块负责将 IR（中间表示）翻译为字节码。支持完整的 `.yx` 字节码文件格式（.42 格式），包含调试信息段。

**代码量**：约 2400 行（7 个源文件）

---

## 功能清单

### 翻译器（translator.rs - 1,073 行）

**算术/位运算（全部完成）**：
- ✅ Add, Sub, Mul, Div, Mod, And, Or, Xor, Shl, Shr, Sar, Neg

**比较运算（全部完成）**：
- ✅ Eq, Ne, Lt, Le, Gt, Ge

**控制流（全部完成）**：
- ✅ Jmp, JmpIf, JmpIfNot, Ret，含跳转偏移回填机制

**函数调用（全部完成）**：
- ✅ CallStatic, CallNative, CallVirt, CallDyn, TailCall

**变量操作（全部完成）**：
- ✅ Move, Load, Store

**内存/对象操作（全部完成）**：
- ✅ Alloc, AllocArray, HeapAlloc
- ✅ LoadField, StoreField, LoadIndex, StoreIndex
- ✅ CreateStruct

**所有权系统（全部完成）**：
- ✅ Drop, ArcNew, ArcClone, ArcDrop
- ✅ Borrow, Release（借用令牌）
- ✅ ShareRef（临时用 Nop 实现）

**闭包/Upvalue（全部完成）**：
- ✅ MakeClosure, LoadUpvalue, StoreUpvalue, CloseUpvalue

**字符串操作（全部完成）**：
- ✅ StringLength, StringConcat, StringGetChar, StringFromInt, StringFromFloat

**并发（部分完成）**：
- ✅ Spawn
- ✅ EvalPush, EvalPop, Yield

### 字节码文件格式（bytecode.rs）

- ✅ 完整的 `BytecodeFile` 结构：文件头 + 类型表 + 常量池 + 代码段 + 可选调试信息段
- ✅ 文件头：魔数 `YXBC` (0x59584243)，版本号 2
- ✅ 支持混合端序：魔数大端序，数据小端序
- ✅ 支持常量类型：Void, Bool, Int, Float, Char, String, Bytes
- ✅ **调试信息段**（DebugSection）：支持源码位置映射（IP -> Span）

### 操作码系统（opcode.rs，shared）

- ✅ 共定义了 **80+ 个 Opcode**，分为 12 个类别
- ✅ 完整的 `TryFrom<u8>` 实现
- ✅ 各种辅助判断方法：`is_numeric_op`, `is_call_op`, `is_jump_op` 等

---

## 未实现/占位的指令

| IR 指令 | 当前实现 | 说明 |
|---------|----------|------|
| ShareRef | Nop | 代码中有 TODO 注释 |
| Free | Nop | 无操作 |
| Dup, Swap | Nop | 栈操作暂未实现 |
| UnsafeBlockStart/End | Nop | unsafe 块标记 |
| PtrFromRef/PtrDeref/PtrStore/PtrLoad | 全部 Nop | 裸指针操作暂不支持 |
| TypeTest | 占位 TypeCheck | 操作数硬编码为 [0, 0, 0] |
| Cast | 操作数硬编码 | 目标类型未编码 |

---

## 测试覆盖

**13 个单元测试**，全部通过：

| 测试文件 | 测试数量 | 覆盖范围 |
|----------|----------|----------|
| `mod.rs` | 1 | 基本上下文创建 |
| `buffer.rs` | 2 | 常量池添加/获取、字节码缓冲区发射 |
| `emitter.rs` | 2 | 指令发射与映射、跳转待回填 |
| `operand.rs` | 2 | 寄存器转换、溢出检测 |
| `flow.rs` | 5 | 标签生成器、寄存器分配器、流管理器、符号表基础、作用域嵌套 |
| `bytecode.rs` | 1 | DebugSection 的编解码往返测试 |

**测试不足之处**：
- ❌ translator.rs 没有独立测试
- ❌ 没有集成测试覆盖完整 `generate()` 流程
- ❌ 没有端到端测试（源码 -> 字节码 -> 反序列化验证）

---

## 代码质量评估

| 维度 | 评分 | 说明 |
|------|------|------|
| 功能完成度 | 85% | 核心翻译流程完整，少数指令占位 |
| 测试覆盖 | 不足 | 约 30%，核心 translator 缺测试 |
| 文档 | 中等 | 有模块和类型文档，缺格式规范和用户文档 |
| 代码质量 | 良好 | 架构清晰，职责分明，但 translator.rs 过大 |
| RFC 一致性 | 基本一致 | 满足 VM 后端需求，LLVM 后端待实现 |

---

## 待改进项

1. **补充 translator.rs 单元测试**
2. **实现 ShareRef/Dup/Swap 等占位指令**
3. **实现 unsafe/指针指令**
4. **拆分 translator.rs**（1,073 行过大）
5. **添加字节码格式规范文档**

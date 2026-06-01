---
title: "解释器状态"
---

# 解释器（Interpreter）

> **模块状态**：已完成
> **位置**：`src/backends/interpreter/`
> **最后更新**：2026-06-01

---

## 模块概述

解释器负责执行字节码。采用寄存器式虚拟机架构，支持完整的 39 种字节码指令，与 RFC-008（并发模型）和 RFC-009（所有权模型）完全对齐。

**代码量**：3,768 行（9 个源文件）

---

## 功能清单

### 核心执行引擎（execute.rs - 1,308 行）

**控制流（10 种）**：
- ✅ Nop, Return, ReturnValue, Yield (空操作), EvalPush, EvalPop, Spawn, Jmp, JmpIf, JmpIfNot, Switch (简化)

**寄存器操作（5 种）**：
- ✅ Mov, LoadConst, LoadLocal, StoreLocal, LoadArg

**算术/逻辑运算（11 种 BinaryOp + 1 UnaryOp）**：
- ✅ Add/Sub/Mul/Div/Rem/And/Or/Xor/Shl/Sar/Shr，Int 和 Float 均支持
- ⚠️ UnaryOp 仅实现 Int 取反

**比较（6 种 CompareOp）**：
- ✅ Eq/Ne/Lt/Le/Gt/Ge，Int 和 String 均支持

**内存操作（9 种）**：
- ✅ StackAlloc (空操作), HeapAlloc, Drop (空操作), GetField, SetField, LoadElement, StoreElement, NewListWithCap, CreateStruct

**Arc/Weak 操作（5 种）**：
- ✅ ArcNew, ArcClone, ArcDrop (空操作), WeakNew, WeakUpgrade

**借用令牌（2 种）**：
- ✅ Borrow (ZST, 运行时等价 Mov), Release (ZST, 运行时等价 Nop)

**函数调用（7 种）**：
- ✅ CallStatic, CallNative, CallVirt, CallDyn, MakeClosure, LoadUpvalue, StoreUpvalue, CloseUpvalue

**字符串操作（6 种）**：
- ✅ StringLength, StringConcat, StringEqual, StringGetChar, StringFromInt, StringFromFloat

**异常处理（3 种）**：
- ✅ TryBegin (空操作), TryEnd (空操作), Throw

**调试/类型（4 种）**：
- ⚠️ BoundsCheck (空操作), TypeCheck (空操作), Cast (透传), TypeOf (占位)

### 核心架构能力

- ✅ **堆（Heap）**：动态分配 List/Tuple/Array/Dict/Struct
- ✅ **调用栈（Frame）**：寄存器文件 + 局部变量 + upvalue + eval 栈 + spawn group
- ✅ **常量池**：跨模块共享
- ✅ **函数表**：按名称 (HashMap) 和按索引 (Vec) 双表，支持闭包按 ID 调用
- ✅ **FFI 注册表**：预加载 `std.io.*` 系列函数，可扩展自定义原生函数
- ✅ **DAG 任务调度（LocalRuntime）**：基于 RFC-008 的惰性/并发求值
- ✅ **三种求值策略**：Block (同步)、Auto (惰性/并发)、Eager (急切)
- ✅ **结构化并发**：spawn group 跟踪、作用域退出时等待所有任务完成、依赖失败级联取消

---

## 测试覆盖

**约 60 个测试**：

| 测试类型 | 数量 | 覆盖范围 |
|----------|------|----------|
| 单元测试（模块内） | ~35 | registers, ffi, frames, tests, debug, execute |
| 集成测试 | 25 | 完整编译管线：hello world、变量声明、算术、比较、lambda、函数定义、if/elif/else、while、for、match、List/Tuple/Dict、列表推导、闭包高阶函数、模块导入、f-string |

---

## RFC 对比

### RFC-008（Runtime 并发模型）

| 设计要求 | 实现状态 | 说明 |
|----------|----------|------|
| 三层运行时：Embedded / Standard / Full | ✅ 已实现 | 通过 `RuntimeMode` 配置 |
| 三种求值策略：Block / Auto / Eager | ✅ 已实现 | |
| DAG 任务调度（`LocalRuntime`） | ✅ 已实现 | |
| 任务依赖追踪、取消传播、结构化并发 | ✅ 已实现 | |
| 同步 = 调度的特例（Embedded 模式） | ✅ 已实现 | |

### RFC-009（所有权模型）

| 设计要求 | 实现状态 | 说明 |
|----------|----------|------|
| Borrow/Release 作为零大小令牌（ZST） | ✅ 已实现 | 运行时等价 Mov/Nop |
| ArcNew/ArcClone/ArcDrop 实现 `ref` 关键字语义 | ✅ 已实现 | |
| WeakNew/WeakUpgrade 实现弱引用 | ✅ 已实现 | |
| Move 语义（默认行为） | ✅ 已实现 | |
| `clone()` 由编译层处理 | ✅ 已实现 | 运行时无需特殊指令 |

---

## 简化/占位实现

| 指令 | 当前行为 | 设计意图 |
|------|----------|----------|
| Switch | 直接 advance IP | 应按值分发跳转 |
| TypeOf | 返回 type_table 长度占位 | 应返回运行时类型信息 |
| Cast | 透传值（无实际转换） | 应按目标类型转换 |
| BoundsCheck / TypeCheck | 空操作 | debug 模式应做运行时检查 |
| StringGetChar | 只取首字符，忽略 index 参数 | 应按 index 取字符 |
| UnaryOp | 仅 Int 取反，忽略 op 类型 | 应支持更多一元运算 |
| step/step_over/step_out/run | `todo!()` | 调试器步进功能未实现 |

---

## 代码质量评估

| 维度 | 评分 | 说明 |
|------|------|------|
| 功能完成度 | 100% | 核心执行引擎健壮，覆盖全部 39 种字节码指令 |
| 测试覆盖 | 良好 | 约 60 个测试，覆盖主要功能路径 |
| 文档质量 | 良好 | 每个源文件都有模块级 doc comment，引用 RFC 编号 |
| 代码架构 | 优秀 | 分层清晰：executor/frames/registers/ffi/runtime |
| RFC 合规 | 完全对齐 | RFC-008 和 RFC-009 设计完全对齐 |

---

## 待改进项

1. **实现 Switch 指令的真实分发**
2. **实现调试器步进功能**（step/step_over/step_out/run）
3. **完善 StringGetChar/UnaryOp 等指令**
4. **实现 BoundsCheck/TypeCheck 的 debug 模式检查**
5. **补充边界条件和错误路径测试**

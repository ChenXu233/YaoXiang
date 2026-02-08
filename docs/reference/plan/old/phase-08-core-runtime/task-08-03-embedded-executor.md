# Task 8.3: 嵌入式运行时（即时执行器）

> **优先级**: P1
> **状态**: ✅ 已完成
> **模块**: `src/embedded/executor.rs`
> **依赖**: task-08-01-value-type, task-08-02-allocator

## 功能描述

实现嵌入式运行时的即时执行器，用于 WASM/游戏脚本等资源受限场景。

### 核心特性

- **即时执行**：读取字节码后立即执行，无调度开销
- **同步执行**：按顺序执行所有操作
- **无 DAG**：不使用依赖图
- **无调度器**：没有任务调度逻辑
- **忽略 spawn**：spawn 标记被当作普通函数调用

### 文件结构

```
src/embedded/
├── mod.rs              # 导出入口
├── executor.rs         # 即时执行器
└── tests/
    └── executor.rs     # 测试
```

## 实现进度

### ✅ P0: 核心基础（已完成）
- [x] EmbeddedRuntime 结构
- [x] Interpreter 结构
- [x] Frame 结构（带 upvalue）
- [x] 基础指令：Nop, Mov, LoadConst, LoadLocal, StoreLocal, LoadArg
- [x] 控制流：Return, ReturnValue, Jmp, JmpIf, JmpIfNot
- [x] 整数运算：I64/I32 Add/Sub/Mul/Div/Rem
- [x] 整数位运算：I64And/Or/Xor/Shl/Sar/Shr, I64Neg
- [x] 浮点运算：F64/F32 Add/Sub/Mul/Div/Rem/Sqrt/Neg
- [x] 比较指令：I64/F64 Eq/Ne/Lt/Le/Gt/Ge, F32 比较
- [x] 运行时错误类型
- [x] P0 测试（5 个测试）

### ✅ P1: 函数调用（已完成）
- [x] CallStatic 指令
- [x] MakeClosure 闭包创建
- [x] LoadUpvalue / StoreUpvalue / CloseUpvalue
- [x] CallDyn 动态调用
- [x] CallVirt 虚函数调用
- [x] TailCall 尾调用优化
- [x] P1 测试（6 个新测试）

### ✅ P2: 内存与对象（已完成）
- [x] StackAlloc 栈分配
- [x] HeapAlloc 堆分配
- [x] Drop 释放
- [x] GetField / SetField 字段访问
- [x] LoadElement / StoreElement 元素访问
- [x] NewListWithCap 列表创建
- [x] ArcNew / ArcClone / ArcDrop 引用计数
- [x] P2 测试（11 个新测试）

### ✅ P3: 字符串操作（已完成）
- [x] StringLength 长度获取
- [x] StringConcat 拼接
- [x] StringEqual 相等比较
- [x] StringGetChar 获取字符
- [x] StringFromInt / StringFromFloat 类型转换
- [x] P3 测试（6 个新测试）

### ✅ P4: 高级特性（已完成）
- [x] Switch 分支表跳转
- [x] LoopStart / LoopInc 循环优化
- [x] TryBegin / TryEnd / Throw / Rethrow 异常处理
- [x] BoundsCheck 边界检查
- [x] TypeCheck / Cast 类型操作
- [x] TypeOf 反射操作
- [x] Spawn 指令（视为 CallStatic）
- [x] Yield 指令（空操作）
- [x] P4 测试（15 个新测试）

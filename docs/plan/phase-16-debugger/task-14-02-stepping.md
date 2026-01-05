# Task 14.2: 单步执行

> **优先级**: P1
> **状态**: ⏳ 待实现

## 功能描述

支持单步执行（step into, step over, step out）。

## 单步类型

```rust
/// 单步模式
enum StepMode {
    /// 进入函数调用
    StepInto,
    /// 跳过函数调用
    StepOver,
    /// 跳出函数
    StepOut,
    /// 单步执行一条指令
    StepInstruction,
}

/// 单步状态
struct StepState {
    mode: StepMode,
    target_frame: usize,  // 目标栈帧
    target_pc: Option<usize>,  // 目标 PC
}
```

## 相关文件

- **stepping.rs**: SteppingManager

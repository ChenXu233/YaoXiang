# Task 12.4: 中断处理

> **优先级**: P1
> **状态**: ✅ 已实现

## 功能描述

处理虚拟机外部中断（Timeout、Breakpoint、StackOverflow、MemoryViolation），由调度器在 DAG 节点边界控制检查时机（最小开销）。

## 设计决策

| 决策 | 说明 |
|------|------|
| 中断类型 | 仅外部中断，移除 GcRequest（无 GC） |
| 错误处理 | 使用 RFC-001 的 `Result[T, E]` 模式 |
| 检查时机 | 由调度器控制，在 DAG 节点边界检查 |

## 中断类型

**文件**: [src/runtime/interrupt.rs](../../../../src/runtime/interrupt.rs)

```rust
/// 外部中断类型
enum Interrupt {
    /// 执行超时
    Timeout(Duration),
    /// 调试断点
    Breakpoint(BreakpointId),
    /// 栈溢出
    StackOverflow,
    /// 内存访问违规
    MemoryViolation {
        address: usize,
        access: AccessType,
    },
}

/// 内存访问类型
enum AccessType {
    Read = 0,
    Write = 1,
    Execute = 2,
}

/// 线程安全的中断状态存储
struct InterruptState {
    interrupt_type: AtomicU8,  // 0=None, 1=Timeout, 2=Breakpoint, 3=StackOverflow, 4=MemoryViolation
    arg0: AtomicU64,           // duration_ns / address / breakpoint_id
    arg1: AtomicU64,           // access_type / breakpoint_id low bits
}

type InterruptHandler = Arc<InterruptState>;
```

## 调度器集成

**文件**: [src/runtime/scheduler/mod.rs](../../../../src/runtime/scheduler/mod.rs)

```rust
impl FlowScheduler {
    /// 获取中断处理器（供外部系统注入中断）
    pub fn interrupt_handler(&self) -> &InterruptHandler

    /// 在 DAG 节点边界检查并清除中断
    pub fn check_interrupt(&self) -> Option<Interrupt>

    /// 检查是否有待处理的中断（不清除）
    pub fn has_interrupt(&self) -> bool

    /// 清除中断状态
    pub fn clear_interrupt(&self)
}
```

## 错误类型

**文件**: [src/vm/errors.rs](../../../../src/vm/errors.rs)

```rust
enum VMError {
    // ... 现有错误

    // === 中断相关错误 ===
    Timeout(Duration),
    Breakpoint(BreakpointId),
    MemoryViolation { addr: usize, access: AccessType },
}
```

## 使用示例

```rust
use yaoxiang::runtime::scheduler::FlowScheduler;
use yaoxiang::runtime::interrupt::{Interrupt, BreakpointId, AccessType};
use std::sync::Arc;
use std::time::Duration;

// 创建调度器
let scheduler = FlowScheduler::new();

// 获取中断处理器（可在其他线程使用）
let handler = scheduler.interrupt_handler().clone();

// 在另一个线程设置超时
thread::spawn(|| {
    handler.set_timeout(Duration::from_secs(5));
});

// 在主线程检查中断
match scheduler.check_interrupt() {
    Some(Interrupt::Timeout(d)) => println!("超时: {}ms", d.as_millis()),
    Some(Interrupt::Breakpoint(id)) => println!("断点: {}", id),
    Some(Interrupt::StackOverflow) => println!("栈溢出"),
    Some(Interrupt::MemoryViolation { addr, access }) => {
        println!("内存违规: {:#x} ({})", addr, access)
    }
    None => println!("无中断"),
}
```

## 相关文件

- [src/runtime/interrupt.rs](../../../../src/runtime/interrupt.rs) - 中断类型定义
- [src/runtime/scheduler/mod.rs](../../../../src/runtime/scheduler/mod.rs) - 调度器中断集成
- [src/vm/errors.rs](../../../../src/vm/errors.rs) - VM 错误类型
- [tests/integration/interrupt/](../../../../../tests/integration/interrupt/) - 中断测试

## 测试覆盖

- `timeout_test.rs` - 超时中断测试
- `breakpoint_test.rs` - 断点中断测试
- `stack_overflow_test.rs` - 栈溢出中断测试
- `memory_violation_test.rs` - 内存违规中断测试

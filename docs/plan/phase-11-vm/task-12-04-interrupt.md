# Task 12.4: 中断处理

> **优先级**: P1
> **状态**: ⏳ 待实现

## 功能描述

处理虚拟机中断，包括、超时和调试断点。

## 中断类型

```rust
/// 中断类型
enum Interrupt {
    /// 超时中断
    Timeout(Duration),
    /// 调试断点
    Breakpoint(BreakpointId),
    /// 栈溢出
    StackOverflow,
    /// 内存访问违规
    MemoryAccessViolation {
        address: usize,
        access_type: AccessType,
    },
}

enum GcReason {
    AllocationFailed,
    ThresholdReached,
    Explicit,
}

enum AccessType {
    Read,
    Write,
    Execute,
}
```

## 中断处理

```rust
impl VM {
    /// 检查和处理中断
    fn check_interrupt(&mut self) -> VMResult<()> {
        if let Some(interrupt) = self.interrupt_handler.peek() {
            match interrupt {
                Interrupt::GcRequest(reason) => {
                    self.runtime.gc().collect(reason);
                    self.interrupt_handler.clear();
                }
                Interrupt::Timeout(duration) => {
                    return Err(VMError::Timeout(duration));
                }
                Interrupt::Breakpoint(id) => {
                    self.handle_breakpoint(id)?;
                }
                Interrupt::StackOverflow => {
                    return Err(VMError::StackOverflow);
                }
                Interrupt::MemoryAccessViolation(addr, access) => {
                    return Err(VMError::MemoryViolation { addr, access });
                }
            }
        }
        Ok(())
    }
}
```

## 相关文件

- `src/vm/interrupt.rs`

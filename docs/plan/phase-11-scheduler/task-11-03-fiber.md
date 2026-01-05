# Task 11.3: Fiber 调度

> **优先级**: P1
> **状态**: ⏳ 待实现

## 功能描述

实现轻量级协程（Fiber）的调度。

## Fiber 结构

```rust
/// Fiber（轻量级协程）
struct Fiber {
    /// Fiber 栈
    stack: FiberStack,
    /// Fiber 上下文
    context: FiberContext,
    /// Fiber 状态
    state: FiberState,
    /// 入口函数
    entry: *mut dyn FnOnce(),
    /// 调度器引用
    scheduler: *const WorkStealingScheduler,
}

enum FiberState {
    /// 准备执行
    Ready,
    /// 正在执行
    Running,
    /// 已暂停（yield）
    Suspended,
    /// 已完成
    Completed,
    /// 出错
    Error,
}

impl Fiber {
    /// 创建 Fiber
    pub fn new(entry: impl FnOnce() + Send + 'static, stack_size: usize) -> Result<Self, Error> { ... }

    /// 让出执行
    pub fn yield_now(&mut self) {
        self.state = FiberState::Suspended;
        unsafe {
            FiberContext::switch(&mut self.context, &self.scheduler.context);
        }
    }

    /// 恢复执行
    pub fn resume(&mut self) {
        self.state = FiberState::Running;
        unsafe {
            FiberContext::switch(&self.scheduler.context, &mut self.context);
        }
    }
}
```

## 相关文件

- **fiber.rs**: Fiber 实现

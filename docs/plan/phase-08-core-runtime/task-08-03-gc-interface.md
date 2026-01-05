# Task 8.3: GC 接口

> **优先级**: P0
> **状态**: ⚠️ 部分实现

## 功能描述

定义 GC 接口，支持多种垃圾回收策略。

## GC Trait

```rust
/// GC  Trait
trait GarbageCollector {
    /// 分配新对象
    fn alloc(&self, size: usize, type_info: &TypeInfo) -> Handle<dyn AnyObject>;

    /// 增加引用计数
    fn retain(&self, handle: Handle<dyn AnyObject>);

    /// 减少引用计数
    fn release(&self, handle: Handle<dyn AnyObject>);

    /// 手动触发 GC
    fn collect(&self, reason: GcReason) -> usize;

    /// 获取当前内存使用
    fn memory_usage(&self) -> usize;

    /// 注册根引用
    fn register_root(&self, handle: &Handle<dyn AnyObject>);

    /// 注销根引用
    fn unregister_root(&self, handle: &Handle<dyn AnyObject>);
}
```

## GC 策略

```rust
/// 垃圾回收策略
enum GcStrategy {
    /// 引用计数（轻量级）
    RefCount,

    /// 标记-清除
    MarkSweep,

    /// 标记-压缩
    MarkCompact,

    /// 分代 GC
    Generational {
        young_generation_size: usize,
        old_generation_threshold: usize,
    },

    /// 三色标记法
    TriColorMarkSweep,
}
```

## GC 触发条件

```rust
enum GcReason {
    AllocationFailure,    // 分配失败
    ThresholdReached,     // 达到阈值
    ExplicitRequest,      // 显式调用
    LowMemory,            // 内存不足
    ThreadDetach,         // 线程分离
}
```

## 相关文件

- `src/runtime/core/gc.rs`
- `src/runtime/memory/gc.rs`

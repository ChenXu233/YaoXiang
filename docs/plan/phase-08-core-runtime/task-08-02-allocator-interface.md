# Task 8.2: 内存分配器接口

> **优先级**: P0
> **状态**: ✅ 已完成
> **模块**: `src/runtime/memory/allocator.rs`
> **依赖**: task-08-01-value-type
> **参考**: RFC-009 所有权模型

## 功能描述

定义简单的内存分配器接口，与 RFC-009 所有权模型保持一致。

### 核心职责

1. **内存分配**：`alloc` / `alloc_zeroed`
2. **内存释放**：`dealloc`
3. **Send + Sync**：线程安全

### 设计原则（RFC-009）

> **核心洞察**：内存分配与所有权语义分离
> - **分配器**：只负责 raw 内存的分配/释放
> - **所有权**：由 `RuntimeValue` 处理（Move / ref=Arc / clone）
> - **RAII**：Rust Drop trait 自动处理

## 核心组件

### Allocator Trait

```rust
/// 内存分配器 Trait
///
/// # RFC-009 对照
/// - 分配器只管 raw 内存，不管所有权
/// - `ref` 关键字由 RuntimeValue::Arc 处理（Arc 自己管理引用计数）
/// - Move 是零拷贝（指针移动），不需要分配器参与
pub trait Allocator: Send + Sync {
    /// 分配内存
    fn alloc(&mut self, layout: MemoryLayout) -> Result<NonNull<u8>, AllocError>;

    /// 分配并清零
    fn alloc_zeroed(&mut self, layout: MemoryLayout) -> Result<NonNull<u8>, AllocError>;

    /// 释放内存
    fn dealloc(&mut self, ptr: NonNull<u8>, layout: MemoryLayout);
}

/// 分配错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllocError {
    OutOfMemory,
    AlignmentError,
}
```

### MemoryLayout

```rust
/// 内存布局
///
/// 包装 std::alloc::Layout，提供便捷构造函数
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryLayout {
    size: usize,
    align: usize,
}

impl MemoryLayout {
    pub fn from_size_align(size: usize, align: usize) -> Option<Self>;
    pub fn new::<T>() -> Self;
    pub fn size(&self) -> usize;
    pub fn align(&self) -> usize;
}
```

### BumpAllocator

```rust
/// Bump 分配器（高吞吐量）
///
/// - O(1) 分配
/// - 零碎片（单线程）
/// - 不支持单独 dealloc（整个 allocator 一起释放）
///
/// # Use Cases
/// - 短期大量分配
/// - 测试/fuzzing
pub struct BumpAllocator {
    next: usize,
    capacity: usize,
    buffer: Vec<u8>,
}

impl BumpAllocator {
    pub fn new() -> Self;
    pub fn with_capacity(capacity: usize) -> Self;
    pub fn remaining(&self) -> usize;
    pub fn reset(&mut self);
}
```

## 与现有代码集成

### Heap 实现 Allocator

```rust
// src/runtime/memory/mod.rs

impl Allocator for Heap {
    fn alloc(&mut self, layout: MemoryLayout) -> Result<NonNull<u8>, AllocError> {
        // 使用现有的 region-based 分配
        // ...
    }

    fn alloc_zeroed(&mut self, layout: MemoryLayout) -> Result<NonNull<u8>, AllocError> {
        // ...
    }

    fn dealloc(&mut self, ptr: NonNull<u8>, layout: MemoryLayout) {
        // Region-based: 不支持单独 dealloc
        // 内存随 region 一起释放
    }
}
```

### RFC-009 对照表

| RFC-009 设计 | 实现位置 | 说明 |
|-------------|---------|------|
| Move（零拷贝） | N/A | 指针移动，零分配 |
| `ref` = Arc | `RuntimeValue::Arc(Arc::new(...))` | Arc 自己管理引用计数 |
| clone() 深拷贝 | `RuntimeValue::explicit_clone_with_heap()` | 使用 Heap 分配新内存 |
| RAII 自动释放 | Rust Drop trait | 不需要额外代码 |
| 无 GC | - | Region-based 分配器 |

## 模块结构

```
src/runtime/memory/
├── mod.rs              # Heap, Region, Allocator trait 实现
├── allocator.rs        # Allocator trait, MemoryLayout, BumpAllocator
└── tests/
    ├── mod.rs
    └── allocator.rs    # 集成测试
```

## 验收测试

```rust
#[test]
fn test_allocator_trait() {
    let mut allocator = BumpAllocator::new();

    // 分配
    let layout = MemoryLayout::new::<i64>();
    let ptr = allocator.alloc(layout).unwrap();
    assert!(!ptr.as_ptr().is_null());

    // 写入值
    unsafe {
        ptr.as_ptr().cast::<i64>().write(42);
        assert_eq!(ptr.as_ptr().cast::<i64>().read(), 42);
    }
}

#[test]
fn test_heap_allocator() {
    let mut heap = Heap::new();

    let layout = MemoryLayout::from_size_align(64, 8).unwrap();
    let ptr = heap.alloc(layout).unwrap();

    unsafe {
        ptr.as_ptr().write(123i32);
        assert_eq!(ptr.as_ptr().read(), 123i32);
    }
}

#[test]
fn test_allocator_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Heap>();
    assert_send_sync::<BumpAllocator>();
}
```

## 测试结果

```
running 19 tests
test runtime::memory::allocator::tests::... - 9 tests passed
test runtime::memory::tests::... - 10 tests passed
test result: ok. 19 passed, 0 failed
```

## 后续任务

- task-08-03-gc-integration: GC 集成（可选）
- task-08-04-object-model: 对象模型

# Task 8.2: 内存分配器

> **优先级**: P0
> **状态**: ⚠️ 部分实现

## 功能描述

提供多种内存分配策略，包括 arena 分配、对象池和栈分配。

## 分配器类型

```rust
/// 分配器trait
trait Allocator {
    fn alloc(&self, size: usize, align: usize) -> Result<*mut u8, AllocError>;
    fn dealloc(&self, ptr: *mut u8, size: usize);
    fn realloc(&self, ptr: *mut u8, old_size: usize, new_size: usize) -> Result<*mut u8, AllocError>;
}

/// Arena 分配器（适合短期存活对象）
struct ArenaAllocator {
    start: *mut u8,
    current: *mut u8,
    end: *mut u8,
    name: String,
}

/// Bump 分配器（快速分配，无释放）
struct BumpAllocator {
    arena: ArenaAllocator,
    allocs: Vec<*mut u8>,  // 记录分配用于批量释放
}

/// 对象池（适合频繁分配/释放的对象）
struct ObjectPool<T: Sized> {
    free_list: Vec<*mut T>,
    pool: *mut T,
    capacity: usize,
}
```

## 分配策略选择

| 场景 | 推荐分配器 |
|------|-----------|
| 短期存活对象 | Arena/Bump |
| 长期存活对象 | GC Heap |
| 频繁小对象 | Object Pool |
| 栈上分配 | Stack Allocator |
| 跨线程对象 | GC Heap |

## 相关文件

- `src/runtime/core/allocator.rs`
- `src/runtime/memory/mod.rs` (当前实现)

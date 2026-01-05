# Task 10.2: GC 实现

> **优先级**: P0
> **状态**: ⚠️ 部分实现

## 功能描述

实现垃圾回收器，包括标记-清除和分代 GC。

## 标记-清除 GC

```rust
struct MarkSweepGC {
    /// 堆区域
    heap: MemoryRegion,
    /// 自由链表
    free_list: Vec<FreeBlock>,
    /// 根集合
    roots: Vec<*mut HeapObject>,
    /// GC 阈值
    threshold: usize,
}

impl GarbageCollector for MarkSweepGC {
    fn collect(&self, _reason: GcReason) -> usize {
        // 1. 标记阶段：遍历根集合，标记可达对象
        self.mark_phase();

        // 2. 清除阶段：回收未标记的对象
        let freed = self.sweep_phase();

        // 3. 压缩（可选）
        self.compact_phase();

        freed
    }
}
```

## 三色标记法

```rust
/// 三色状态
enum TriColor {
    White,   // 尚未发现
    Gray,    // 已发现但未遍历
    Black,   // 已遍历
}

/// 三色标记 GC
struct TriColorGC {
    state: HashMap<*mut HeapObject, TriColor>,
    work_list: Vec<*mut HeapObject>,
}
```

## 相关文件

- **gc.rs**: GC 实现

# Task 10.3: 内存池

> **优先级**: P1
> **状态**: ⏳ 待实现

## 功能描述

实现对象池，复用已分配的对象。

## 内存池实现

```rust
/// 内存池
struct MemoryPool {
    /// 池名称
    name: String,
    /// 对象大小
    object_size: usize,
    /// 对齐要求
    align: usize,
    /// 空闲对象链表
    free_list: Vec<*mut u8>,
    /// 池内存
    pool: *mut u8,
    /// 池容量
    capacity: usize,
    /// 已分配计数
    allocated: usize,
}

impl MemoryPool {
    /// 从池中分配
    pub fn alloc(&mut self) -> Result<*mut u8, AllocError> {
        if let Some(ptr) = self.free_list.pop() {
            self.allocated += 1;
            Ok(ptr)
        } else {
            Err(AllocError::PoolExhausted)
        }
    }

    /// 释放回池
    pub fn free(&mut self, ptr: *mut u8) {
        self.free_list.push(ptr);
        self.allocated -= 1;
    }
}
```

## 相关文件

- **pool.rs**: MemoryPool

# Task 10.1: 区域分配

> **优先级**: P0
> **状态**: ⚠️ 部分实现

## 功能描述

实现区域（Region）分配策略，适合批量分配和整体释放。

## 区域分配器

```rust
/// 内存区域
struct MemoryRegion {
    /// 区域起始地址
    start: *mut u8,
    /// 当前指针
    current: *mut u8,
    /// 区域结束地址
    end: *mut u8,
    /// 区域名称（用于调试）
    name: String,
}

impl MemoryRegion {
    /// 从区域分配
    pub fn alloc(&mut self, size: usize, align: usize) -> Result<*mut u8, AllocError> {
        // 对齐调整
        let offset = self.current.align_offset(align);
        if self.current.add(offset + size) > self.end {
            return Err(AllocError::OutOfMemory);
        }

        let ptr = self.current.add(offset);
        self.current = ptr.add(size);

        // 初始化为零
        ptr.write_bytes(0, size);

        Ok(ptr)
    }

    /// 释放整个区域
    pub fn reset(&mut self) {
        self.current = self.start;
    }

    /// 获取已用大小
    pub fn used(&self) -> usize {
        self.current as usize - self.start as usize
    }
}
```

## 相关文件

- **region.rs**: MemoryRegion

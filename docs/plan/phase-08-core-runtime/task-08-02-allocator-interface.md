# Task 8.2: 内存分配器接口

> **优先级**: P0
> **状态**: ⬜ 待开始
> **模块**: `src/core/allocator.rs`
> **依赖**: task-08-01-value-type

## 功能描述

定义内存分配器的统一接口，支持 YaoXiang 的所有权模型。

### 核心职责

1. **内存分配**：`alloc` / `alloc_zeroed`
2. **内存释放**：`dealloc`（RAII，自动释放）
3. **重新分配**：`realloc`
4. **引用计数管理**：支持 Arc 的原子引用计数
5. **泄漏检测**：开发模式下的内存追踪

### 设计原则

> **核心洞察**：YaoXiang 使用**所有权模型**管理内存，不是 GC。
> - **Move 语义**：赋值/传参时转移所有权，零拷贝
> - **ref 关键字**：使用 Arc 实现引用计数
> - **clone()**：显式深拷贝
> - **RAII**：值离开作用域时自动释放

## Allocator Trait

```rust
/// 内存分配器 Trait（核心接口）
///
/// # 设计说明
/// - 简单的三方法接口：alloc / dealloc / realloc
/// - 不包含所有权逻辑，只负责内存分配
/// - 所有权由 RuntimeValue 和所有权模型保证
pub trait Allocator: Send + Sync {
    /// 分配内存
    fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocError>;

    /// 分配并清零
    fn alloc_zeroed(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocError>;

    /// 释放内存
    fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout);

    /// 重新分配
    fn realloc(
        &mut self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<u8>, AllocError>;
}

/// 分配错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllocError {
    /// 内存不足
    OutOfMemory,
    /// 对齐错误
    AlignmentError,
    /// 无效指针
    InvalidPointer,
    /// 溢出错误
    Overflow,
}

/// 内存布局
#[derive(Debug, Clone, Copy)]
pub struct Layout {
    size: usize,
    align: usize,
}

impl Layout {
    /// 从大小和对齐创建布局
    pub fn from_size_align(size: usize, align: usize) -> Option<Self> {
        if align == 0 || align.is_power_of_two() {
            // 检查大小是否对齐
            if size % align == 0 {
                Some(Layout { size, align })
            } else {
                // 调整大小以满足对齐要求
                let aligned_size = (size + align - 1) & !(align - 1);
                Some(Layout { size: aligned_size, align })
            }
        } else {
            None
        }
    }

    /// 新建布局
    pub fn new<T>() -> Self {
        Layout {
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
        }
    }

    pub fn size(&self) -> usize { self.size }
    pub fn align(&self) -> usize { self.align }
}
```

## 引用计数分配器（用于 Arc）

```rust
/// 引用计数分配器 Trait（用于 Arc）
///
/// # 说明
/// - Arc 使用原子引用计数，需要专门的接口
/// - inc_ref / dec_ref 是原子操作
/// - 当 ref_count 归零时自动释放
pub trait RefCountedAllocator: Allocator {
    /// 增加引用计数，返回新计数
    fn inc_ref(&self, ptr: NonNull<u8>) -> usize;

    /// 减少引用计数，返回是否应该释放
    fn dec_ref(&self, ptr: NonNull<u8>) -> bool;

    /// 获取当前引用计数
    fn ref_count(&self, ptr: NonNull<u8>) -> usize;

    /// 分配并初始化 Arc
    fn alloc_arc<T>(&mut self, value: T) -> Arc<T>
    where
        T: 'static + Send + Sync,
    {
        let layout = Layout::new::<ArcInner<T>>();
        let ptr = self.alloc(layout).unwrap();
        let inner = ArcInner {
            value: value.into(),
            ref_count: AtomicUsize::new(1),
        };
        unsafe {
            ptr.as_ptr().write(inner);
        }
        unsafe { Arc::from_raw(ptr.as_ptr() as *const ArcInner<T>) }
    }
}

/// Arc 内部结构
struct ArcInner<T> {
    value: T,
    ref_count: AtomicUsize,
}
```

## 内存块追踪（用于泄漏检测）

```rust
/// 内存块（用于追踪分配的内存）
struct MemoryBlock {
    /// 内存指针
    ptr: NonNull<u8>,
    /// 布局信息
    layout: Layout,
    /// 分配位置（文件名:行号，用于调试）
    location: &'static str,
    /// 分配时间戳
    timestamp: u64,
}

/// 泄漏检测器（开发模式）
struct LeakDetector {
    /// 分配的内存块
    blocks: HashMap<NonNull<u8>, MemoryBlock>,
    /// 活跃所有者
    owners: HashMap<OwnerId, OwnerInfo>,
    /// 是否启用
    enabled: bool,
}

struct OwnerInfo {
    name: String,
    active_blocks: usize,
}

impl LeakDetector {
    pub fn new(enabled: bool) -> Self {
        LeakDetector {
            blocks: HashMap::new(),
            owners: HashMap::new(),
            enabled,
        }
    }

    /// 记录分配
    fn record_alloc(&mut self, block: MemoryBlock) {
        if !self.enabled { return; }
        self.blocks.insert(block.ptr, block);
    }

    /// 记录释放
    fn record_dealloc(&mut self, ptr: NonNull<u8>) {
        if !self.enabled { return; }
        self.blocks.remove(&ptr);
    }

    /// 检测泄漏
    fn detect_leaks(&self) -> Vec<LeakReport> {
        if !self.enabled { return vec![]; }
        self.blocks.values().map(|block| LeakReport {
            location: block.location,
            size: block.layout.size(),
            timestamp: block.timestamp,
        }).collect()
    }
}

/// 泄漏报告
struct LeakReport {
    location: &'static str,
    size: usize,
    timestamp: u64,
}
```

## 所有权感知的分配器包装

```rust
/// 所有权感知分配器（集成所有权模型）
///
/// # 说明
/// - 包装基础分配器
/// - 自动处理 Move / ref / clone 语义
/// - 值离开作用域时自动释放（RAII）
struct OwnershipAwareAllocator<A: Allocator> {
    inner: A,
    /// Arc 引用计数表
    arc_refs: HashMap<NonNull<u8>, AtomicUsize>,
}

impl<A: Allocator> OwnershipAwareAllocator<A> {
    /// 分配并获取所有权
    fn alloc_owned(&mut self, value: RuntimeValue) -> OwnedValue {
        let layout = value.layout();
        let ptr = self.inner.alloc(layout).unwrap();

        unsafe {
            // 将值写入内存
            std::ptr::write(ptr.as_ptr() as *mut RuntimeValue, value);
        }

        OwnedValue {
            ptr,
            layout,
            allocator: &mut self.inner as *mut A,
        }
    }

    /// 分配 Arc（ref 关键字）
    fn alloc_arc(&mut self, value: RuntimeValue) -> RuntimeValue {
        let ptr = self.alloc_owned(value);
        let ptr_addr = ptr.ptr;

        // 初始化引用计数为 1
        self.arc_refs.insert(ptr_addr, AtomicUsize::new(1));

        RuntimeValue::Arc(unsafe {
            Arc::from_raw(ptr.ptr.as_ptr() as *const RuntimeValue)
        })
    }

    /// Arc 引用计数增加
    fn inc_arc_ref(&self, ptr: NonNull<u8>) {
        if let Some(counter) = self.arc_refs.get(&ptr) {
            counter.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Arc 引用计数减少，返回是否应该释放
    fn dec_arc_ref(&mut self, ptr: NonNull<u8>) -> bool {
        if let Some(counter) = self.arc_refs.get(&ptr) {
            let prev = counter.fetch_sub(1, Ordering::SeqCst);
            if prev == 1 {
                // 引用计数归零，释放内存
                self.arc_refs.remove(&ptr);
                return true;
            }
        }
        false
    }
}

/// 拥有所有权的值（RAII）
struct OwnedValue {
    ptr: NonNull<u8>,
    layout: Layout,
    allocator: *mut dyn Allocator,
}

impl Drop for OwnedValue {
    fn drop(&mut self) {
        unsafe {
            // 析构值
            std::ptr::drop_in_place(self.ptr.as_ptr() as *mut RuntimeValue);
            // 释放内存
            (*self.allocator).dealloc(self.ptr, self.layout);
        }
    }
}
```

## 验收测试

```rust
#[test]
fn test_allocator_trait() {
    let mut allocator = BumpAllocator::new(ArenaConfig {
        size: 64 * 1024,
        align: 16,
        max_alloc_size: 1024,
    });

    // 分配
    let layout = Layout::new::<i64>();
    let ptr = allocator.alloc(layout).unwrap();
    assert!(!ptr.as_ptr().is_null());

    // 写入值
    unsafe {
        ptr.as_ptr().write(42);
        assert_eq!(ptr.as_ptr().read(), 42);
    }

    // 释放
    allocator.dealloc(ptr, layout);
}

#[test]
fn test_arc_ref_counting() {
    let mut allocator = BumpAllocator::new(ArenaConfig::default());

    // 分配 Arc
    let arc1 = allocator.alloc_arc(RuntimeValue::Int(42));

    // 增加引用
    let arc2 = arc1.clone();
    assert_eq!(arc1.ref_count(), 2);

    // 减少引用
    drop(arc2);
    assert_eq!(arc1.ref_count(), 1);

    // 最后释放
    drop(arc1);
    // 验证内存已释放
}

#[test]
fn test_leak_detection() {
    let detector = LeakDetector::new(true);

    // 模拟泄漏
    detector.record_alloc(MemoryBlock {
        ptr: NonNull::dangling(),
        layout: Layout::new::<i64>(),
        location: "test.rs:42",
        timestamp: 0,
    });

    let reports = detector.detect_leaks();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].size, 8);
}

#[test]
fn test_ownership_aware() {
    let mut allocator = OwnershipAwareAllocator {
        inner: BumpAllocator::new(ArenaConfig::default()),
        arc_refs: HashMap::new(),
    };

    // 分配 Owned 值
    let owned = allocator.alloc_owned(RuntimeValue::Int(42));
    assert_eq!(unsafe { ptr.as_ptr().read() }, 42);

    // 分配 Arc
    let arc = allocator.alloc_arc(RuntimeValue::Int(100));
    assert!(matches!(arc, RuntimeValue::Arc(_)));

    // Arc 超出作用域时自动释放
    drop(arc);
}
```

## 模块结构

```
src/core/
├── allocator.rs           # Allocator trait 定义
│   ├── Allocator          # 基础分配器接口
│   ├── RefCountedAllocator # 引用计数接口（Arc）
│   ├── Layout             # 内存布局
│   ├── LeakDetector       # 泄漏检测（开发模式）
│   └── OwnershipAwareAllocator # 所有权集成
└── tests/
    ├── mod.rs
    ├── trait_impl.rs      # Trait 实现测试
    └── leak_detection.rs  # 泄漏检测测试
```

## 与 RFC-009 对照

| RFC-009 设计 | 分配器实现 | 说明 |
|-------------|-----------|------|
| Move 语义 | `alloc_owned` | 零拷贝转移 |
| `ref` 关键字 | `alloc_arc` | Arc 引用计数 |
| `clone()` | 手动复制 | 用户调用 clone() |
| RAII | `OwnedValue::drop` | 自动释放 |
| 循环检测 | 编译期完成 | phase-05-ownership |

# Task 4.9: 逃逸分析集成 [已废弃]

> **优先级**: P2
> **状态**: ⚠️ 已废弃

## ⚠️ 废弃说明

**此任务已废弃** - YaoXiang 使用所有权模型，不需要逃逸分析。

### 废弃原因

YaoXiang 是一门使用**所有权模型**的语言，内存分配由程序员显式控制：

```yaoxiang
# 栈分配（默认）
x: Int = 42

# 堆分配（显式使用 Box）
heap_data: Box[List[Int]] = Box.new([1, 2, 3])

# 线程共享（显式使用 Arc）
shared: Arc[Data] = Arc.new(data)
```

编译器不需要推断"变量是否逃逸"来决定分配策略。

### 并发安全检查

如果需要检查并发安全性，请参考 **Phase 5: Send/Sync 约束检查**：
- **Send**: 类型可以安全地跨线程传输
- **Sync**: 类型可以安全地跨线程共享

---

## 历史实现（保留参考）

> ⚠️ 以下内容为历史实现参考，已标记为废弃，代码保留仅用于可能的教学参考。

### 代码位置

- **模块**: `src/middle/escape_analysis/mod.rs`
- **集成**: `src/middle/codegen/mod.rs` 中的 `escape_analysis` 字段

### 旧设计（已废弃）

```rust
// 逃逸分析决定栈分配 vs 堆分配
fn should_heap_allocate_for_var(
    &self,
    local_idx: usize,
    ty: &MonoType,
) -> bool {
    // 1. 首先检查逃逸分析结果
    if let Some(ref escape) = self.escape_analysis {
        let local_id = LocalId::new(local_idx);
        if escape.should_heap_allocate(local_id) {
            return true;
        }
    }

    // 2. 根据类型决定（回退策略）
    self.should_heap_allocate_for_type(ty)
}
```

### 结论

| 方面 | 旧设计（已废弃） | 新设计 |
|-----|-----------------|-------|
| 内存分配 | 编译器推断 | 程序员显式控制 |
| 堆分配触发 | 逃逸分析决定 | `Box[T]` / `Arc[T]` |
| 栈分配 | 默认尝试 | 默认行为 |
| 闭包捕获 | 逃逸分析决定 | 所有权检查 + Send/Sync |

---

# Task 5.3: ref 关键字（Arc 引用计数）

> **优先级**: P0
> **状态**: 🔄 待实现
> **模块**: `src/core/ownership/ref.rs`

## 功能描述

`ref` 关键字创建 Arc（原子引用计数），用于安全共享所有权：

- **`ref` = Arc**：原子引用计数，线程安全
- **自动 Send + Sync**：Arc 自动满足并发约束
- **跨 spawn 安全**：可安全捕获到闭包中

> **RFC-009 v7 核心设计**：`ref` 替代借用检查器，通过 Arc 实现安全共享。

## ref 规则

### ref 创建 Arc

```yaoxiang
# ref 创建 Arc（原子引用计数）
p: Point = Point(1.0, 2.0)
shared = ref p    # p 的引用计数 = 1

# 多个共享引用
shared2 = ref p   # p 的引用计数 = 2
shared3 = ref p   # p 的引用计数 = 3

# 当所有 Arc 释放时，值自动释放
# shared, shared2, shared3 释放后，p 自动释放
```

### 跨 spawn 边界安全

```yaoxiang
# ✅ ref 可安全跨 spawn 边界
p: Point = Point(1.0, 2.0)
shared = ref p    # Arc，线程安全

spawn(() => {
    print(shared.x)   # ✅ 安全访问
})
# spawn 自动检查 Send 约束

# ✅ 多个任务共享
task1 = spawn(() => print(shared.x))
task2 = spawn(() => print(shared.y))

# 两个任务都通过 Arc 安全访问同一值
```

### ref 与 Move 对比

```yaoxiang
# Move：值转移
data: List[Int] = [1, 2, 3]
new_owner = data    # data 不再可用

# ref：共享访问（Arc）
data: List[Int] = [1, 2, 3]
shared = ref data   # data 和 shared 都可用

# 原值仍可访问
print(data.length)  # ✅
print(shared.length) # ✅

# Arc 引用计数
# shared 释放时计数减少
# 计数归零时 data 自动释放
```

## 检查算法

```rust
struct RefAnalyzer {
    /// Arc 引用计数
    arc_counts: HashMap<ValueId, ArcCount>,
    /// Arc 关联的原值
    arc_target: HashMap<ValueId, ValueId>,
    /// 跨 spawn 的 Arc 引用
    spawned_arcs: Vec<SpawnedArc>,
    /// 错误
    errors: Vec<RefError>,
}

#[derive(Debug, Clone)]
struct ArcCount {
    /// 当前计数
    count: AtomicUsize,
    /// 位置（用于错误信息）
    locations: Vec<Location>,
}

impl RefAnalyzer {
    /// 分析 ref 表达式
    fn analyze_ref(&mut self, expr: &RefExpr) -> Result<(), RefError> {
        let target_id = self.get_value_id(&expr.target)?;
        let ref_id = self.get_value_id(&expr.result)?;

        // 获取或创建 Arc 计数
        let count = self.arc_counts
            .entry(target_id)
            .or_insert_with(|| ArcCount {
                count: AtomicUsize::new(1),
                locations: vec![expr.span],
            });

        // 增加引用计数
        count.count.fetch_add(1, Ordering::AcqRel);

        // 记录 Arc 关联
        self.arc_target.insert(ref_id, target_id);

        // 检查是否跨 spawn
        if self.is_in_spawn() {
            self.spawned_arcs.push(SpawnedArc {
                arc: ref_id,
                target: target_id,
                spawn_id: self.current_spawn_id(),
            });
        }

        Ok(())
    }

    /// 分析 Arc 释放
    fn analyze_arc_drop(&mut self, arc: &ArcDrop) -> Result<(), RefError> {
        let arc_id = self.get_value_id(&arc.arc)?;

        if let Some(target_id) = self.arc_target.get(&arc_id) {
            if let Some(count) = self.arc_counts.get_mut(target_id) {
                // 减少引用计数
                let prev = count.count.fetch_sub(1, Ordering::AcqRel);

                if prev == 1 {
                    // 计数归零，原值可以释放
                    // 延迟到作用域结束时释放
                }
            }
        }

        Ok(())
    }

    /// 检查跨 spawn 的 Arc 是否安全
    fn check_spawned_arcs(&self) -> Result<(), RefError> {
        for spawned in &self.spawned_arcs {
            // Arc 自动满足 Send + Sync
            // 只需检查目标值类型是否安全共享

            if let Some(count) = self.arc_counts.get(&spawned.target) {
                // 多个任务持有 Arc，确保线程安全
                // Arc 内部使用原子操作，是线程安全的
            }
        }

        Ok(())
    }
}
```

## 错误类型

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum RefError {
    /// ref 应用于非所有者
    RefNonOwner {
        ref_span: Span,
        target_span: Span,
    },
    /// Arc 循环引用（跨任务）
    ArcCycleAcrossTasks {
        arc: ValueId,
        cycle: Vec<ValueId>,
    },
    /// 引用计数溢出
    ArcCountOverflow {
        value: ValueId,
    },
}
```

## 与 RFC-009 v7 对照

| RFC-009 v7 设计 | 实现状态 |
|----------------|---------|
| ref 关键字创建 Arc | ✅ 待实现 |
| Arc 自动 Send + Sync | ✅ 待实现 |
| 跨 spawn 安全捕获 | ✅ 待实现 |
| 引用计数管理 | ✅ 待实现 |
| 跨任务循环检测 | ❌ 见 task-05-06 |

## 验收测试

```yaoxiang
# test_ref.yx

# === ref 创建 Arc ===
p: Point = Point(1.0, 2.0)
shared = ref p
assert(p.x == 1.0)     # ✅ 原值仍可用
assert(shared.x == 1.0) # ✅ Arc 可访问

# === 多个 ref ===
shared2 = ref p
shared3 = ref p
# 引用计数 = 3

# === 跨 spawn 安全 ===
p: Point = Point(1.0, 2.0)
shared = ref p

task1 = spawn(() => {
    print(shared.x)   # ✅ 安全
})

task2 = spawn(() => {
    print(shared.y)   # ✅ 安全
})

# === ref 计数归零释放 ===
p: Point = Point(1.0, 2.0)
shared = ref p
# shared 释放后，p 可被释放

print("ref (Arc) tests passed!")
```

## 相关文件

- **src/core/ownership/ref.rs**: ref 关键字分析
- **src/core/ownership/arc.rs**: Arc 引用计数实现
- **src/core/ownership/mod.rs**: 所有权检查器主模块

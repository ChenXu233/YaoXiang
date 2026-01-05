# Task 9.3: 惰性求值

> **优先级**: P1
> **状态**: ✅ 已实现

## 功能描述

实现基于 DAG 的惰性求值，只在需要时计算值。

## 惰性求值策略

```rust
/// 惰性值
struct Lazy<T> {
    /// DAG 节点
    node: NodeId,
    /// 缓存的值
    cached: Option<T>,
    /// 依赖的节点
    dependencies: Vec<NodeId>,
}

impl<T> Lazy<T> {
    /// 获取值（触发计算）
    pub fn get(&mut self) -> &T {
        if self.cached.is_none() {
            self.evaluate();
        }
        self.cached.as_ref().unwrap()
    }

    /// 触发求值
    fn evaluate(&mut self) {
        // 按依赖顺序计算
        for dep in &self.dependencies {
            Dag::get(dep);
        }
        // 计算当前节点
        self.cached = Some(Dag::compute(self.node));
    }
}
```

## 相关文件

- **lazy.rs**: 惰性求值

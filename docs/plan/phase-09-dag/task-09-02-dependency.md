# Task 9.2: 依赖分析

> **优先级**: P0
> **状态**: ✅ 已实现

## 功能描述

分析任务间的依赖关系，构建 DAG。

## 依赖类型

| 类型 | 说明 | 示例 |
|------|------|------|
| `Data` | 数据依赖 | `b = a + 1` |
| `Control` | 控制依赖 | `if cond { a } else { b }` |
| `Resource` | 资源依赖 | `mutex.lock()` |
| `Order` | 顺序依赖 | `a; b` |

## 依赖分析算法

```rust
impl DependencyAnalyzer {
    /// 分析表达式的依赖
    pub fn analyze_expr(&self, expr: &Expr) -> Dependencies { ... }

    /// 分析函数调用的依赖
    pub fn analyze_call(&self, call: &Call) -> Dependencies { ... }

    /// 构建 DAG
    pub fn build_dag(&self, module: &Module) -> Dag<Function> { ... }
}
```

## 相关文件

- **dependency.rs**: 依赖分析

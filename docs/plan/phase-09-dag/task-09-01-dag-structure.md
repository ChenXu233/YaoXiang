# Task 9.1: DAG 结构

> **优先级**: P0
> **状态**: ✅ 已实现

## 功能描述

实现 DAG 数据结构，用于表示任务依赖关系。

## DAG 结构

```rust
/// DAG 节点
struct DagNode<T> {
    /// 节点 ID
    id: NodeId,
    /// 节点数据
    data: T,
    /// 前驱节点
    predecessors: Vec<NodeId>,
    /// 后继节点
    successors: Vec<NodeId>,
    /// 节点状态
    state: NodeState,
    /// 依赖计数（入度）
    dependency_count: usize,
}

/// DAG 图
struct Dag<T> {
    nodes: HashMap<NodeId, DagNode<T>>,
    entry: NodeId,
    exit: NodeId,
}
```

## 节点状态

```rust
enum NodeState {
    /// 未就绪（有未完成的依赖）
    Pending,
    /// 已就绪，可以执行
    Ready,
    /// 正在执行
    Running,
    /// 已完成
    Completed,
    /// 出错
    Error(DagError),
}
```

## 相关文件

- **mod.rs**: Dag 结构

# Task 5.6: 跨任务循环引用检测

> **优先级**: P1
> **状态**: 🔄 待实现
> **模块**: `src/core/lifetime/cycle_check.rs`
> **依赖**: task-05-03（ref Arc），phase-09（DAG 分析）

## 功能描述

检测跨任务边是否形成循环引用：

- **任务内循环**：允许（泄漏可控，任务结束后释放）
- **跨任务循环**：编译器检测并报错

> **RFC-009 v7 核心规则**：跨 spawn 边的 ref 引用不能形成环。

## 循环检测规则

### 任务内循环（允许）

```yaoxiang
# ✅ 允许：任务内循环引用（泄漏可控）
type Node = Node(child: ?Node)

main: () -> Void = () => {
    a: Node = Node(None)
    b: Node = Node(None)

    # 任务内循环：允许
    a.child = ref b
    b.child = ref a

    # 任务结束后，Arc 计数归零，值释放
    # 泄漏是可控的
}
```

### 跨任务循环（检测）

```yaoxiang
# ❌ 错误：跨任务循环引用
type Node = Node(child: ?Node)

# 任务 A 创建节点
task_a: () -> Node = () => {
    a: Node = Node(None)
    a
}

# 任务 B 创建节点并引用 A
task_b: (Node) -> Void = (a_ref) => {
    b: Node = Node(None)
    b.child = ref a_ref
    # 如果 a_ref 又 ref b，就会形成循环
}

main: () -> Void = () => {
    a = spawn(task_a())
    b = spawn(task_b(ref a))

    # ❌ 编译错误：跨任务循环
    # a 持有 b 的引用，b 持有 a 的引用
}
```

### 检测原理

```
┌─────────────────────────────────────────────────────────────────────┐
│                        循环检测原理                                   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  任务树构建：                                                         │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  spawn ──▶ Task A ──▶ ref ──▶ Node A                        │   │
│  │       │                                                      │   │
│  │       └──▶ Task B ──▶ ref ──▶ Node B ──▶ ref ──▶ Node A     │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  边类型：                                                            │
│  - 任务创建边：spawn ──▶ Task                                       │
│  - 引用边：Task/Node ──▶ ref ──▶ Node                               │
│                                                                      │
│  检测算法：                                                          │
│  1. 构建任务图（所有 spawn 节点）                                    │
│  2. 追踪所有 ref 引用的源和目标                                      │
│  3. 检测跨任务边是否形成环                                           │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## 检查算法

```rust
struct CycleChecker {
    /// 任务图
    task_graph: TaskGraph,
    /// 引用关系图
    ref_graph: RefGraph,
    /// 跨任务引用边
    cross_task_edges: Vec<CrossTaskEdge>,
    /// 错误
    errors: Vec<CycleError>,
}

#[derive(Debug)]
struct TaskGraph {
    /// 任务节点
    tasks: HashMap<TaskId, TaskNode>,
    /// 任务创建关系
    spawn_edges: Vec<SpawnEdge>,
}

#[derive(Debug)]
struct RefGraph {
    /// 节点（任务或值）
    nodes: HashMap<NodeId, RefNode>,
    /// 引用边
    edges: Vec<RefEdge>,
}

#[derive(Debug, Clone)]
struct CrossTaskEdge {
    /// 源任务
    from_task: TaskId,
    /// 目标任务
    to_task: TaskId,
    /// 引用的值
    target_value: ValueId,
    /// 位置
    span: Span,
}

impl CycleChecker {
    /// 检查循环引用
    fn check_cycles(&mut self) -> Result<(), CycleError> {
        // 1. 构建任务图
        self.build_task_graph()?;

        // 2. 构建引用图
        self.build_ref_graph()?;

        // 3. 收集跨任务引用边
        self.collect_cross_task_edges()?;

        // 4. 检测跨任务循环
        self.detect_cross_task_cycles()
    }

    /// 构建任务图
    fn build_task_graph(&mut self) -> Result<(), CycleError> {
        for spawn in self.all_spawn_exprs() {
            let parent_task = self.current_task();
            let child_task = spawn.task_id();

            // 记录 spawn 边
            self.task_graph.spawn_edges.push(SpawnEdge {
                parent: parent_task,
                child: child_task,
                span: spawn.span,
            });

            // 添加任务节点
            self.task_graph.tasks.insert(child_task, TaskNode {
                id: child_task,
                created_by: parent_task,
                values: spawn.captured_values(),
            });
        }

        Ok(())
    }

    /// 构建引用图
    fn build_ref_graph(&mut self) -> Result<(), CycleError> {
        for ref_expr in self.all_ref_exprs() {
            let source = ref_expr.source();
            let target = ref_expr.target();
            let target_task = self.get_value_task(target);

            // 确定源节点类型
            let source_node = if source.is_task() {
                NodeId::Task(source.task_id())
            } else {
                NodeId::Value(source.value_id())
            };

            // 确定目标节点
            let target_node = NodeId::Value(target.value_id());

            // 记录引用边
            self.ref_graph.edges.push(RefEdge {
                from: source_node,
                to: target_node,
                span: ref_expr.span,
            });

            // 添加到节点映射
            self.ref_graph.nodes.insert(source_node, RefNode {
                id: source_node,
                refs_to: vec![target_node],
            });
        }

        Ok(())
    }

    /// 收集跨任务引用边
    fn collect_cross_task_edges(&mut self) -> Result<(), CycleError> {
        for edge in &self.ref_graph.edges {
            let from_task = self.get_edge_source_task(edge);
            let to_task = self.get_edge_target_task(edge);

            // 跨任务引用
            if from_task != to_task {
                self.cross_task_edges.push(CrossTaskEdge {
                    from_task,
                    to_task,
                    target_value: edge.to_value(),
                    span: edge.span,
                });
            }
        }

        Ok(())
    }

    /// 检测跨任务循环（Tarjan SCC 算法）
    fn detect_cross_task_cycles(&mut self) -> Result<(), CycleError> {
        // 使用 Tarjan 算法找强连通分量（SCC）
        let sccs = self.tarjan_scc(&self.cross_task_edges)?;

        // 检查每个 SCC 是否包含跨任务引用
        for scc in &sccs {
            if self.is_cross_task_cycle(scc) {
                return Err(CycleError::CrossTaskCycle {
                    tasks: scc.tasks.clone(),
                    edges: scc.edges.clone(),
                });
            }
        }

        Ok(())
    }

    /// 判断 SCC 是否构成跨任务循环
    fn is_cross_task_cycle(&self, scc: &SCC) -> bool {
        // 跨任务循环条件：
        // 1. SCC 包含多个任务
        // 2. 任务间有引用边形成环

        if scc.tasks.len() <= 1 {
            return false;
        }

        // 检查是否每个任务都可达其他任务
        let tasks: HashSet<TaskId> = scc.tasks.iter().cloned().collect();

        for edge in &scc.edges {
            let from = self.get_edge_source_task(edge);
            let to = self.get_edge_target_task(edge);

            // 确实是跨任务边
            if tasks.contains(&from) && tasks.contains(&to) {
                return true;
            }
        }

        false
    }

    /// Tarjan SCC 算法
    fn tarjan_scc(&self, edges: &[CrossTaskEdge]) -> Result<Vec<SCC>, CycleError> {
        let mut index = 0;
        let mut indices = HashMap::new();
        let mut lowlink = HashMap::new();
        let mut on_stack = HashSet::new();
        let mut stack = Vec::new();
        let mut sccs = Vec::new();

        let nodes: HashSet<TaskId> = edges
            .iter()
            .flat_map(|e| vec![e.from_task, e.to_task])
            .collect();

        fn strongconnect(
            node: TaskId,
            edges: &[CrossTaskEdge],
            index: &mut usize,
            indices: &mut HashMap<TaskId, usize>,
            lowlink: &mut HashMap<TaskId, usize>,
            on_stack: &mut HashSet<TaskId>,
            stack: &mut Vec<TaskId>,
            sccs: &mut Vec<SCC>,
        ) {
            *index += 1;
            indices.insert(node, *index);
            lowlink.insert(node, *index);
            stack.push(node);
            on_stack.insert(node);

            for edge in edges {
                if edge.from_task == node {
                    let successor = edge.to_task;
                    if !indices.contains_key(&successor) {
                        strongconnect(
                            successor, edges, index, indices, lowlink,
                            on_stack, stack, sccs,
                        );
                        lowlink.insert(node, min(*lowlink.get(&node).unwrap(), *lowlink.get(&successor).unwrap()));
                    } else if on_stack.contains(&successor) {
                        lowlink.insert(node, min(*lowlink.get(&node).unwrap(), *indices.get(&successor).unwrap()));
                    }
                }
            }

            if lowlink.get(&node) == indices.get(&node) {
                let mut scc_tasks = Vec::new();
                let mut scc_edges = Vec::new();
                loop {
                    let w = stack.pop().unwrap();
                    on_stack.remove(&w);
                    scc_tasks.push(w);
                    if w == node {
                        break;
                    }
                }
                sccs.push(SCC {
                    tasks: scc_tasks,
                    edges: scc_edges,
                });
            }
        }

        for node in nodes {
            if !indices.contains_key(&node) {
                strongconnect(
                    node, edges, &mut index, &mut indices, &mut lowlink,
                    &mut on_stack, &mut stack, &mut sccs,
                );
            }
        }

        Ok(sccs)
    }
}
```

## 错误类型

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum CycleError {
    /// 跨任务循环引用
    CrossTaskCycle {
        /// 循环中的任务
        tasks: Vec<TaskId>,
        /// 形成循环的边
        edges: Vec<CrossTaskEdge>,
    },
    /// 循环路径详细信息
    CyclePath {
        /// 起始任务
        start_task: TaskId,
        /// 循环路径
        path: Vec<TaskId>,
    },
}
```

## 错误信息示例

```
error[E001]: cross-task reference cycle detected

  ┌──────────────────────────────────────────────────────────────┐
  │  Cycle detected between tasks:                               │
  │                                                              │
  │    Task "task_a" (at main.yaoxiang:10)                       │
  │         │                                                    │
  │         │ ref (at main.yaoxiang:15)                          │
  │         ▼                                                    │
  │    Task "task_b" (at main.yaoxiang:12)                       │
  │         │                                                    │
  │         │ ref (at main.yaoxiang:18)                          │
  │         ▼                                                    │
  │    Task "task_a" ◀─── back to start                          │
  │                                                              │
  │  Solution: Use `unsafe` block if intentional,                │
  │  or restructure to break the cycle.                         │
  └──────────────────────────────────────────────────────────────┘
```

## 与 RFC-009 v7 对照

| RFC-009 v7 设计 | 实现状态 |
|----------------|---------|
| 任务内循环：允许 | ✅ 见本实现 |
| 跨任务循环：检测 | ✅ 待实现 |
| DAG 分析 | ✅ 见 phase-09 |
| unsafe 可绕过检测 | ✅ 见 phase-06 |

## 验收测试

```yaoxiang
# test_cycle_detection.yx

# === 任务内循环（允许）===
type Node = Node(next: ?Node)

task_with_cycle: () -> Void = () => {
    a: Node = Node(None)
    b: Node = Node(None)

    # ✅ 允许：任务内循环
    a.next = ref b
    b.next = ref a
}

# === 跨任务循环（检测）===
type Shared = Shared(data: Int)

task1: () -> Shared = () => {
    s = Shared(1)
    s
}

task2: (Shared) -> Void = (s) => {
    other = Shared(2)
    other.data = s.data  # 引用 task1 的结果
}

main: () -> Void = () => {
    t1 = spawn(task1())
    t2 = spawn(task2(ref t1))

    # ❌ 这里应该检测循环
    # t2 通过 ref t1 持有 t1 的引用
}

# === 复杂循环检测 ===
type Link = Link(ref: ?Link)

main: () -> Void = () => {
    a = spawn(() => {
        link = Link(None)
        link
    })

    b = spawn(() => {
        link = Link(None)
        link
    })

    # ❌ 循环：A 持有 B，B 持有 A
    a.ref = ref b
    b.ref = ref a
}

print("Cycle detection tests passed!")
```

## 相关文件

- **src/core/ownership/cycle_check.rs**: 循环检测器
- **src/core/ownership/ref.rs**: ref Arc 分析
- **src/middle/dag/mod.rs**: DAG 分析（phase-09）

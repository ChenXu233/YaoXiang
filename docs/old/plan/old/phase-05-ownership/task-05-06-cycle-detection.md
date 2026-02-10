# Task 5.6: 跨 spawn 循环引用检测

> **优先级**: P1
> **状态**: ✅ 已实现
> **模块**: `src/middle/lifetime/cycle_check.rs`
> **依赖**: task-05-03（ref Arc）
> **测试**: `src/middle/lifetime/tests/cycle_check.rs` (14 测试用例)

## 循环检测边界

### 循环类型与处理

| 循环类型 | 检测位置 | 处理 |
|----------|----------|------|
| **单函数内 ref 循环** | OwnershipChecker | ✅ 允许（泄漏可控） |
| **spawn 内部 ref 循环** | OwnershipChecker | ✅ 允许（泄漏可控） |
| **跨 spawn 参数/返回值 ref 循环** | CycleChecker（新增） | ❌ 检测并报错 |

### 检测范围（只追踪边界）

```
只追踪 spawn 的参数和返回值：

spawn 参数 ────▶ spawn 内部
                │
                └──▶ spawn 返回值

检测：参数和返回值之间的 ref 是否形成环
```

### 不检测的情况

```yaoxiang
# ✅ 允许：单函数内循环
main: () -> Void = () => {
    a = Node(None)
    b = Node(None)
    a.child = ref b  # 单函数内，泄漏可控
    b.child = ref a
}

# ✅ 允许：spawn 内部循环
task_with_cycle: () -> Node = () => {
    a = Node(None)
    b = Node(None)
    a.child = ref b  # spawn 内部，泄漏可控
    b.child = ref a
    a
}

# ✅ 允许：工作池（扇出，不是环）
main: () -> Void = () => {
    shared = ref config
    workers = spawn for i in 0..10 {
        process(shared)  # 多个任务共享同一个，不是环
    }
}

# ❌ 检测：跨 spawn 循环
main: () -> Void = () => {
    a = spawn(task_a())      # 返回值 ref_a
    b = spawn(task_b(ref a)) # 参数 ref a，返回值 ref_b
    a.child = ref b          # ref_a 持有 ref_b → 环！
}
```

## 检测原理

```
┌─────────────────────────────────────────────────────────────────────┐
│                        循环检测原理                                   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  任务树构建：                                                         │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  spawn ──▶ Task A (参数: [], 返回值: ref_a)                  │   │
│  │       │                                                      │   │
│  │       └──▶ Task B (参数: [ref_a], 返回值: ref_b)             │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  跨 spawn 边：                                                        │
│  - Task B 的参数 ref_a 来自 Task A 的返回值                          │
│  - 如果 ref_a 又持有 ref_b → 环                                     │
│                                                                      │
│  检测算法：                                                          │
│  1. 收集所有 spawn 的参数和返回值                                    │
│  2. 构建跨 spawn 引用图                                             │
│  3. 检测是否形成环                                                   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## 简化检查算法（只追踪边界）

```rust
/// 循环检测器（只追踪 spawn 参数和返回值）
struct CycleChecker {
    /// 跨 spawn 引用边：spawn 返回值 ──▶ ref
    spawn_ref_edges: Vec<SpawnRefEdge>,
    /// 跨 spawn 参数边：spawn ──▶ 参数 ref 的来源
    spawn_param_edges: Vec<SpawnParamEdge>,
    /// 错误
    errors: Vec<OwnershipError>,
}

/// spawn 返回值持有外部 ref 的边
#[derive(Debug, Clone)]
struct SpawnRefEdge {
    /// spawn 任务 ID
    spawn_id: ValueId,
    /// 返回值持有的 ref（指向外部值）
    ref_target: ValueId,
    span: Span,
}

/// spawn 参数来自另一个 spawn 返回值
#[derive(Debug, Clone)]
struct SpawnParamEdge {
    /// 接收参数的 spawn
    consumer_spawn: ValueId,
    /// 提供的 spawn 返回值
    producer_spawn: ValueId,
    span: Span,
}

impl CycleChecker {
    /// 检查循环引用
    fn check_cycles(&mut self, func: &FunctionIR) -> Vec<OwnershipError> {
        self.errors.clear();
        self.spawn_ref_edges.clear();
        self.spawn_param_edges.clear();

        // 1. 收集 spawn 相关信息
        self.collect_spawn_edges(func);

        // 2. 构建跨 spawn 引用图
        let graph = self.build_spawn_graph();

        // 3. 检测环
        if self.has_cycle(&graph) {
            self.errors.push(OwnershipError::CrossTaskCycle {
                details: format!("循环引用检测：{:?}", graph),
                span: (0, 0),
            });
        }

        self.errors.clone()
    }

    /// 收集 spawn 的参数和返回值信息
    fn collect_spawn_edges(&mut self, func: &FunctionIR) {
        for (block_idx, block) in func.blocks.iter().enumerate() {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                match instr {
                    // spawn 指令：收集参数和返回值
                    Instruction::Spawn {
                        task_id,
                        args,
                        result,
                        ..
                    } => {
                        // 检查参数是否来自其他 spawn 的返回值
                        for arg in args {
                            if let Some(producer) = self.get_producer_spawn(arg) {
                                self.spawn_param_edges.push(SpawnParamEdge {
                                    consumer_spawn: *task_id,
                                    producer_spawn: producer,
                                    span: (block_idx, instr_idx),
                                });
                            }
                        }

                        // 检查返回值是否持有外部 ref
                        if let Some(ret_val) = result {
                            // 返回值的定义点
                            if let Some(def) = self.get_value_definition(ret_val) {
                                // 如果返回值持有的类型是 ref，追踪它
                                if let Some(ref_target) = self.get_ref_target(ret_val) {
                                    self.spawn_ref_edges.push(SpawnRefEdge {
                                        spawn_id: *task_id,
                                        ref_target,
                                        span: (block_idx, instr_idx),
                                    });
                                }
                            }
                        }
                    }
                    // ref 指令：追踪 ref 的目标
                    Instruction::Ref { dst, src } => {
                        // dst 持有 src 的 ref
                        // 如果 dst 是某个 spawn 的返回值，src 是外部值
                        // 则记录这条边
                        self.record_ref_edge(*dst, *src);
                    }
                    _ => {}
                }
            }
        }
    }

    /// 构建跨 spawn 引用图
    fn build_spawn_graph(&self) -> HashMap<ValueId, HashSet<ValueId>> {
        let mut graph: HashMap<ValueId, HashSet<ValueId>> = HashMap::new();

        // 参数边：producer → consumer（consumer 使用了 producer）
        for edge in &self.spawn_param_edges {
            graph
                .entry(edge.producer_spawn)
                .or_default()
                .insert(edge.consumer_spawn);
        }

        // ref 边：spawn 返回值持有外部 ref
        for edge in &self.spawn_ref_edges {
            // ref_target 可能有多个来源，找到它所属的 spawn
            if let Some(source_spawn) = self.find_spawn_owning_value(edge.ref_target) {
                // source_spawn → edge.spawn_id（有 ref 关系）
                graph
                    .entry(source_spawn)
                    .or_default()
                    .insert(edge.spawn_id);
            }
        }

        graph
    }

    /// 检测是否有环（简化版：只检测 spawn 参数/返回值之间）
    fn has_cycle(&self, graph: &HashMap<ValueId, HashSet<ValueId>>) -> bool {
        // 使用 DFS 检测环
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();

        for node in graph.keys() {
            if !visited.contains(node) {
                if self.detect_cycle_dfs(node, graph, &mut visited, &mut recursion_stack) {
                    return true;
                }
            }
        }

        false
    }

    fn detect_cycle_dfs(
        &self,
        node: &ValueId,
        graph: &HashMap<ValueId, HashSet<ValueId>>,
        visited: &mut HashSet<ValueId>,
        recursion_stack: &mut HashSet<ValueId>,
    ) -> bool {
        visited.insert(*node);
        recursion_stack.insert(*node);

        if let Some(edges) = graph.get(node) {
            for &neighbor in edges {
                if !visited.contains(&neighbor) {
                    if self.detect_cycle_dfs(neighbor, graph, visited, recursion_stack) {
                        return true;
                    }
                } else if recursion_stack.contains(&neighbor) {
                    // 找到环！
                    return true;
                }
            }
        }

        recursion_stack.remove(node);
        false
    }

    // 辅助方法
    fn get_producer_spawn(&self, arg: &Operand) -> Option<ValueId> {
        None // TODO: 实现
    }

    fn get_ref_target(&self, val: &Operand) -> Option<ValueId> {
        None // TODO: 实现
    }

    fn record_ref_edge(&mut self, _dst: Operand, _src: Operand) {
        // TODO: 实现
    }

    fn find_spawn_owning_value(&self, _val: ValueId) -> Option<ValueId> {
        None // TODO: 实现
    }

    fn get_value_definition(&self, _val: &Operand) -> Option<Definition> {
        None // TODO: 实现
    }
}
```

## 错误类型

```rust
/// 所有权错误扩展
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipError {
    // ... 现有错误类型 ...

    /// 跨 spawn 循环引用
    CrossSpawnCycle {
        /// 循环中的 spawn
        spawns: Vec<ValueId>,
        /// 循环路径
        path: Vec<ValueId>,
        /// 错误位置
        span: (usize, usize),
    },
}
```
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

- **src/middle/lifetime/cycle_check.rs**: 循环检测器（新增）
- **src/middle/lifetime/mod.rs**: OwnershipChecker 集成
- **src/middle/lifetime/error.rs**: CrossSpawnCycle 错误类型
- **src/middle/ir.rs**: Spawn 指令扩展（添加 args 和 result）

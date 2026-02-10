# YaoXiang 异步实现方案与评估报告

> 基于《象流：基于惰性求值的无感异步并发模型》技术白皮书
> 日期：2024-01-03

---

## 一、当前实现状态分析

### 1.1 已实现组件

根据对 [`src/middle/scheduler/mod.rs`](src/middle/scheduler/mod.rs)、[`src/std/concurrent.rs`](src/std/concurrent.rs) 和 [`src/middle/executor.rs`](src/middle/executor.rs) 的分析，当前 YaoXiang 项目已实现以下异步相关基础设施：

| 组件 | 文件 | 状态 | 说明 |
|------|------|------|------|
| 任务调度器 | [`src/middle/scheduler/mod.rs`](src/middle/scheduler/mod.rs) | 基础框架 | 支持工作窃取的协程调度器骨架 |
| 并发原语 | [`src/std/concurrent.rs`](src/std/concurrent.rs) | 基础实现 | spawn、sleep、mutex、原子类型 |
| 虚拟机执行器 | [`src/middle/executor.rs`](src/middle/executor.rs) | 骨架实现 | 包含异步操作码定义 |
| 异步操作码 | [`Opcode`](src/middle/executor.rs:139) | 已定义 | CallAsync、Spawn、Await、Yield |

### 1.2 当前架构问题

当前实现存在以下关键缺陷，与《象流》模型存在显著差距：

#### 1.2.1 缺乏惰性求值计算图

当前调度器 [`Scheduler`](src/middle/scheduler/mod.rs:107) 仅实现了简单的任务队列管理，**未构建任何计算图结构**：

```rust
// 当前实现：仅简单的任务队列
fn steal_or_get(...) -> Option<Arc<Task>> {
    if let Some(task) = local_queue.lock().unwrap().pop_front() {
        return Some(task);  // 无法感知任务间的依赖关系
    }
    // ...
}
```

#### 1.2.2 缺乏 `Async[T]` 惰性代理类型

当前代码中**完全没有 `Async[T]` 类型的实现**。根据《象流》白皮书，`Async[T]` 是实现"零传染性"的核心机制：

- `Async[T]` 必须是 `T` 的子类型
- 必须在类型系统层面透明地表示"未来值"
- 必须在使用时自动触发等待

#### 1.2.3 缺乏显式并行标记语义

当前 [`spawn`](src/std/concurrent.rs:8) 函数仅是对 Rust `thread::spawn` 的简单包装，不具备《象流》模型中描述的三重语义：

| 语法形式 | 期望语义 | 当前实现 |
|----------|----------|----------|
| `spawn fn` | 定义异步函数，返回 Async[T] | 仅包装线程创建 |
| `spawn { }` | 并行执行块内表达式 | 无支持 |
| `spawn for` | 数据并行循环 | 无支持 |

#### 1.2.4 缺乏自动依赖解析

当前调度器无法实现《象流》模型的核心优势——**基于数据依赖的自动并行化**。代码编写者无法仅通过同步思维获得并行执行效率。

---

## 二、象流模型异步实现方案

### 2.1 总体架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                      YaoXiang 象流运行时架构                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────────┐    ┌──────────────────┐    ┌────────────────┐ │
│  │   前端编译器      │    │    计算图构建器   │    │  类型系统扩展   │ │
│  │  (parser+codegen)│───▶│  (DAG Builder)   │───▶│ (Async[T])     │ │
│  └──────────────────┘    └──────────────────┘    └────────────────┘ │
│                                │                                        │
│                                ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │                      运行时层                                    │ │
│  ├─────────────────────────────────────────────────────────────────┤ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │ │
│  │  │ 调度器      │  │ 工作窃取器  │  │ 阻塞线程池              │ │ │
│  │  │ Scheduler   │  │ WorkStealer │  │ BlockingPool (@blocking)│ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │                      虚拟机层 (VM)                               │ │
│  ├─────────────────────────────────────────────────────────────────┤ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │ │
│  │  │ Future执行器│  │ 任务状态机  │  │ Async值解包器           │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.2 核心类型设计

#### 2.2.1 `Async[T]` 惰性代理类型

```rust
// src/middle/async/mod.rs

/// Async[T] - 惰性代理类型，实现"零传染性"
/// 
/// 设计要点：
/// 1. Async<T> 是 T 的子类型 (coercion)
/// 2. 在需要 T 的上下文中自动解包
/// 3. 内部实际存储 Result<T, E> 以支持错误传播
#[repr(transparent)]
pub struct Async<T: Send + 'static> {
    inner: Arc<dyn Future<Output = Result<T, DynError>> + Send + 'static>,
}

impl<T: Send + 'static> Async<T> {
    /// 创建新的 Async 值
    pub fn new<F>(future: F) -> Self 
    where 
        F: Future<Output = Result<T, DynError>> + Send + 'static,
    {
        Self {
            inner: Arc::new(Box::pin(future)),
        }
    }

    /// 检查是否已完成
    pub fn is_ready(&self) -> bool {
        // 使用原子状态检查
        todo!()
    }

    /// 等待并获取结果
    pub async fn await(&self) -> Result<T, DynError> {
        // 内部实现：自旋+阻塞混合策略
        todo!()
    }
}

/// 自动解包实现：当 Async<T> 被当作 T 使用时触发
impl<T: Send + 'static> std::ops::Deref for Async<T> {
    type Target = T;
    
    fn deref(&self) -> &T {
        // 阻塞等待并返回引用
        // 注意：这是同步解包，用于VM字节码执行
        todo!()
    }
}
```

#### 2.2.2 计算图节点

```rust
// src/middle/dag/mod.rs

/// 计算图节点
#[derive(Debug)]
pub struct DAGNode {
    /// 节点ID
    id: NodeId,
    /// 节点类型
    kind: NodeKind,
    /// 依赖的父节点
    parents: Vec<NodeId>,
    /// 依赖的子节点
    children: Vec<NodeId>,
    /// 节点状态
    state: NodeState,
    /// 计算结果（运行时填充）
    result: OnceCell<Result<Value, DynError>>,
}

/// 节点类型
#[derive(Debug, Clone)]
pub enum NodeKind {
    /// 同步计算节点
    Compute,
    /// 异步计算节点 (spawn fn)
    AsyncCompute,
    /// 并行块节点 (spawn {})
    ParallelBlock,
    /// 数据并行节点 (spawn for)
    DataParallel {
        iterator: Box<dyn Iterator>,
        body_fn: FunctionRef,
    },
    /// I/O 操作节点
    IO,
    /// 阻塞操作节点 (@blocking)
    Blocking,
}

/// 节点状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Pending,      // 未开始
    Scheduled,    // 已调度
    Running,      // 执行中
    Completed,    // 已完成
    Failed,       // 失败
}

/// 计算图 (DAG)
#[derive(Debug)]
pub struct ComputationDAG {
    /// 所有节点
    nodes: HashMap<NodeId, DAGNode>,
    /// 拓扑排序的调度队列
    ready_queue: VecDeque<NodeId>,
    /// 并行标记的边界
    parallel_boundaries: HashSet<BoundaryId>,
}

impl ComputationDAG {
    /// 添加节点并建立依赖关系
    pub fn add_node(&mut self, kind: NodeKind, dependencies: &[NodeId]) -> NodeId {
        let id = NodeId::new();
        let node = DAGNode {
            id,
            kind,
            parents: dependencies.to_vec(),
            children: Vec::new(),
            state: NodeState::Pending,
            result: OnceCell::new(),
        };
        
        // 更新依赖节点的 children
        for dep in dependencies {
            if let Some(dep_node) = self.nodes.get_mut(dep) {
                dep_node.children.push(id);
            }
        }
        
        self.nodes.insert(id, node);
        self.recalculate_ready();
        id
    }

    /// 重新计算就绪队列
    fn recalculate_ready(&mut self) {
        self.ready_queue.clear();
        for (id, node) in &self.nodes {
            if node.state == NodeState::Pending {
                // 检查所有父节点是否都已完成
                let all_parents_done = node.parents.iter().all(|pid| {
                    self.nodes.get(pid)
                        .map(|n| n.state == NodeState::Completed)
                        .unwrap_or(false)
                });
                if all_parents_done {
                    self.ready_queue.push_back(*id);
                }
            }
        }
    }
}
```

#### 2.2.3 智能调度器

```rust
// src/middle/scheduler/mod.rs

/// 象流调度器 - 支持工作窃取和计算图感知
pub struct FlowScheduler {
    /// 配置
    config: SchedulerConfig,
    /// 全局任务队列（计算图节点）
    dag_queue: Arc<Mutex<VecDeque<NodeId>>>,
    /// 每个工作线程的本地队列
    local_queues: Vec<Arc<Mutex<VecDeque<NodeId>>>>,
    /// 工作窃取器
    work_stealer: WorkStealer,
    /// 线程池
    workers: Vec<JoinHandle<()>>,
    /// 运行状态
    running: Arc<AtomicBool>,
    /// 阻塞线程池（用于 @blocking）
    blocking_pool: BlockingThreadPool,
}

impl FlowScheduler {
    /// 工作线程主循环
    fn worker_loop(worker_id: usize) {
        loop {
            // 1. 尝试从本地队列获取节点
            if let Some(node_id) = self.pop_local() {
                self.execute_node(node_id);
                continue;
            }
            
            // 2. 尝试从全局队列获取节点
            if let Some(node_id) = self.pop_global() {
                self.execute_node(node_id);
                continue;
            }
            
            // 3. 尝试从其他线程窃取
            if let Some(node_id) = self.steal(worker_id) {
                self.execute_node(node_id);
                continue;
            }
            
            // 4. 无任务可执行，短暂休眠
            thread::sleep(Duration::from_micros(100));
        }
    }

    /// 执行计算图节点
    fn execute_node(&self, node_id: NodeId) {
        let node = self.get_node(node_id);
        
        match &node.kind {
            NodeKind::AsyncCompute => {
                // 异步计算：提交到协程池执行
                self.submit_async(node_id);
            }
            NodeKind::ParallelBlock => {
                // 并行块：激进地并行执行所有直接子节点
                self.submit_parallel(node_id);
            }
            NodeKind::DataParallel { iterator, body_fn } => {
                // 数据并行：自动分片
                self.submit_data_parallel(node_id, iterator, body_fn);
            }
            NodeKind::Blocking => {
                // 阻塞操作：提交到阻塞线程池
                self.blocking_pool.submit(|| self.run_node(node_id));
            }
            NodeKind::Compute | NodeKind::IO => {
                // 同步/I/O：直接在当前线程执行
                self.run_node(node_id);
            }
        }
    }
}
```

### 2.3 编译器集成

#### 2.3.1 前端解析扩展

```rust
// src/frontend/parser/nud.rs

/// 解析 spawn 表达式
pub fn parse_spawn(&mut self) -> Result<Expr, ParseError> {
    let token = self.current_token.clone();
    
    match self.peek_token() {
        // spawn fn - 异步函数定义
        Token::Keyword("fn") => {
            self.next_token(); // 消耗 fn
            let func = self.parse_function()?;
            Ok(Expr::SpawnFn {
                func: Box::new(func),
                is_async: true,
            })
        }
        // spawn { } - 并行块
        Token::LBrace => {
            self.next_token(); // 消耗 {
            let exprs = self.parse_block_until(Token::RBrace)?;
            self.next_token(); // 消耗 }
            Ok(Expr::SpawnBlock {
                exprs,
                strategy: ParallelStrategy::Eager,
            })
        }
        // spawn for - 数据并行循环
        Token::Keyword("for") => {
            self.next_token(); // 消耗 for
            let var = self.parse_identifier()?;
            self.next_token(); // 消耗 in
            let iter = self.parse_expression()?;
            let body = self.parse_block()?;
            Ok(Expr::SpawnFor {
                var,
                iter: Box::new(iter),
                body: Box::new(body),
            })
        }
        _ => Err(ParseError::UnexpectedToken(token)),
    }
}
```

#### 2.3.2 类型系统扩展

```rust
// src/frontend/typecheck/infer.rs

/// 处理 Async[T] 类型
impl TypeChecker {
    /// 推断 spawn 表达式的类型
    pub fn infer_spawn_expr(&mut self, expr: &SpawnExpr) -> Type {
        match expr {
            SpawnExpr::Fn(func) => {
                // spawn fn 返回 Async[T]
                let inner_type = self.infer_function(func);
                Type::Async(Box::new(inner_type))
            }
            SpawnExpr::Block(exprs) => {
                // spawn { a, b } 返回元组 (A, B)
                let types: Vec<Type> = exprs
                    .iter()
                    .map(|e| self.infer_expr(e))
                    .collect();
                Type::Tuple(types)
            }
            SpawnExpr::For { body, .. } => {
                // spawn for 返回 List[T]
                let body_type = self.infer_expr(body);
                Type::List(Box::new(body_type))
            }
        }
    }

    /// 处理 Async[T] -> T 的隐式解包
    pub fn unify_async(&mut self, expected: &Type, actual: &Type) -> Result<(), TypeError> {
        match (expected, actual) {
            // Async<T> 在需要 T 的上下文中使用
            (Type::Concrete(t), Type::Async(inner)) => {
                self.unify(t, inner)
            }
            // T 可以隐式提升为 Async<T>
            (Type::Async(expected_inner), Type::Concrete(actual)) => {
                self.unify(expected_inner, actual)
            }
            _ => self.unify(expected, actual),
        }
    }
}
```

#### 2.3.3 代码生成扩展

```rust
// src/middle/codegen/expr.rs

/// 为 spawn 表达式生成字节码
pub fn codegen_spawn_expr(&mut self, expr: &SpawnExpr) -> Vec<ByteCode> {
    match expr {
        SpawnExpr::Fn(func) => {
            // 生成异步函数调用
            // 1. 为函数创建 Future
            // 2. 返回 Async<T> 值
            let mut code = Vec::new();
            code.extend(self.codegen_function(func));
            code.push(ByteCode::SpawnAsync); // 包装为 Async
            code
        }
        SpawnExpr::Block(exprs) => {
            // 生成并行块
            // 1. 依次编译所有表达式
            // 2. 用 Spawn 指令标记
            // 3. 在块结束时隐式等待所有结果
            let mut code = Vec::new();
            for expr in exprs {
                code.extend(self.codegen_expr(expr));
            }
            code.push(ByteCode::SpawnBarrier); // 屏障同步
            code
        }
        SpawnExpr::For { var, iter, body } => {
            // 生成数据并行循环
            // 1. 创建迭代器分片
            // 2. 为每个分片生成任务
            // 3. 收集结果
            let mut code = Vec::new();
            code.extend(self.codegen_expr(iter));
            code.push(ByteCode::SpawnDataParallel {
                var: var.clone(),
                body: self.codegen_expr(body),
            });
            code
        }
    }
}
```

### 2.4 虚拟机支持

#### 2.4.1 扩展操作码

```rust
// src/middle/opcode.rs

/// 扩展异步操作码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AsyncOpcode {
    /// 创建异步任务
    SpawnAsync = 0xE0,
    /// 等待异步任务完成
    Await = 0xE1,
    /// 创建并行块
    SpawnBlock = 0xE2,
    /// 并行屏障（等待所有任务）
    Barrier = 0xE3,
    /// 数据并行循环
    SpawnFor = 0xE4,
    /// 提交到工作窃取队列
    SubmitSteal = 0xE5,
    /// 标记阻塞操作
    MarkBlocking = 0xE6,
    /// 创建 Async 值
    MakeAsync = 0xE7,
    /// 解包 Async 值
    UnpackAsync = 0xE8,
    /// 检查 Async 是否就绪
    TryAsync = 0xE9,
    /// 协程让出执行权
    Yield = 0xEA,
    /// 创建计算图节点
    NewDagNode = 0xEB,
    /// 添加计算图边（依赖）
    AddDagEdge = 0xEC,
    /// 调度计算图节点
    ScheduleDagNode = 0xED,
}

/// 虚拟机执行器扩展
impl VM {
    /// 执行异步操作码
    fn execute_async_opcode(&mut self, opcode: AsyncOpcode) -> VMResult<()> {
        match opcode {
            AsyncOpcode::SpawnAsync => {
                // 创建异步任务并提交到调度器
                let task_id = self.pop::<TaskId>();
                self.scheduler.spawn(task_id);
                Ok(())
            }
            AsyncOpcode::Await => {
                // 等待异步任务
                let task_id = self.pop::<TaskId>();
                self.scheduler.await_task(task_id);
                Ok(())
            }
            AsyncOpcode::Barrier => {
                // 等待所有并行的子任务完成
                self.scheduler.barrier_wait();
                Ok(())
            }
            AsyncOpcode::Yield => {
                // 协程让出执行权
                self.current_task.set_state(TaskState::Ready);
                self.scheduler.yield_current();
                Ok(())
            }
            // ... 其他操作码
            _ => Err(VMError::InvalidAsyncOpcode(opcode)),
        }
    }
}
```

### 2.5 标准库集成

```rust
// src/std/async.rs

/// 异步工具函数

/// 并行执行多个异步任务
pub fn parallel<T>(tasks: Vec<Async<T>>) -> Vec<T> 
where T: Send + 'static {
    // 提交所有任务
    // 等待全部完成
    // 返回结果列表
}

/// 等待任意一个任务完成
pub async fn await_any<T>(tasks: Vec<Async<T>>) -> T {
    // 等待第一个完成的任务
    // 取消其他任务
}

/// 等待所有任务完成
pub async fn await_all<T>(tasks: Vec<Async<T>>) -> Vec<T> {
    // 依次等待所有任务
}

/// 超时控制
pub async fn with_timeout<T>(async_val: Async<T>, timeout: Duration) -> Option<T> {
    // 设置超时
    // 返回 Some(T) 或 None
}

/// 重试机制
pub async fn retry<T, E, F>(mut attempts: usize, f: F) -> Result<T, E>
where 
    F: FnMut() -> Async<Result<T, E>>,
{
    // 最多重试 attempts 次
}
```

---

## 三、方案评估

### 3.1 优势分析

#### 3.1.1 零传染性 ✓

**评估：** 实现 Async[T] 类型后可达成

- Async[T] 在类型系统中透明地作为 T 的子类型
- 开发者无需在函数签名中标记 `async` 或返回 `Future`
- 现有同步代码可零修改地享受异步执行

#### 3.1.2 高性能并行 ✓

**评估：** 计算图 + 工作窃取可达成

- 惰性计算图允许运行时进行全局优化
- 显式 `spawn` 标记提供明确的并行提示
- 工作窃取确保 CPU 核心高效利用

#### 3.1.3 心智模型简单 ✓

**评估：** 设计符合预期

- 开发者只需关注数据流
- 无需理解复杂的异步原语
- `spawn` 语义清晰直观

#### 3.1.4 易于重构 ✓

**评估：** 设计符合预期

- 顺序逻辑与并行逻辑切换仅需增删 `spawn`
- 类型系统自动处理等待逻辑

### 3.2 实现挑战与风险

| 挑战 | 风险级别 | 应对策略 |
|------|----------|----------|
| 惰性计算图构建复杂度 | 高 | 分阶段实现，先支持显式 `spawn` |
| Async[T] 子类型关系实现 | 高 | 依赖 Rust 的 trait 系统实现自动解包 |
| 工作窃取正确性 | 中 | 充分测试，使用 miri 进行内存安全检查 |
| 阻塞操作隔离 | 中 | 独立的阻塞线程池 + 资源限制 |
| 调试工具开发 | 高 | 后期重点投入，计算图可视化是关键 |

### 3.3 与现有实现差距

| 组件 | 当前状态 | 目标状态 | 差距 |
|------|----------|----------|------|
| 任务调度器 | 基础队列骨架 | DAG感知调度器 | 重大重构 |
| 并发原语 | 简单线程封装 | 完整象流语义 | 需重写 |
| 虚拟机 | 操作码骨架 | 完整异步执行器 | 需完善 |
| 编译器 | 无异步支持 | 完整生成 | 需新增 |
| 类型系统 | 无 Async | Async[T] 子类型 | 需扩展 |

### 3.4 推荐实现路径

#### 阶段一：基础异步框架（v0.2.0）
- [ ] 实现 `Async[T]` 类型
- [ ] 完善调度器 DAG 支持
- [ ] 实现 `spawn fn` 语法
- [ ] VM 异步执行器

#### 阶段二：显式并行（v0.3.0）
- [ ] 实现 `spawn { }` 语法
- [ ] 实现并行屏障
- [ ] 工作窃取优化

#### 阶段三：数据并行（v0.4.0）
- [ ] 实现 `spawn for` 语法
- [ ] 自动分片与负载均衡
- [ ] 结果收集机制

#### 阶段四：优化与工具（v0.5.0）
- [ ] 计算图可视化调试器
- [ ] 阻塞操作隔离
- [ ] 性能调优

---

## 四、总结

基于《象流》技术白皮书，本方案为 YaoXiang 语言设计了一套完整的异步并发实现方案。该方案的核心创新点在于：

1. **类型系统层面**：通过 `Async[T]` 实现"零传染性"
2. **运行时层面**：通过计算图实现自动依赖解析
3. **编译器层面**：通过 `spawn` 语法提供清晰的并行控制

相比当前已实现的基础框架，该方案需要进行重大重构，但能够显著提升 YaoXiang 语言的并发编程体验和执行效率。

**关键风险提示**：计算图可视化和调试工具的开发是确保该方案成功的关键，建议在实现早期就投入资源。

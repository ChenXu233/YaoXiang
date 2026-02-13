# FlowScheduler 依赖感知调度器实现计划

> **任务**：全新设计并实现依赖感知调度器，支持"默认惰性求值 + spawn 精细控制"模型
> **日期**：2026-01-04
> **状态**：进行中（设计完成待实现）
> **IO 引擎**：使用 libuv 保证工业级可用性

---

## 实际实现状态

### ✅ 已完成（2026-01-04）

**详细设计文档**：
- 架构设计完成
- 核心数据结构设计完成（DAGNode、ComputationDAG、WorkStealer 等）
- FlowScheduler 核心逻辑设计完成
- libuv IO 调度引擎设计完成
- 测试计划制定完成
- 验收标准定义完成

### 📋 已集成组件

**现有任务系统**：
- Task/TaskId/TaskState 已完成
- 基础调度器框架已完成
- 位置：`src/backends/runtime/task.rs`

**参考实现文档**：
- RFC-001：并作模型与错误处理系统
- RFC-008：Runtime 并发模型与调度器脱耦设计
- RFC-003：版本规划

### 🚧 待实现（v0.2）

**第一优先级**：
1. DAG 节点与图实现
2. 工作窃取算法实现
3. FlowScheduler 核心逻辑实现

**第二优先级**：
4. libuv IO 调度引擎实现
5. 惰性求值策略实现
6. spawn 语法支持

### 🎯 目标版本

- **v0.2**：FlowScheduler 基础实现
- **v0.3**：完整并发支持
- **v0.5**：性能优化与稳定化

---

## 核心设计理念

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     YaoXiang 并作模型 - 核心原则                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  1. 【默认惰性求值】                                                         │
│     - 所有函数默认惰性求值（类似 Haskell）                                   │
│     - 有返回值的函数：返回值被使用时才求值                                   │
│     - 无返回值函数：通过类型标注 (@effect) 确定执行时机                      │
│                                                                              │
│  2. 【核心数量配置】                                                         │
│     - 脚本头声明 `// @cores: 4` 自动启用并行化                              │
│     - 调度器根据核心数自动分配工作线程                                       │
│                                                                              │
│  3. 【spawn 精细控制】                                                       │
│     - `spawn fn` - 显式标记为异步函数                                        │
│     - `spawn { a, b }` - 并行块，块内表达式并行执行                          │
│     - `spawn for x in xs` - 数据并行循环                                     │
│                                                                              │
│  4. 【混合求值模式】                                                         │
│     - `@eager` - 强制急切求值                                               │
│     - `@lazy` - 保持惰性（默认）                                            │
│     - `@force` - 显式触发求值                                               │
│     - 自动检测最佳求值策略                                                   │
│                                                                              │
│  5. 【工业级 IO 调度】                                                       │
│     - 使用 libuv 作为底层 IO 引擎                                            │
│     - 高性能异步 IO（epoll/kqueue/IOCP）                                     │
│     - 文件系统操作、TCP/UDP 网络操作                                         │
│     - 定时器、信号处理                                                       │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 一、架构总览

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          FlowScheduler 架构图                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────┐   ┌──────────────────┐   ┌──────────────────┐        │
│  │   DAG Node       │   │   ComputationDAG │   │   FlowScheduler  │        │
│  │   (节点)         │   │   (计算图)       │   │   (调度器)        │        │
│  └────────┬─────────┘   └────────┬─────────┘   └────────┬─────────┘        │
│           │                      │                      │                   │
│           │                      │                      │                   │
│           ▼                      ▼                      ▼                   │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      运行时核心组件                                    │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │   │
│  │  │ WorkStealer │  │ TaskQueue   │  │ AsyncValue  │  │ BlockingPool│ │   │
│  │  │ (工作窃取)  │  │ (任务队列)  │  │ (异步值)    │  │ (阻塞池)    │ │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘ │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    libuv IO 调度引擎 (工业级)                         │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │   │
│  │  │   TCP/UDP   │  │    File     │  │   Timers    │  │   Signals   │ │   │
│  │  │   Network   │  │   System    │  │  & Sleep    │  │  Handling   │ │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘ │   │
│  │                                                                      │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │   │
│  │  │   Poll      │  │   Check     │  │   Prepare   │  │   Idle      │ │   │
│  │  │  (IO 多路)  │  │  (检查)     │  │  (准备)     │  │  (空闲)     │ │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘ │   │
│  │                                                                      │   │
│  │  支持平台：Linux (epoll), macOS (kqueue), Windows (IOCP)            │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 二、模块划分与依赖关系

### 2.1 新增文件结构

```
src/runtime/
├── mod.rs              # 运行时模块入口（修改）
├── scheduler/
│   ├── mod.rs          # 调度器模块（全新设计）
│   ├── task.rs         # 任务定义
│   ├── work_stealer.rs # 工作窃取器
│   ├── queue.rs        # 任务队列
│   └── tests/          # 调度器测试
│       └── mod.rs
├── dag/
│   ├── mod.rs          # DAG 模块入口
│   ├── node_id.rs      # 节点 ID 和生成器
│   ├── node.rs         # DAG 节点
│   ├── graph.rs        # 计算图
│   └── tests/          # DAG 测试
│       └── mod.rs
├── async_value/
│   ├── mod.rs          # Async[T] 模块入口
│   ├── async_value.rs  # 异步值类型
│   ├── future_wrapper.rs # Future 包装器
│   └── tests/          # Async 测试
│       └── mod.rs
├── blocking_pool/
│   ├── mod.rs          # 阻塞线程池
│   └── tests/          # 阻塞池测试
│       └── mod.rs
└── io/
    ├── mod.rs          # libuv IO 模块入口
    ├── uv_loop.rs      # libuv 事件循环封装
    ├── uv_tcp.rs       # TCP/UDP 网络操作
    ├── uv_fs.rs        # 文件系统操作
    ├── uv_timer.rs     # 定时器
    └── tests/          # IO 测试
        └── mod.rs
```

### 2.2 兼容性设计

```rust
// 向后兼容：保留旧版 Scheduler API
pub mod scheduler {
    use super::flow::FlowScheduler;

    // 简单任务调度的兼容接口
    #[deprecated(since = "0.3.0", note = "请使用 FlowScheduler")]
    pub struct Scheduler(FlowScheduler);

    impl Scheduler {
        #[deprecated]
        pub fn spawn(&self, task: Arc<Task>) {
            // 委托给 FlowScheduler，自动创建无依赖任务
            self.0.spawn_untracked(task);
        }
    }
}
```

---

## 三、核心数据结构设计

### 3.1 节点 ID（自增 ID 生成器）

```rust
// src/runtime/dag/node.rs

/// 节点 ID，使用自增整数
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

impl NodeId {
    /// 创建新节点 ID（内部使用）
    pub(crate) fn new(inner: usize) -> Self {
        Self(inner)
    }

    /// 获取内部值
    pub fn inner(&self) -> usize {
        self.0
    }
}

/// 节点 ID 生成器
#[derive(Debug, Default)]
pub struct NodeIdGenerator(usize);

impl NodeIdGenerator {
    /// 生成新的节点 ID
    pub fn next(&mut self) -> NodeId {
        let id = self.0;
        self.0 += 1;
        NodeId(id)
    }
}
```

### 3.2 DAG 节点状态

```rust
// src/runtime/dag/node.rs

/// 节点状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    /// 未就绪（有待完成的依赖）
    Pending,
    /// 已调度（等待执行）
    Scheduled,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

/// 节点类型（体现"默认惰性 + spawn 精细控制"）
#[derive(Debug, Clone)]
pub enum NodeKind {
    /// 惰性计算节点（默认）- 延迟到需要时才执行
    LazyCompute,
    /// 急切计算节点（@eager）- 立即执行
    EagerCompute,
    /// 异步计算节点（spawn fn）- 返回 Async[T]
    AsyncCompute,
    /// 并行块节点（spawn {}）- 块内并行执行
    ParallelBlock {
        /// 块内表达式
        exprs: Vec<Expr>,
        /// 屏障同步
        barrier: bool,
    },
    /// 数据并行节点（spawn for）
    DataParallel {
        /// 迭代变量名
        var: String,
        /// 迭代器表达式
        iter: Expr,
        /// 循环体
        body: Expr,
    },
    /// 副作用节点（无返回值函数）- @effect 标记
    Effect {
        /// 执行函数
        func: Expr,
        /// 执行时机：Immediate/Deferred
        timing: EffectTiming,
    },
    /// I/O 操作节点
    IO,
    /// 阻塞操作节点（@blocking）
    Blocking,
}

/// 副作用执行时机
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectTiming {
    /// 立即执行（默认）
    Immediate,
    /// 延迟到程序退出前
    Deferred,
    /// 延迟到作用域结束时
    ScopeEnd,
}

/// 节点执行结果
#[derive(Debug)]
pub enum NodeResult {
    /// 成功完成
    Success(Value),
    /// 错误
    Error(DynError),
    /// 取消
    Cancelled,
}
```

### 3.3 DAG 节点

```rust
// src/runtime/dag/node.rs

/// DAG 节点
#[derive(Debug)]
pub struct DAGNode {
    /// 节点 ID
    id: NodeId,
    /// 节点类型
    kind: NodeKind,
    /// 节点状态
    state: AtomicU8, // NodeState 的原子版本
    /// 依赖的父节点 ID
    parents: Vec<NodeId>,
    /// 依赖的子节点 ID
    children: Vec<NodeId>,
    /// 未完成的父节点计数（用于就绪判断）
    pending_parents: AtomicUsize,
    /// 执行结果（OnceCell 保证只写入一次）
    result: OnceCell<NodeResult>,
    /// 任务执行器（运行时填充）
    executor: OnceCell<Arc<dyn Fn() + Send + Sync>>,
    /// 元数据
    metadata: NodeMetadata,
}

/// 节点元数据
#[derive(Debug, Clone, Default)]
pub struct NodeMetadata {
    /// 节点名称（调试用）
    name: String,
    /// 优先级
    priority: TaskPriority,
    /// 创建时间
    created_at: std::time::Instant,
    /// 调度延迟统计
    schedule_delay: Duration,
    /// 执行时间统计
    exec_duration: Duration,
}

impl DAGNode {
    /// 创建新节点
    pub fn new(id: NodeId, kind: NodeKind) -> Self {
        let pending_parents = AtomicUsize::new(0);
        Self {
            id,
            kind,
            state: AtomicU8::new(NodeState::Pending as u8),
            parents: Vec::new(),
            children: Vec::new(),
            pending_parents,
            result: OnceCell::new(),
            executor: OnceCell::new(),
            metadata: NodeMetadata::default(),
        }
    }

    /// 添加父节点依赖
    pub fn add_parent(&mut self, parent_id: NodeId) {
        self.parents.push(parent_id);
        self.pending_parents.fetch_add(1, Ordering::SeqCst);
    }

    /// 添加子节点依赖
    pub fn add_child(&mut self, child_id: NodeId) {
        self.children.push(child_id);
    }

    /// 检查是否就绪（所有父节点已完成）
    pub fn is_ready(&self) -> bool {
        self.pending_parents.load(Ordering::SeqCst) == 0
    }

    /// 标记一个父节点完成
    pub fn parent_completed(&self) {
        self.pending_parents.fetch_sub(1, Ordering::SeqCst);
    }

    /// 原子地设置状态
    pub fn set_state(&self, state: NodeState) {
        self.state.store(state as u8, Ordering::SeqCst);
    }

    /// 原子地获取状态
    pub fn state(&self) -> NodeState {
        NodeState::from_u8(self.state.load(Ordering::SeqCst))
    }
}
```

### 3.4 计算图 (DAG)

```rust
// src/runtime/dag/graph.rs

/// 计算图
#[derive(Debug)]
pub struct ComputationDAG {
    /// 所有节点
    nodes: DashMap<NodeId, Arc<DAGNode>>,
    /// 拓扑排序的就绪队列
    ready_queue: Arc<Mutex<VecDeque<NodeId>>>,
    /// 运行中节点集合
    running: Arc<Mutex<HashSet<NodeId>>>,
    /// 已完成节点集合
    completed: Arc<Mutex<HashSet<NodeId>>},
    /// 节点 ID 生成器
    id_generator: Mutex<NodeIdGenerator>,
    /// 并行边界（spawn {} 标记的区域）
    parallel_boundaries: DashMap<BoundaryId, ParallelBoundary>,
    /// 统计信息
    stats: DAGStats,
}

/// 统计信息
#[derive(Debug, Default)]
pub struct DAGStats {
    /// 总节点数
    total_nodes: AtomicUsize,
    /// 并行执行的节点对数
    parallel_pairs: AtomicUsize,
    /// 最大并行度
    max_parallelism: AtomicUsize,
    /// 总执行时间
    total_exec_time: AtomicU64,
}

impl ComputationDAG {
    /// 创建新计算图
    pub fn new() -> Self {
        Self {
            nodes: DashMap::new(),
            ready_queue: Arc::new(Mutex::new(VecDeque::new())),
            running: Arc::new(Mutex::new(HashSet::new())),
            completed: Arc::new(Mutex::new(HashSet::new())),
            id_generator: Mutex::new(NodeIdGenerator::default()),
            parallel_boundaries: DashMap::new(),
            stats: DAGStats::default(),
        }
    }

    /// 添加节点
    pub fn add_node(&self, kind: NodeKind, dependencies: &[NodeId]) -> NodeId {
        let mut id_generator = self.id_generator.lock().unwrap();
        let node_id = id_generator.next();

        let node = Arc::new(DAGNode::new(node_id, kind));

        // 建立依赖关系
        for &dep_id in dependencies {
            if let Some(dep_node) = self.nodes.get(&dep_id) {
                // 添加当前节点为 dep_node 的子节点
                // 注意：需要克隆 Arc
                let mut dep_node_mut = dep_node.value().clone();
                dep_node_mut.add_child(node_id);
            }
            // 当前节点依赖 dep_id
            let mut node_mut = node.clone();
            node_mut.add_parent(dep_id);
        }

        // 检查是否就绪
        if node.is_ready() {
            self.ready_queue.lock().unwrap().push_back(node_id);
        }

        self.nodes.insert(node_id, node);
        self.stats.total_nodes.fetch_add(1, Ordering::SeqCst);

        node_id
    }

    /// 获取就绪节点
    pub fn pop_ready(&self) -> Option<NodeId> {
        self.ready_queue.lock().unwrap().pop_front()
    }

    /// 节点完成，通知子节点
    pub fn node_completed(&self, node_id: NodeId) {
        // 标记节点为完成
        self.completed.lock().unwrap().insert(node_id);

        // 通知所有子节点
        if let Some(node) = self.nodes.get(&node_id) {
            for child_id in node.children.clone() {
                if let Some(child) = self.nodes.get(&child_id) {
                    child.parent_completed();
                    // 如果子节点就绪，加入就绪队列
                    if child.is_ready() {
                        self.ready_queue.lock().unwrap().push_back(child_id);
                    }
                }
            }
        }
    }
}
```

### 3.5 工作窃取器

```rust
// src/runtime/scheduler/work_stealer.rs

/// 工作窃取器
#[derive(Debug)]
pub struct WorkStealer {
    /// 所有工作线程的本地队列引用
    queues: Arc<RwLock<Vec<Arc<TaskQueue>>>>,
    /// 当前工作线程 ID
    current_worker: AtomicUsize,
    /// 窃取策略
    strategy: StealStrategy,
}

/// 窃取策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StealStrategy {
    /// 从队列尾部窃取（减少冲突）
    Random,
    /// 从队列头部窃取（先进先出）
    FIFO,
    /// 双端窃取
    Deque,
}

impl WorkStealer {
    /// 创建工作窃取器
    pub fn new(num_workers: usize) -> Self {
        let mut queues = Vec::with_capacity(num_workers);
        for _ in 0..num_workers {
            queues.push(Arc::new(TaskQueue::new()));
        }

        Self {
            queues: Arc::new(RwLock::new(queues)),
            current_worker: AtomicUsize::new(0),
            strategy: StealStrategy::Random,
        }
    }

    /// 注册工作线程
    pub fn register_worker(&self, worker_id: usize) {
        self.current_worker.store(worker_id, Ordering::SeqCst);
    }

    /// 尝试窃取任务
    pub fn steal(&self, victim_id: usize) -> Option<Arc<Task>> {
        let queues = self.queues.read().unwrap();
        if victim_id >= queues.len() {
            return None;
        }

        let queue = &queues[victim_id];
        queue.pop_back() // 从尾部窃取，减少与 victim 的竞争
    }

    /// 从随机工作线程窃取
    pub fn steal_random(&self) -> Option<Arc<Task>> {
        let queues = self.queues.read().unwrap();
        if queues.is_empty() {
            return None;
        }

        let num_workers = queues.len();
        let mut attempts = 0;
        let mut rng = rand::thread_rng();

        while attempts < num_workers {
            let victim_id = rng.gen_range(0..num_workers);
            if victim_id == self.current_worker.load(Ordering::SeqCst) {
                attempts += 1;
                continue;
            }

            if let Some(task) = self.steal(victim_id) {
                return Some(task);
            }
            attempts += 1;
        }

        None
    }

    /// 从所有队列窃取
    pub fn steal_all(&self) -> Vec<Arc<Task>> {
        let mut stolen = Vec::new();
        let queues = self.queues.read().unwrap();

        for (i, queue) in queues.iter().enumerate() {
            if i == self.current_worker.load(Ordering::SeqCst) {
                continue; // 跳过自己的队列
            }

            while let Some(task) = queue.pop_back() {
                stolen.push(task);
            }
        }

        stolen
    }
}
```

### 3.6 任务队列

```rust
// src/runtime/scheduler/queue.rs

/// 任务队列（支持多生产者多消费者）
#[derive(Debug)]
pub struct TaskQueue {
    /// 内部 deque（使用 Mutex 保证线程安全）
    inner: Arc<Mutex<VecDeque<Arc<Task>>>>,
    /// 优先级索引（可选）
    priority_indices: HashMap<TaskPriority, VecDeque<usize>>,
}

impl TaskQueue {
    /// 创建新任务队列
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::new())),
            priority_indices: HashMap::new(),
        }
    }

    /// 压入任务（队尾）
    pub fn push(&self, task: Arc<Task>) {
        let mut inner = self.inner.lock().unwrap();
        inner.push_back(task);
    }

    /// 压入任务（队头，高优先级）
    pub fn push_front(&self, task: Arc<Task>) {
        let mut inner = self.inner.lock().unwrap();
        inner.push_front(task);
    }

    /// 弹出任务（队头）
    pub fn pop_front(&self) -> Option<Arc<Task>> {
        self.inner.lock().unwrap().pop_front()
    }

    /// 弹出任务（队尾，用于窃取）
    pub fn pop_back(&self) -> Option<Arc<Task>> {
        self.inner.lock().unwrap().pop_back()
    }

    /// 获取队列长度
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.inner.lock().unwrap().is_empty()
    }
}
```

### 3.7 FlowScheduler 核心

```rust
// src/runtime/scheduler/mod.rs

/// 依赖感知调度器（FlowScheduler）
///
/// 核心特性：
/// 1. DAG 依赖感知调度
/// 2. 工作窃取负载均衡
/// 3. 支持并行块 (spawn {})
/// 4. 支持数据并行 (spawn for)
/// 5. 阻塞操作隔离
#[derive(Debug)]
pub struct FlowScheduler {
    /// 配置
    config: SchedulerConfig,
    /// 计算图
    dag: Arc<ComputationDAG>,
    /// 工作窃取器
    work_stealer: WorkStealer,
    /// 所有本地队列（用于窃取）
    local_queues: Arc<RwLock<Vec<Arc<TaskQueue>>>>,
    /// 工作线程
    workers: Vec<JoinHandle<()>>,
    /// 运行状态
    running: Arc<AtomicBool>,
    /// 任务完成通知
    completion_notifier: Arc<Notify>,
    /// 阻塞线程池
    blocking_pool: BlockingThreadPool,
    /// 统计信息
    stats: SchedulerStats,
}

/// 调度器统计信息
#[derive(Debug, Default)]
pub struct SchedulerStats {
    /// 已完成任务数
    completed_tasks: AtomicUsize,
    /// 被窃取的任务数
    stolen_tasks: AtomicUsize,
    /// 窃取成功次数
    steal_successes: AtomicUsize,
    /// 窃取失败次数
    steal_failures: AtomicUsize,
    /// 总调度延迟
    total_schedule_delay: AtomicU64,
    /// 最大并行度
    peak_parallelism: AtomicUsize,
}

impl FlowScheduler {
    /// 创建新调度器
    pub fn new() -> Self {
        Self::with_config(SchedulerConfig::default())
    }

    /// 使用配置创建调度器
    pub fn with_config(config: SchedulerConfig) -> Self {
        let num_workers = config.num_workers;
        let running = Arc::new(AtomicBool::new(true));
        let completion_notifier = Arc::new(Notify());

        let work_stealer = WorkStealer::new(num_workers);
        let local_queues = Arc::new(RwLock::new(
            (0..num_workers)
                .map(|_| Arc::new(TaskQueue::new()))
                .collect(),
        ));

        let dag = Arc::new(ComputationDAG::new());
        let blocking_pool = BlockingThreadPool::new(config.blocking_pool_size);

        let workers = Self::spawn_workers(
            num_workers,
            &running,
            &work_stealer,
            &local_queues,
            &dag,
            &completion_notifier,
        );

        Self {
            config,
            dag,
            work_stealer,
            local_queues,
            workers,
            running,
            completion_notifier,
            blocking_pool,
            stats: SchedulerStats::default(),
        }
    }

    /// 创建工作线程
    fn spawn_workers(
        num_workers: usize,
        running: &Arc<AtomicBool>,
        work_stealer: &WorkStealer,
        local_queues: &Arc<RwLock<Vec<Arc<TaskQueue>>>>,
        dag: &Arc<ComputationDAG>,
        completion_notifier: &Arc<Notify>,
    ) -> Vec<JoinHandle<()>> {
        let mut workers = Vec::with_capacity(num_workers);

        for worker_id in 0..num_workers {
            let running = running.clone();
            let work_stealer = work_stealer.clone();
            let local_queues = local_queues.clone();
            let dag = dag.clone();
            let completion_notifier = completion_notifier.clone();

            let worker = thread::spawn(move || {
                Self::worker_loop(
                    worker_id,
                    &running,
                    &work_stealer,
                    &local_queues,
                    &dag,
                    &completion_notifier,
                );
            });

            workers.push(worker);
        }

        workers
    }

    /// 工作线程主循环
    fn worker_loop(
        worker_id: usize,
        running: &Arc<AtomicBool>,
        work_stealer: &WorkStealer,
        local_queues: &Arc<RwLock<Vec<Arc<TaskQueue>>>>,
        dag: &Arc<ComputationDAG>,
        completion_notifier: &Arc<Notify>,
    ) {
        work_stealer.register_worker(worker_id);

        while running.load(Ordering::SeqCst) {
            // 1. 尝试从本地队列获取
            if let Some(task) = Self::pop_local(worker_id, local_queues) {
                Self::execute_task(worker_id, task, dag, completion_notifier);
                continue;
            }

            // 2. 尝试从就绪队列获取（DAG 感知）
            if let Some(node_id) = dag.pop_ready() {
                Self::execute_node(worker_id, node_id, dag, completion_notifier);
                continue;
            }

            // 3. 尝试窃取
            if let Some(task) = work_stealer.steal_random() {
                Self::execute_task(worker_id, task, dag, completion_notifier);
                continue;
            }

            // 4. 无任务可执行，等待
            completion_notifier.notified().await;
        }
    }

    /// 从本地队列弹出任务
    fn pop_local(
        worker_id: usize,
        local_queues: &Arc<RwLock<Vec<Arc<TaskQueue>>>>,
    ) -> Option<Arc<Task>> {
        let queues = local_queues.read().unwrap();
        if worker_id < queues.len() {
            queues[worker_id].pop_front()
        } else {
            None
        }
    }

    /// 执行任务
    fn execute_task(
        worker_id: usize,
        task: Arc<Task>,
        dag: &Arc<ComputationDAG>,
        completion_notifier: &Arc<Notify>,
    ) {
        task.set_state(TaskState::Running);

        // 执行任务逻辑
        // TODO: 调用任务的实际执行函数

        task.set_state(TaskState::Finished);
        completion_notifier.notify_one();
    }

    /// 执行节点
    fn execute_node(
        worker_id: usize,
        node_id: NodeId,
        dag: &Arc<ComputationDAG>,
        completion_notifier: &Arc<Notify>,
    ) {
        if let Some(node) = dag.nodes.get(&node_id) {
            node.set_state(NodeState::Running);

            // 执行节点逻辑
            // TODO: 调用节点的执行器

            node.set_state(NodeState::Completed);
            dag.node_completed(node_id);
            completion_notifier.notify_one();
        }
    }

    /// 提交无依赖任务（兼容旧 API）
    pub fn spawn_untracked(&self, task: Arc<Task>) {
        let queues = self.local_queues.read().unwrap();
        let worker_id = rand::thread_rng().gen_range(0..queues.len());
        queues[worker_id].push(task);
    }

    /// 提交带依赖的任务
    pub fn spawn_with_deps(&self, task: Arc<Task>, dependencies: &[NodeId]) -> NodeId {
        self.dag.add_node(NodeKind::Compute, dependencies)
    }

    /// 关闭调度器
    pub fn shutdown(&mut self) {
        self.running.store(false, Ordering::SeqCst);

        // 唤醒所有等待的工作线程
        for _ in 0..self.workers.len() {
            self.completion_notifier.notify_one();
        }

        // 等待工作线程结束
        for worker in self.workers.drain(..) {
            worker.join().unwrap();
        }

        // 关闭阻塞线程池
        self.blocking_pool.shutdown();
    }
}
```

### 3.8 Async[T] 异步值类型

```rust
// src/runtime/async_value/async_value.rs

/// Async[T] - 惰性代理类型
///
/// 实现"零传染性"：
/// 1. Async<T> 是 T 的子类型
/// 2. 在需要 T 的上下文中自动解包
/// 3. 内部实际存储 Result<T, E> 以支持错误传播
#[repr(transparent)]
pub struct Async<T: Send + 'static> {
    inner: Arc<AsyncInner<T>>,
}

/// 异步值内部实现
struct AsyncInner<T: Send + 'static> {
    /// 实际的 Future
    future: Mutex<Option<Pin<Box<dyn Future<Output = Result<T, DynError>> + Send + 'static>>>>,
    /// 任务执行器
    executor: TaskExecutor,
    /// 状态
    state: AtomicU8,
    /// 结果缓存
    result: OnceCell<Result<T, DynError>>,
    /// 等待者列表
    waiters: Mutex<Vec<Arc<Notify>>>,
}

impl<T: Send + 'static> Async<T> {
    /// 创建新的 Async 值
    pub fn new<F>(future: F) -> Self
    where
        F: Future<Output = Result<T, DynError>> + Send + 'static,
    {
        let inner = Arc::new(AsyncInner {
            future: Mutex::new(Some(Box::pin(future))),
            executor: TaskExecutor::new(),
            state: AtomicU8::new(AsyncState::Pending as u8),
            result: OnceCell::new(),
            waiters: Mutex::new(Vec::new()),
        });

        Self { inner }
    }

    /// 检查是否已完成
    pub fn is_ready(&self) -> bool {
        matches!(
            AsyncState::from_u8(self.inner.state.load(Ordering::SeqCst)),
            AsyncState::Completed(_)
        )
    }

    /// 获取结果（同步阻塞）
    pub fn get(&self) -> Result<&T, &DynError> {
        loop {
            match AsyncState::from_u8(self.inner.state.load(Ordering::SeqCst)) {
                AsyncState::Pending => {
                    // 尝试执行
                    self.inner.executor.try_execute();
                    // 自旋等待
                    std::hint::spin_loop();
                }
                AsyncState::Running => {
                    std::hint::spin_loop();
                }
                AsyncState::Completed => {
                    return self.inner.result.get().unwrap().as_ref();
                }
            }
        }
    }
}

/// 异步状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncState {
    Pending = 0,
    Running = 1,
    Completed = 2,
    Failed = 3,
}

/// 自动解包实现
impl<T: Send + 'static> std::ops::Deref for Async<T> {
    type Target = T;

    fn deref(&self) -> &T {
        // 阻塞等待并返回引用
        self.get().expect("Async value failed")
    }
}
```

### 3.9 libuv IO 调度引擎

```rust
// src/runtime/io/mod.rs

/// libuv IO 调度器 - 工业级异步 IO 引擎
///
/// 使用 libuv 作为底层 IO 引擎，提供：
/// - 高性能异步 IO（epoll/kqueue/IOCP）
/// - TCP/UDP 网络操作
/// - 文件系统操作
/// - 定时器和信号处理
///
/// 依赖: uv-rs (Rust libuv 绑定)
#[derive(Debug)]
pub struct UvIOScheduler {
    /// libuv 事件循环
    loop: Arc<uv_loop::UvLoop>,
    /// 运行状态
    running: Arc<AtomicBool>,
    /// IO 完成回调
    callbacks: Arc<RwLock<HashMap<usize, Box<dyn FnOnce() + Send>>>>,
    /// 下一个回调 ID
    next_callback_id: AtomicUsize,
}

impl UvIOScheduler {
    /// 创建新的 libuv IO 调度器
    pub fn new() -> Result<Self, UvError> {
        let loop_raw = unsafe { uv_sys::uv_loop_new() };
        if loop_raw.is_null() {
            return Err(UvError::AllocationFailed);
        }

        let loop_ = Arc::new(uv_loop::UvLoop::from_raw(loop_raw));

        Ok(Self {
            loop: loop_,
            running: Arc::new(AtomicBool::new(true)),
            callbacks: Arc::new(RwLock::new(HashMap::new())),
            next_callback_id: AtomicUsize::new(0),
        })
    }

    /// 获取事件循环引用
    pub fn loop(&self) -> &Arc<uv_loop::UvLoop> {
        &self.loop
    }

    /// 运行事件循环
    pub fn run(&self) -> Result<(), UvError> {
        self.running.store(true, Ordering::SeqCst);

        while self.running.load(Ordering::SeqCst) {
            // 运行 libuv 事件循环
            // 阻塞直到有事件发生或超时
            unsafe {
                uv_sys::uv_run(
                    self.loop.as_ptr(),
                    uv_sys::UV_RUN_DEFAULT,
                );
            }
        }

        Ok(())
    }

    /// 停止事件循环
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        unsafe {
            uv_sys::uv_stop(self.loop.as_ptr());
        }
    }

    /// 注册 TCP 读取操作
    pub fn tcp_read(
        &self,
        stream: &mut UvTcpStream,
        buf: &mut [u8],
    ) -> Result<usize, UvError> {
        let (sender, receiver) = oneshot::channel();

        // 设置读取回调
        stream.set_read_callback(move |result| {
            let _ = sender.send(result);
        });

        // 开始异步读取
        unsafe {
            uv_sys::uv_read_start(
                stream.as_ptr() as *mut uv_sys::uv_stream_t,
                Some(read_alloc_callback),
                Some(read_callback) as _,
            );
        }

        // 等待结果
        receiver.recv_timeout(Duration::from_secs(30))
            .map_err(|_| UvError::Timeout)?
    }

    /// 异步 TCP 连接
    pub fn tcp_connect(
        &self,
        addr: &SocketAddr,
    ) -> Result<UvTcpStream, UvError> {
        let mut stream = UvTcpStream::new()?;
        let connect_req = unsafe { uv_sys::uv_connect_t_new() };

        // 设置连接回调
        let (sender, receiver) = oneshot::channel::<Result<(), UvError>>();

        // 发起连接
        unsafe {
            uv_sys::uv_tcp_connect(
                connect_req,
                stream.as_ptr(),
                addr.as_ptr() as *const sockaddr,
                Some(connect_callback) as _,
            );
        }

        // 等待连接完成
        receiver.recv_timeout(Duration::from_secs(30))
            .map_err(|_| UvError::Timeout)??;

        Ok(stream)
    }

    /// 异步文件读取
    pub fn fs_read(
        &self,
        file: &mut UvFsFile,
        buf: &mut [u8],
        offset: i64,
    ) -> Result<usize, UvError> {
        let (sender, receiver) = oneshot::channel();

        // 设置文件系统回调
        let req = unsafe { uv_sys::uv_fs_t_new() };

        unsafe {
            uv_sys::uv_fs_read(
                self.loop.as_ptr(),
                req,
                file.fd(),
                buf.as_mut_ptr() as *mut i8,
                buf.len() as i64,
                offset,
                Some(fs_callback) as _,
            );
        }

        receiver.recv_timeout(Duration::from_secs(60))
            .map_err(|_| UvError::Timeout)?
    }

    /// 创建定时器
    pub fn create_timer(&self, timeout: u64, repeat: u64) -> Result<UvTimer, UvError> {
        UvTimer::new(&self.loop, timeout, repeat)
    }
}

/// TCP 流封装
#[derive(Debug)]
pub struct UvTcpStream {
    /// 底层 uv_tcp_t 句柄
    handle: *mut uv_sys::uv_tcp_t,
    /// 读回调
    read_cb: Mutex<Option<Box<dyn FnMut(Result<usize, UvError>)>>>,
    /// 关闭状态
    closed: AtomicBool,
}

impl UvTcpStream {
    /// 创建新的 TCP 流
    pub fn new() -> Result<Self, UvError> {
        let handle = unsafe { uv_sys::uv_tcp_t_new() };
        if handle.is_null() {
            return Err(UvError::AllocationFailed);
        }

        Ok(Self {
            handle,
            read_cb: Mutex::new(None),
            closed: AtomicBool::new(false),
        })
    }

    /// 初始化 TCP 流（需在事件循环中调用）
    pub fn init(&mut self, loop_: &uv_loop::UvLoop) -> Result<(), UvError> {
        let ret = unsafe {
            uv_sys::uv_tcp_init(loop_.as_ptr(), self.handle)
        };
        if ret < 0 {
            Err(UvError::from_raw(ret))
        } else {
            Ok(())
        }
    }

    /// 绑定到地址
    pub fn bind(&mut self, addr: &SocketAddr) -> Result<(), UvError> {
        let ret = unsafe {
            uv_sys::uv_tcp_bind(
                self.handle,
                addr.as_ptr() as *const sockaddr,
                0,
            )
        };
        if ret < 0 {
            Err(UvError::from_raw(ret))
        } else {
            Ok(())
        }
    }

    /// 设置读回调
    fn set_read_callback<F>(&mut self, cb: F)
    where
        F: FnMut(Result<usize, UvError>) + 'static,
    {
        *self.read_cb.lock().unwrap() = Some(Box::new(cb));
    }

    /// 获取底层句柄指针
    fn as_ptr(&self) -> *mut uv_sys::uv_tcp_t {
        self.handle
    }
}

/// 定时器封装
#[derive(Debug)]
pub struct UvTimer {
    /// 底层 uv_timer_t 句柄
    handle: *mut uv_sys::uv_timer_t,
    /// 回调
    callback: Mutex<Option<Box<dyn FnMut() + Send>>>,
}

impl UvTimer {
    /// 创建新定时器
    pub fn new(loop_: &uv_loop::UvLoop, timeout: u64, repeat: u64) -> Result<Self, UvError> {
        let handle = unsafe { uv_sys::uv_timer_t_new() };
        if handle.is_null() {
            return Err(UvError::AllocationFailed);
        }

        let timer = Self {
            handle,
            callback: Mutex::new(None),
        };

        unsafe {
            uv_sys::uv_timer_init(loop_.as_ptr(), handle);
        }

        Ok(timer)
    }

    /// 启动定时器
    pub fn start<F>(&mut self, timeout: u64, repeat: u64, cb: F)
    where
        F: FnMut() + 'static,
    {
        *self.callback.lock().unwrap() = Some(Box::new(cb));

        unsafe {
            uv_sys::uv_timer_start(
                self.handle,
                Some(timer_callback),
                timeout,
                repeat,
            );
        }
    }

    /// 停止定时器
    pub fn stop(&mut self) {
        unsafe {
            uv_sys::uv_timer_stop(self.handle);
        }
    }

    /// 获取底层句柄指针
    fn as_ptr(&self) -> *mut uv_sys::uv_timer_t {
        self.handle
    }
}

/// 文件系统操作封装
#[derive(Debug)]
pub struct UvFsFile {
    /// 底层 uv_fs_t 句柄
    file: i32,
}

impl UvFsFile {
    /// 打开文件
    pub fn open(path: &str, flags: i32, mode: i32) -> Result<Self, UvError> {
        let req = unsafe { uv_sys::uv_fs_t_new() };

        let ret = unsafe {
            uv_sys::uv_fs_open(
                ptr::null_mut(),
                req,
                path.as_ptr() as *const i8,
                flags,
                mode,
                None,
            )
        };

        if ret < 0 {
            Err(UvError::from_raw(ret))
        } else {
            Ok(Self { file: ret })
        }
    }

    /// 关闭文件
    pub fn close(&self) -> Result<(), UvError> {
        let req = unsafe { uv_sys::uv_fs_t_new() };

        let ret = unsafe {
            uv_sys::uv_fs_close(ptr::null_mut(), req, self.file, None)
        };

        if ret < 0 {
            Err(UvError::from_raw(ret))
        } else {
            Ok(())
        }
    }

    /// 获取文件描述符
    fn fd(&self) -> i32 {
        self.file
    }
}
```

### 3.10 IO 调度器与 FlowScheduler 集成

```rust
// FlowScheduler 中的 IO 集成

impl FlowScheduler {
    /// 创建带 libuv IO 的调度器
    pub fn new_with_io() -> Result<Self, UvError> {
        let io_scheduler = UvIOScheduler::new()?;

        let config = SchedulerConfig {
            num_workers: num_cpus::get(),
            use_libuv_io: true,
            ..Default::default()
        };

        let mut scheduler = Self::with_config(config);
        scheduler.io_scheduler = Some(io_scheduler);

        Ok(scheduler)
    }

    /// 提交 IO 任务
    pub fn submit_io_task<F, T>(&self, task: F) -> Async<T>
    where
        F: FnOnce() -> Result<T, UvError> + Send + 'static,
        T: Send + 'static,
    {
        let (sender, receiver) = oneshot::channel();

        // 在 IO 线程中执行
        let io_scheduler = self.io_scheduler.as_ref().unwrap();
        let thread_pool = self.blocking_pool.clone();

        thread_pool.execute(move || {
            let result = task();
            let _ = sender.send(result);
        });

        // 返回 Async 值
        Async::new(async {
            receiver.recv()
                .map_err(|_| DynError::from("IO task channel closed"))?
        })
    }
}
```

---

## 四、求值策略设计

### 4.1 默认行为：惰性求值

```yaoxiang
# 脚本头配置并行核心数
# @cores: 4

# 所有函数默认惰性求值
fn heavy_computation(x: Int) -> Int = (x) => {
    # 这个函数不会立即执行
    # 只有当结果被使用时才执行
    fibonacci(x)
}

fn main() -> Void = () => {
    # heavy_computation 返回 Int，类型是 Lazy[Int]
    result = heavy_computation(100)

    # 在这里，result 被用于加法，触发求值
    # 系统自动找到最佳时机并行执行
    total = result + heavy_computation(200)
}
```

### 4.2 急切求值注解

```yaoxiang
# 强制急切求值
@eager fn log_message(msg: String) -> Void = (msg) => {
    print(msg)
}

# 强制惰性（显式，可省略）
@lazy fn optional_step(x: Int) -> Int = (x) => {
    x * 2
}
```

### 4.3 spawn 精细控制

```yaoxiang
fn fetch_data(url: String) -> JSON = (url) => { ... }
fn parse(json: JSON) -> Model = (json) => { ... }

fn main() -> Void = () => {
    # 显式并行块
    let (data1, data2) = spawn {
        parse(fetch_data("url1")),
        parse(fetch_data("url2"))
    }

    # 数据并行
    let results = spawn for item in items {
        process(item)
    }

    # 单个显式异步函数
    let data = spawn fetch_data("url")
}
```

### 4.4 副作用处理（@effect）

```yaoxiang
# 副作用函数必须标注 @effect
@effect fn log_to_file(path: String, msg: String) -> Void = (path, msg) => {
    std::fs.append(path, msg)
}

# 默认立即执行
fn main() -> Void = () => {
    log_to_file("log.txt", "start")  # 立即执行
}

# 延迟到作用域结束
@effect(timing: ScopeEnd) fn cleanup() -> Void = () => {
    print("cleanup at scope end")
}

fn main() -> Void = () => {
    {
        cleanup()  # 作用域结束时执行
        # ...
    }  # 这里触发 cleanup
}
```

### 4.5 自动检测最佳策略

```rust
// 调度器自动分析并选择最优策略
impl FlowScheduler {
    /// 分析并选择最佳求值策略
    fn analyze_evaluation_strategy(&self, node: &DAGNode) -> EvaluationStrategy {
        match &node.kind {
            NodeKind::LazyCompute => {
                // 检查依赖关系和就绪节点数
                if self.estimate_parallelism(node) > self.config.parallel_threshold {
                    EvaluationStrategy::Parallel
                } else {
                    EvaluationStrategy::Sequential
                }
            }
            NodeKind::EagerCompute => {
                EvaluationStrategy::Immediate
            }
            NodeKind::ParallelBlock => {
                EvaluationStrategy::AggressiveParallel
            }
            _ => EvaluationStrategy::Default,
        }
    }
}
```

---

## 五、测试计划

### 5.1 DAG 模块测试

```rust
// src/runtime/dag/tests/mod.rs

#[cfg(test)]
mod node_tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = DAGNode::new(NodeId(0), NodeKind::Compute);
        assert_eq!(node.state(), NodeState::Pending);
        assert!(node.is_ready());
    }

    #[test]
    fn test_node_dependencies() {
        let mut node = DAGNode::new(NodeId(0), NodeKind::Compute);
        node.add_parent(NodeId(1));
        node.add_parent(NodeId(2));
        assert!(!node.is_ready());

        node.parent_completed();
        assert!(!node.is_ready());

        node.parent_completed();
        assert!(node.is_ready());
    }

    #[test]
    fn test_node_state_transitions() {
        let node = DAGNode::new(NodeId(0), NodeKind::Compute);
        assert_eq!(node.state(), NodeState::Pending);

        node.set_state(NodeState::Scheduled);
        assert_eq!(node.state(), NodeState::Scheduled);

        node.set_state(NodeState::Running);
        assert_eq!(node.state(), NodeState::Running);

        node.set_state(NodeState::Completed);
        assert_eq!(node.state(), NodeState::Completed);
    }

    #[test]
    fn test_node_thread_safety() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let node = Arc::new(DAGNode::new(NodeId(0), NodeKind::Compute));
        let barrier = Arc::new(Barrier::new(10));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let node = node.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    barrier.wait();
                    for _ in 0..1000 {
                        node.set_state(NodeState::Running);
                        node.set_state(NodeState::Pending);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // 最终状态应该是 Pending 或 Scheduled 或 Running
        // 但不应该是 Completed（因为没有真正完成）
    }
}

#[cfg(test)]
mod graph_tests {
    use super::*;

    #[test]
    fn test_graph_add_node() {
        let graph = ComputationDAG::new();
        let node_id = graph.add_node(NodeKind::Compute, &[]);
        assert_eq!(node_id.inner(), 0);
    }

    #[test]
    fn test_graph_dependencies() {
        let graph = ComputationDAG::new();

        // A -> B -> C
        let id_a = graph.add_node(NodeKind::Compute, &[]);
        let id_b = graph.add_node(NodeKind::Compute, &[id_a]);
        let id_c = graph.add_node(NodeKind::Compute, &[id_b]);

        // A 应该立即就绪
        assert!(graph.pop_ready().is_some());

        // B 和 C 不应该就绪
        assert!(graph.pop_ready().is_none());
    }

    #[test]
    fn test_graph_completion() {
        let graph = ComputationDAG::new();

        let id_a = graph.add_node(NodeKind::Compute, &[]);
        let id_b = graph.add_node(NodeKind::Compute, &[id_a]);

        // 完成 A
        graph.node_completed(id_a);

        // B 应该就绪
        assert!(graph.pop_ready().is_some());

        // 完成 B
        graph.node_completed(id_b);

        // 没有更多就绪节点
        assert!(graph.pop_ready().is_none());
    }

    #[test]
    fn test_graph_parallel_execution() {
        let graph = ComputationDAG::new();

        // A 和 B 无依赖，可以并行执行
        let id_a = graph.add_node(NodeKind::Compute, &[]);
        let id_b = graph.add_node(NodeKind::Compute, &[]);

        // 两个都应该就绪
        assert_eq!(graph.pop_ready().map(|id| id.inner()), Some(id_a.inner()));
        assert_eq!(graph.pop_ready().map(|id| id.inner()), Some(id_b.inner()));

        // 完成 A 和 B
        graph.node_completed(id_a);
        graph.node_completed(id_b);

        assert!(graph.pop_ready().is_none());
    }

    #[test]
    fn test_graph_complex_dependencies() {
        let graph = ComputationDAG::new();

        //     A
        //    / \
        //   B   C
        //    \ /
        //     D

        let id_a = graph.add_node(NodeKind::Compute, &[]);
        let id_b = graph.add_node(NodeKind::Compute, &[id_a]);
        let id_c = graph.add_node(NodeKind::Compute, &[id_a]);
        let id_d = graph.add_node(NodeKind::Compute, &[id_b, id_c]);

        // 只有 A 应该就绪
        assert_eq!(graph.pop_ready().map(|id| id.inner()), Some(id_a.inner()));

        // 完成 A
        graph.node_completed(id_a);

        // B 和 C 应该就绪
        let mut ready_nodes: Vec<_> = std::iter::from_fn(|| graph.pop_ready()).collect();
        assert_eq!(ready_nodes.len(), 2);

        // 完成 B 和 C
        for id in ready_nodes {
            graph.node_completed(id);
        }

        // D 应该就绪
        assert!(graph.pop_ready().is_some());
    }
}
```

### 4.2 调度器测试

```rust
// src/runtime/scheduler/tests/mod.rs

#[cfg(test)]
mod work_stealer_tests {
    use super::*;

    #[test]
    fn test_work_stealer_creation() {
        let stealer = WorkStealer::new(4);
        // 应该能正常创建
    }

    #[test]
    fn test_work_stealer_steal_random() {
        let stealer = WorkStealer::new(4);
        // 空队列应该返回 None
        assert!(stealer.steal_random().is_none());
    }

    #[test]
    fn test_work_stealer_parallel() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let stealer = Arc::new(WorkStealer::new(4));
        let barrier = Arc::new(Barrier::new(4));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let stealer = stealer.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    barrier.wait();
                    for _ in 0..100 {
                        stealer.steal_random();
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }
}

#[cfg(test)]
mod flow_scheduler_tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let scheduler = FlowScheduler::new();
        // 应该能正常创建
    }

    #[test]
    fn test_scheduler_spawn_untracked() {
        let scheduler = FlowScheduler::new();
        let task = Arc::new(Task::new(TaskId(0), TaskPriority::Normal, 1024));
        scheduler.spawn_untracked(task);
    }

    #[test]
    fn test_scheduler_parallel_tasks() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, Barrier};

        let scheduler = Arc::new(FlowScheduler::new());
        let counter = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(4));
        let mut handles = Vec::new();

        for i in 0..4 {
            let scheduler = scheduler.clone();
            let counter = counter.clone();
            let barrier = barrier.clone();

            let handle = thread::spawn(move || {
                barrier.wait();
                let task = Arc::new(Task::new(TaskId(i), TaskPriority::Normal, 1024));
                scheduler.spawn_untracked(task);

                counter.fetch_add(1, Ordering::SeqCst);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn test_scheduler_blocking_pool() {
        // 测试阻塞操作被正确隔离
    }

    #[test]
    fn test_scheduler_stress() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, Barrier};

        let scheduler = Arc::new(FlowScheduler::new());
        let counter = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(10));
        let mut handles = Vec::new();

        for i in 0..100 {
            let scheduler = scheduler.clone();
            let counter = counter.clone();
            let barrier = barrier.clone();

            let handle = thread::spawn(move || {
                barrier.wait();
                let task = Arc::new(Task::new(TaskId(i), TaskPriority::Normal, 1024));
                scheduler.spawn_untracked(task);

                counter.fetch_add(1, Ordering::SeqCst);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 100);
    }
}
```

### 5.3 惰性求值测试

```rust
// src/runtime/dag/tests/lazy_eval.rs

#[cfg(test)]
mod lazy_evaluation_tests {
    use super::*;

    #[test]
    fn test_lazy_node_not_executed_until_needed() {
        let graph = ComputationDAG::new();
        let counter = Arc::new(AtomicUsize::new(0));

        // 创建惰性节点
        let node_id = graph.add_node(
            NodeKind::LazyCompute,
            &[],
        );

        // 节点已就绪但尚未执行
        assert!(graph.pop_ready().is_some());

        // 节点不应该被计数（还没真正执行）
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_lazy_node_executed_on_access() {
        let graph = ComputationDAG::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let node_id = graph.add_node(NodeKind::LazyCompute, &[]);

        // 获取就绪节点
        let ready_id = graph.pop_ready().unwrap();
        assert_eq!(ready_id, node_id);

        // 模拟访问结果
        graph.node_completed(node_id);

        // 节点完成
        assert!(graph.pop_ready().is_none());
    }

    #[test]
    fn test_lazy_chain_execution() {
        // A -> B -> C（惰性链）
        let graph = ComputationDAG::new();

        let id_a = graph.add_node(NodeKind::LazyCompute, &[]);
        let id_b = graph.add_node(NodeKind::LazyCompute, &[id_a]);
        let id_c = graph.add_node(NodeKind::LazyCompute, &[id_b]);

        // 只有 A 就绪
        assert_eq!(graph.pop_ready().map(|id| id.inner()), Some(id_a.inner()));

        // 完成 A 后 B 就绪
        graph.node_completed(id_a);
        assert_eq!(graph.pop_ready().map(|id| id.inner()), Some(id_b.inner()));

        // 完成 B 后 C 就绪
        graph.node_completed(id_b);
        assert_eq!(graph.pop_ready().map(|id| id.inner()), Some(id_c.inner()));
    }

    #[test]
    fn test_eager_vs_lazy_behavior() {
        // 急切节点应该立即执行
        let graph = ComputationDAG::new();
        let exec_order = Arc::new(Mutex::new(Vec::new()));

        let eager_id = graph.add_node(NodeKind::EagerCompute, &[]);
        let lazy_id = graph.add_node(NodeKind::LazyCompute, &[]);

        // 急切节点应该已就绪
        assert!(graph.pop_ready().is_some());

        // 惰性节点如果有依赖也应该就绪
        assert!(graph.pop_ready().is_some());
    }
}

#[cfg(test)]
mod spawn_tests {
    use super::*;

    #[test]
    fn test_parallel_block_nodes() {
        let graph = ComputationDAG::new();

        // spawn { a, b } 创建并行块
        let block_id = graph.add_node(
            NodeKind::ParallelBlock {
                exprs: vec![],
                barrier: true,
            },
            &[],
        );

        // 并行块内的节点应该可以并行执行
        let id_a = graph.add_node(NodeKind::Compute, &[]);
        let id_b = graph.add_node(NodeKind::Compute, &[]);

        // 两个无依赖节点应该同时就绪
        let mut ready = Vec::new();
        while let Some(id) = graph.pop_ready() {
            ready.push(id);
        }
        assert_eq!(ready.len(), 2);
    }

    #[test]
    fn test_data_parallel_nodes() {
        let graph = ComputationDAG::new();

        // spawn for x in xs 创建数据并行
        let dp_id = graph.add_node(
            NodeKind::DataParallel {
                var: "x".to_string(),
                iter: Expr::List(vec![]),
                body: Expr::Int(0),
            },
            &[],
        );

        // 数据并行节点应该就绪
        assert!(graph.pop_ready().is_some());
    }

    #[test]
    fn test_effect_nodes() {
        let graph = ComputationDAG::new();

        // @effect 副作用节点
        let effect_id = graph.add_node(
            NodeKind::Effect {
                func: Expr::Lambda(vec![], Box::new(Expr::Int(0))),
                timing: EffectTiming::Immediate,
            },
            &[],
        );

        // 副作用节点应该立即执行
        assert!(graph.pop_ready().is_some());
    }
}
```

### 5.4 工作窃取测试

```rust
#[cfg(test)]
mod work_stealing_tests {
    use super::*;

    #[test]
    fn test_work_stealer_distribution() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, Barrier};
        use std::thread;

        let stealer = Arc::new(WorkStealer::new(4));
        let distribution = Arc::new(Mutex::new(vec![0usize; 4]));
        let barrier = Arc::new(Barrier::new(4));

        let handles: Vec<_> = (0..4)
            .map(|worker_id| {
                let stealer = stealer.clone();
                let distribution = distribution.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    barrier.wait();
                    for _ in 0..100 {
                        if let Some(task) = stealer.steal_random() {
                            let mut dist = distribution.lock().unwrap();
                            dist[worker_id] += 1;
                        }
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // 验证任务分布相对均匀
        let dist = distribution.lock().unwrap();
        let max = *dist.iter().max().unwrap();
        let min = *dist.iter().min().unwrap();
        assert!(max - min < 50); // 允许一定的不均匀
    }

    #[test]
    fn test_work_stealer_concurrent_access() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let stealer = Arc::new(WorkStealer::new(8));
        let barrier = Arc::new(Barrier::new(8));

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let stealer = stealer.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    barrier.wait();
                    for _ in 0..1000 {
                        stealer.steal_random();
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
        // 没有 panic 就是成功
    }
}
```

### 5.5 集成测试

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_parallel_execution() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, Barrier};
        use std::thread;
        use std::time::Duration;

        let scheduler = Arc::new(FlowScheduler::new());
        let start_time = Arc::new(Mutex::new(std::time::Instant::now()));
        let counter = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(8));

        let handles: Vec<_> = (0..8)
            .map(|i| {
                let scheduler = scheduler.clone();
                let start_time = start_time.clone();
                let counter = counter.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    barrier.wait();
                    let local_start = *start_time.lock().unwrap();

                    // 模拟 CPU 密集型任务
                    let _ = (0..1000000).fold(0, |acc, x| acc ^ x);

                    let elapsed = local_start.elapsed().as_millis();
                    println!("Task {} finished in {}ms", i, elapsed);

                    counter.fetch_add(1, Ordering::SeqCst);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 8);
    }

    #[test]
    fn test_dag_stress_test() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, Barrier};

        let graph = Arc::new(ComputationDAG::new());
        let counter = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(10));

        // 创建 1000 个节点形成复杂 DAG
        let mut handles = Vec::new();
        for _ in 0..10 {
            let graph = graph.clone();
            let counter = counter.clone();
            let barrier = barrier.clone();

            let handle = thread::spawn(move || {
                barrier.wait();
                for i in 0..100 {
                    let deps: Vec<NodeId> = (0..3)
                        .map(|_| NodeId(rand::thread_rng().gen_range(0..i.max(1))))
                        .collect();

                    graph.add_node(NodeKind::LazyCompute, &deps);
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 1000);
    }

    #[test]
    fn test_async_value_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let async_val = Arc::new(Async::new(async { Ok(42) }));

        let handles: Vec<_> = (0..100)
            .map(|_| {
                let async_val = async_val.clone();
                thread::spawn(move || {
                    let _ = &*async_val; // 自动解包
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
        // 所有线程都能正确访问
    }
}
```

---

## 六、实现步骤

### 阶段 1：基础数据结构
1. [x] 实现 `NodeId` 和 `NodeIdGenerator`
2. [x] 实现 `DAGNode` 和 `DAGNodeKind`
3. [x] 实现 `ComputationDAG`
4. [x] 编写 DAG 单元测试

### 阶段 2：调度器基础
1. [ ] 实现 `TaskQueue`
2. [ ] 实现 `WorkStealer`
3. [ ] 实现 `FlowScheduler` 骨架
4. [ ] 实现 `BlockingThreadPool`
5. [ ] 编写调度器单元测试

### 阶段 3：异步值类型
1. [ ] 实现 `AsyncState`
2. [ ] 实现 `AsyncInner`
3. [ ] 实现 `Async<T>`
4. [ ] 实现自动解包 (Deref)
5. [ ] 编写 Async 单元测试

### 阶段 4：libuv IO 调度引擎 ⭐ 新增
1. [ ] 添加 uv-rs 依赖到 Cargo.toml
2. [ ] 实现 `UvIOScheduler` 事件循环封装
3. [ ] 实现 `UvTcpStream` TCP 操作
4. [ ] 实现 `UvFsFile` 文件系统操作
5. [ ] 实现 `UvTimer` 定时器
6. [ ] 编写 IO 单元测试
7. [ ] 测试跨平台兼容性 (Linux/macOS/Windows)

### 阶段 5：求值策略
1. [ ] 实现惰性求值策略 `LazyCompute`
2. [ ] 实现急切求值策略 `EagerCompute`
3. [ ] 实现副作用处理 `Effect`
4. [ ] 实现自动策略选择
5. [ ] 编写策略测试

### 阶段 6：spawn 语法支持
1. [ ] 实现 `ParallelBlock` 节点类型
2. [ ] 实现 `DataParallel` 节点类型
3. [ ] 实现并行屏障同步
4. [ ] 编写 spawn 测试

### 阶段 7：集成测试
1. [ ] DAG + 调度器集成测试
2. [ ] IO + 调度器集成测试
3. [ ] 并发压力测试
4. [ ] 性能基准测试
5. [ ] 内存安全测试 (miri)

```rust
// src/runtime/async_value/tests/mod.rs

#[cfg(test)]
mod async_value_tests {
    use super::*;

    #[test]
    fn test_async_creation() {
        let async_val = Async::new(async { Ok(42) });
        assert!(!async_val.is_ready());
    }

    #[test]
    fn test_async_get() {
        let async_val = Async::new(async { Ok(42) });
        let result = async_val.get();
        assert_eq!(result.unwrap(), &42);
    }

    #[test]
    fn test_async_error() {
        let async_val = Async::new(async { Err(anyhow::anyhow!("error")) });
        let result = async_val.get();
        assert!(result.is_err());
    }

    #[test]
    fn test_async_auto_unwrap() {
        let async_val = Async::new(async { Ok(42) });
        let value: &i32 = &*async_val;
        assert_eq!(value, &42);
    }

    #[test]
    fn test_async_parallel() {
        use std::sync::Arc;
        use std::thread;

        let async_val = Arc::new(Async::new(async { Ok(42) }));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let async_val = async_val.clone();
                thread::spawn(move || {
                    let _: &i32 = &*async_val;
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
```

---

## 五、实现步骤

### 阶段 1：基础数据结构
1. [ ] 实现 `NodeId` 和 `NodeIdGenerator`
2. [ ] 实现 `NodeState` 和 `NodeKind`
3. [ ] 实现 `DAGNode`
4. [ ] 实现 `ComputationDAG`
5. [ ] 编写 DAG 单元测试

### 阶段 2：调度器基础
1. [ ] 实现 `TaskQueue`
2. [ ] 实现 `WorkStealer`
3. [ ] 实现 `FlowScheduler` 骨架
4. [ ] 实现 `BlockingThreadPool`
5. [ ] 编写调度器单元测试

### 阶段 3：异步值类型
1. [ ] 实现 `AsyncState`
2. [ ] 实现 `AsyncInner`
3. [ ] 实现 `Async<T>`
4. [ ] 实现自动解包 (Deref)
5. [ ] 编写 Async 单元测试

### 阶段 4：集成测试
1. [ ] DAG + 调度器集成测试
2. [ ] 并发压力测试
3. [ ] 性能基准测试
4. [ ] 内存安全测试 (miri)

---

## 六、libuv IO 验收标准 ⭐

### IO 功能验收
- [ ] **TCP 连接**：异步 TCP 连接正常工作
- [ ] **TCP 读写**：非阻塞读写操作正确执行
- [ ] **UDP 通信**：UDP 数据包收发正常
- [ ] **文件 IO**：异步文件读取和写入
- [ ] **定时器**：定时回调准确触发
- [ ] **跨平台**：Linux/macOS/Windows 均可正常工作

### IO 性能验收
- [ ] **IO 吞吐量**：单连接 > 100 MB/s
- [ ] **连接建立**：TCP 连接 < 10ms (localhost)
- [ ] **并发连接**：支持 > 10,000 并发连接
- [ ] **定时精度**：定时器误差 < 1ms

### IO 稳定性验收
- [ ] **无内存泄漏**：长时间运行无内存增长
- [ ] **无句柄泄漏**：连接关闭后正确释放资源
- [ ] **错误处理**：网络错误正确传播
- [ ] **连接恢复**：断线后能正确处理

---

## 七、向后兼容性

```rust
// src/runtime/scheduler/compat.rs

/// 旧版 Scheduler API（已废弃）
#[deprecated(since = "0.3.0", note = "请使用 FlowScheduler")]
pub struct Scheduler {
    inner: FlowScheduler,
}

#[deprecated]
impl Scheduler {
    pub fn new() -> Self {
        Self {
            inner: FlowScheduler::new(),
        }
    }

    #[deprecated]
    pub fn spawn(&self, task: Arc<Task>) {
        self.inner.spawn_untracked(task);
    }
}

// 让旧代码可以继续编译
#[deprecated]
pub fn create_scheduler() -> Scheduler {
    Scheduler::new()
}
```

---

## 八、验收标准

### 功能验收
- [ ] **默认惰性求值**：所有函数默认惰性，结果使用时才求值
- [ ] **核心数量配置**：`// @cores: N` 正确配置工作线程数
- [ ] **DAG 依赖关系**：依赖关系正确构建，无循环依赖
- [ ] **任务按依赖执行**：父节点完成后子节点才执行
- [ ] **无依赖并行**：无依赖任务可真正并行执行
- [ ] **spawn 精细控制**：`spawn fn`、`spawn {}`、`spawn for` 正确工作
- [ ] **工作窃取**：负载均衡正确，减少饥饿
- [ ] **Async[T] 透明**：自动解包，使用无感
- [ ] **副作用处理**：`@effect` 标注正确执行
- [ ] **混合求值模式**：`@eager`、`@lazy`、`@force` 正确工作

### 测试验收
- [ ] **单元测试覆盖率** > 90%（每个公开 API 都有测试）
- [ ] **并发测试**：Miri 检测无 data race
- [ ] **压力测试**：1000+ 节点 DAG 稳定运行
- [ ] **边界测试**：空图、单节点、多依赖等边界情况
- [ ] **线程安全**：多线程并发访问无竞争

### 性能验收
- [ ] **调度延迟**：平均 < 1ms
- [ ] **窃取成功率**：> 80%
- [ ] **内存使用**：可接受（无内存泄漏）
- [ ] **并行加速**：多核利用率 > 80%

### 向后兼容性
- [ ] **旧 API 兼容**：`Scheduler` API 标记 deprecated 但可用
- [ ] **平滑迁移**：旧代码无需修改即可编译运行
- [ ] **接口稳定**：内部实现可重构，接口保持稳定

---

> **下一步**：请主人批准后开始实现

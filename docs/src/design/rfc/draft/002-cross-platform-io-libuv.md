---
title: RFC-002：跨平台 I/O 与 libuv 集成
---

# RFC-002：跨平台I/O与libuv集成

> **状态**: 草案
> **作者**: 晨煦
> **创建日期**: 2025-01-05
> **最后更新**: 2026-02-12

## 摘要

提出YaoXiang的跨平台异步I/O方案，集成libuv实现统一的异步抽象。核心设计目标是让阻塞I/O操作自动透明地异步化，开发者无需关心底层细节。

## 动机

### 为什么需要libuv？

YaoXiang的并作模型需要高效的异步I/O支持：

| 需求     | 传统方案的问题                                          |
| ------ | ------------------------------------------------ |
| 跨平台I/O | 各平台API不统一（Windows IOCP、Linux epoll、macOS kqueue） |
| 异步事件循环 | 从零实现复杂且易出错                                       |
| 线程池管理  | 阻塞操作需要专用线程池                                      |
| 性能要求   | 需要零开销的异步抽象                                       |

### libuv的优势

```
libuv ✓ 成熟稳定 - Node.js底层运行时，经过大规模验证
libuv ✓ 跨平台 - 统一Windows、Linux、macOS的I/O API
libuv ✓ 高性能 - 事件驱动、非阻塞I/O
libuv ✓ 线程池 - 内置阻塞操作线程池管理
```

## 提案

### 1. 技术选型决策

| 组件 | 选型 | 理由 |
|------|------|------|
| I/O运行时 | libuv | 跨平台成熟、Node.js验证 |
| 事件循环 | libuv loop | 轻量级、高效 |
| 线程池 | libuv + 自定义 | 阻塞操作专用 |
| 调度算法 | 工作窃取 + DAG优化 | 高性能、负载均衡 |
| 内存管理 | 所有权 + 栈分配 | 无GC、零成本抽象 |

### 2. 架构设计

#### 2.1 运行时整体结构

```
┌─────────────────────────────────────────────────────────────┐
│                    YaoXiang Runtime                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              WorkStealingScheduler                  │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐              │   │
│  │  │Worker 0 │ │Worker 1 │ │Worker 2 │ ...         │   │
│  │  │   DAG   │ │   DAG   │ │   DAG   │              │   │
│  │  │ Executor│ │ Executor│ │ Executor│              │   │
│  │  └────┬────┘ └────┬────┘ └────┬────┘              │   │
│  └───────┼───────────┼───────────┼────────────────────┘   │
│          │           │           │                        │
│          └───────────┴───────────┘                        │
│                      │                                    │
│          ┌───────────▼───────────┐                        │
│          │   libuv Event Loop    │                        │
│          │   (跨平台I/O抽象)      │                        │
│          └───────────┬───────────┘                        │
│                      │                                    │
│          ┌───────────▼───────────┐                        │
│          │   Thread Pool         │  ← 阻塞操作专用        │
│          │   (libuv threads)     │                        │
│          └───────────────────────┘                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 2.2 运行时结构定义

```rust
struct YaoXiangRuntime {
    // libuv事件循环（跨平台I/O核心）
    uv_loop: *mut uv_loop_t,

    // 工作窃取调度器
    scheduler: WorkStealingScheduler,

    // 阻塞操作线程池
    io_thread_pool: ThreadPool,

    // 任务队列
    task_queues: Vec<Deque<Task>>,

    // 统计信息
    stats: RuntimeStats,
}

struct WorkStealingScheduler {
    workers: Vec<WorkerThread>,
    global_queue: ConcurrentDeque<Task>,
    victim_steal_attempts: AtomicUsize,
}

struct ThreadPool {
    size: usize,
    sender: Sender<Task>,
    receiver: Receiver<Task>,
}
```

### 3. 统一异步抽象

#### 3.1 阻塞到透明的转换

```
┌─────────────────────────────────────────────────────────────┐
│  阻塞C函数  →  自动包装  →  透明Async[T]                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  // 原始阻塞API                                             │
│  data = File.read("file.txt")  // 阻塞调用                  │
│                                                             │
│  // YaoXiang自动转换                                        │
│  // 1. 检测到阻塞调用                                       │
│  // 2. 自动提交到线程池                                     │
│  // 3. 返回 Async[T] 代理                                   │
│  // 4. 使用时自动等待结果                                   │
│                                                             │
│  // 开发者视角                                              │
│  content = File.read("config.yaml")  // Async[String]       │
│  data = parse(content)               // 自动等待            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 3.2 I/O操作示例

```yaoxiang
# 异步文件读取（开发者视角：同步语法，自动异步）
read_config: (String) -> Config spawn = (path) => {
    content = File.read(path)  # 自动异步化
    config = parse_yaml(content)
    config
}

# 异步网络请求
fetch_user: (Int) -> User spawn = (user_id) => {
    response = HTTP.get("/users/" + user_id.to_string())
    parse_user(response.body())
}

# 并发文件处理
process_files: ([String]) -> [Result[FileData, Error]] = (paths) => {
    # 自动并行读取所有文件
    data = paths.map(path => {
        File.read(path)  # spawn自动插入
    })
    data.map(d => process_content(d))
}

# 流式处理（逐步读取）
stream_large_file: (String) -> Void = (path) => {
    stream = File.open_stream(path)
    for chunk in stream.chunks(8192) {  # 自动异步迭代
        process(chunk)
    }
}
```

#### 3.3 网络I/O

```yaoxiang
# HTTP服务器
router: (HTTPRequest) -> HTTPResponse = (req) => {
    match req.path {
        "/" => home_page()
        "/api/users" => list_users()
        "/api/posts" => list_posts()
        _ => not_found()
    }
}

start_server: (Int) -> Void spawn = (port) => {
    server = HTTP.Server.new(port)
    server.serve(router)  # 自动处理并发请求
}

# WebSocket
chat_server: (String) -> Void spawn = (port) => {
    ws = WebSocket.new("ws://localhost:" + port.to_string())
    for message in ws.incoming() {  # 自动流式处理
        broadcast(message)
    }
}
```

### 4. 跨平台保证

#### 4.1 平台支持矩阵

| 平台          | 状态    | 事件机制   | 备注        |
| ----------- | ----- | ------ | --------- |
| **Linux**   | ✅ 支持  | epoll  | 主要开发平台    |
| **macOS**   | ✅ 支持  | kqueue | libuv原生支持 |
| **Windows** | ✅ 支持  | IOCP   | libuv原生支持 |
| **WASM**    | ⚠️ 待定 | 浏览器API | 需要额外适配    |
| **WASI**    | ⚠️ 待定 | WASI调用 | 长期目标      |

#### 4.2 跨平台API统一

```yaoxiang
# 文件I/O - 统一API
file_api: () -> Void = () => {
    # 所有平台相同API
    content = File.read("data.txt")      # 读取
    File.write("output.txt", content)    # 写入
    exists = File.exists("data.txt")     # 检查
    File.delete("temp.txt")              # 删除
}

# 网络I/O - 统一API
network_api: () -> Void = () => {
    socket = Net.Socket.new(Net.IP.v4(127, 0, 0, 1), 8080)
    socket.connect()
    socket.send("Hello")
    response = socket.recv()
    socket.close()
}

# 进程I/O - 统一API
process_api: () -> Void = () => {
    output = Process.run("ls", ["-la"])  # 跨平台执行
    print(output.stdout)
}
```

#### 4.3 平台特定优化

```yaoxiang
# Windows特定优化
when os() == "windows" {
    use_windows_registry()
}

# Linux特定优化
when os() == "linux" {
    use_inotify()
}

# macOS特定优化
when os() == "macos" {
    use_fsevents()
}
```

### 5. 性能考量

#### 5.1 线程池配置

```yaoxiang
# 脚本头配置线程池大小
# @thread_pool: 4

# 或运行时配置
configure_runtime: () -> Void = () => {
    Runtime.set_thread_pool_size(8)
    Runtime.set_max_concurrent_tasks(100)
}
```

#### 5.2 I/O批量优化

```yaoxiang
# 批量文件操作（减少系统调用）
batch_read: ([String]) -> [String] = (paths) => {
    # libuv批量提交，减少上下文切换
    File.batch_read(paths)
}

# 零拷贝优化
zero_copy_transfer: (String, String) -> Void = (src, dst) => {
    # 在支持的平台使用sendfile/ splice
    File.transfer(src, dst)
}
```

## 详细设计

### 1. Rust绑定结构

```rust
// libuv绑定模块
pub mod uv {
    use std::ffi::c_void;
    use std::ptr::null_mut;

    // 基础类型
    pub struct UvLoop(uv_loop_t);

    // 文件操作
    pub trait FileOps {
        fn fs_open(path: &str, flags: i32, mode: i32) -> Result<RawFd, Errno>;
        fn fs_read(fd: RawFd, buf: &mut [u8], offset: i64) -> Result<usize, Errno>;
        fn fs_write(fd: RawFd, buf: &[u8], offset: i64) -> Result<usize, Errno>;
        fn fs_close(fd: RawFd) -> Result<(), Errno>;
    }

    // 网络操作
    pub trait NetOps {
        fn tcp_new() -> Result<RawTcpSocket, Errno>;
        fn tcp_connect(socket: RawTcpSocket, addr: &SocketAddr) -> Result<(), Errno>;
        fn tcp_read(socket: RawTcpSocket, buf: &mut [u8]) -> Result<usize, Errno>;
        fn tcp_write(socket: RawTcpSocket, buf: &[u8]) -> Result<usize, Errno>;
    }

    // 线程池
    pub struct ThreadPool {
        size: usize,
        queue: Channel<Task>,
    }
}
```

### 2. 调度器设计

```rust
// 工作窃取调度器
pub struct WorkStealingScheduler {
    workers: Vec<Worker>,
    global_queue: ConcurrentDeque<Task>,
    victim_queue: AtomicUsize,
}

impl WorkStealingScheduler {
    pub fn schedule(&self, task: Task) {
        // 优先加入本地队列
        if let Ok(worker) = self.current_worker() {
            worker.local_queue.push_back(task);
        } else {
            // 无工作者时加入全局队列
            self.global_queue.push_back(task);
        }
    }

    pub fn steal(&self, victim: &Worker) -> Option<Task> {
        // 从其他工作者的队列窃取任务
        victim.local_queue.pop_back()
    }
}
```

### 3. 异步任务生命周期

```
┌─────────────────────────────────────────────────────────────┐
│  Task Lifecycle                                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────┐   ┌─────────────┐   ┌─────────┐              │
│  │ Created │ → │ Scheduled   │ → │ Running │              │
│  └─────────┘   └─────────────┘   └────┬────┘              │
│                                       │                    │
│                      ┌────────────────┴────────────────┐   │
│                      ▼                                 ▼   │
│               ┌───────────┐                    ┌───────────┐│
│               │ Completed │                    │  Failed   ││
│               └───────────┘                    └───────────┘│
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 4. 错误处理集成

```rust
// I/O错误传播
#[derive(Debug)]
pub enum IoError {
    FileNotFound(String),
    PermissionDenied(String),
    IoErrno(i32, String),
    Cancelled,
}

impl From<uv::UvError> for IoError {
    fn from(err: uv::UvError) -> Self {
        match err.code() {
            uv::ENOENT => IoError::FileNotFound(err.path()),
            uv::EACCES => IoError::PermissionDenied(err.path()),
            _ => IoError::IoErrno(err.code(), err.message()),
        }
    }
}
```

## 权衡

### 优点

1. **跨平台一致**：同一套API覆盖所有主流平台
2. **高性能**：事件驱动+工作窃取，接近手写异步性能
3. **透明异步**：开发者无需手动处理异步细节
4. **阻塞安全**：阻塞操作自动进入线程池，不阻塞事件循环
5. **成熟稳定**：libuv经过Node.js大规模验证

### 缺点

1. **依赖引入**：需要绑定libuv C库
2. **Windows兼容性**：某些API在Windows行为略有差异
3. **WASM支持**：需要额外适配工作
4. **调试困难**：异步栈追踪可能不完整

## 替代方案

| 方案 | 为什么不选择 |
|------|--------------|
| 从零实现事件循环 | 复杂且易出错，无法与libuv成熟度相比 |
| 使用 mio | 仅提供原始异步原语，缺乏线程池 |
| 使用 async-std/tokio | Rust生态，但YaoXiang需要自己的运行时 |
| 直接使用libc epoll | 无法跨平台 |

## 实现策略

### 阶段划分

1. **阶段1 (v0.1)**: 基础libuv绑定、简单文件I/O
2. **阶段2 (v0.3)**: 网络I/O、线程池集成
3. **阶段3 (v0.5)**: 高级特性、流式API
4. **阶段4 (v1.0)**: WASM适配、性能优化

### 依赖关系

- 无外部 RFC 依赖
- **RFC-001 并发模型**：定义 DAG 调度器，RFC-002 提供 IO 抽象

## 与 RFC-001 并发模型的集成

RFC-001 定义了 **DAG 调度器**（调度层），RFC-002 定义了 **libuv + 线程池**（IO 层）。两者协作实现"同步语法，自动并发"。

### 分层架构

```
┌─────────────────────────────────────────────────────────────┐
│                    YaoXiang Runtime                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐    ┌─────────────────────┐        │
│  │   RFC-001: DAG      │    │  RFC-002: libuv     │        │
│  │   调度层            │    │  IO 层              │        │
│  │                     │    │                     │        │
│  │  • 拓扑排序调度     │    │  • 跨平台 I/O       │        │
│  │  • 工作窃取         │    │  • 事件循环         │        │
│  │  • 依赖分析         │    │  • 线程池           │        │
│  └──────────┬──────────┘    └──────────┬──────────┘        │
│             │                         │                    │
│             │     ┌───────────────────┘                    │
│             │     │                                         │
│             ▼     ▼                                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Runtime 接口层                          │   │
│  │  • spawn/suspend/resume 协议                         │   │
│  │  • IO Completion 回调                                │   │
│  │  • 任务提交与唤醒                                    │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 协作流程

```markdown
1. **编译期**：资源类型操作被识别为 IO 节点
   - File.read, HTTP.get 等被标记为 "需要异步执行"
   - 创建 DAG 节点，标记为 IO 类型

2. **运行期**：DAG 调度器遇到 IO 节点
   - 识别为非计算节点，提交到 libuv
   - 调度器继续执行其他可执行节点

3. **IO 完成**：libuv 回调触发
   - libuv 线程池完成阻塞操作
   - completion 回调通知 DAG 调度器
   - 下游节点变为可执行
```

### 接口协议

```rust
// RFC-001 定义的 IO 节点接口
trait IoScheduler {
    // 提交 IO 任务，返回 future/handle
    fn submit_io(&self, task: IoTask) -> IoHandle;

    // IO 完成时由 libuv 调用，唤醒 DAG 节点
    fn on_io_complete(&self, handle: IoHandle);
}

// RFC-002 实现的 libuv 集成
impl IoScheduler for LibUvRuntime {
    fn submit_io(&self, task: IoTask) -> IoHandle {
        // 1. 将任务提交到 libuv 线程池
        let handle = self.thread_pool.submit(|| {
            // 阻塞执行实际 IO
            let result = perform_blocking_io(&task);
            // 2. IO 完成，调用回调
            self.on_io_complete(handle);
        });
        handle
    }

    fn on_io_complete(&self, handle: IoHandle) {
        // 通知 DAG 调度器唤醒下游节点
        self.dag_scheduler.wake_dependents(handle.node_id);
    }
}
```

### 透明异步机制

#### 编译期处理

```yaoxiang
# 用户代码（同步语法）
read_config: String -> Config = (path) => {
    content = File.read(path)  # 资源操作
    parse_yaml(content)
}

# 编译期自动转换
# 1. 识别 File.read 为资源类型操作
# 2. 创建 DAG 节点，标记为 IO 类型
# 3. 添加隐式 await 点
```

#### 运行时处理

```markdown
| 步骤 | 操作 | 说明 |
|------|------|------|
| 1 | 解析 DAG | 发现 IO 节点 |
| 2 | 提交 IO | 将任务加入 libuv 线程池 |
| 3 | 继续调度 | 执行其他可执行节点 |
| 4 | IO 完成 | libuv 回调触发 |
| 5 | 唤醒下游 | DAG 调度器 resume 等待的节点 |
```

### 资源类型与 IO 操作的映射

```yaoxiang
# RFC-001 定义：资源类型
FilePath: Resource
HttpUrl: Resource

# RFC-002 实现：资源操作的 IO 语义
File.read: (FilePath) -> String = path => {
    # 标记为 IO 操作，自动进入 libuv 线程池
}

HTTP.get: (HttpUrl) -> Response = url => {
    # 标记为 IO 操作，使用 libuv 异步网络 API
}
```

**处理规则**：
- 资源类型参数的操作 → 标记为 IO 节点
- IO 节点提交到 libuv 线程池执行
- completion 回调唤醒 DAG 下游节点

### 风险

1. **libuv绑定完整性**：完整绑定需要大量工作
2. **Windows兼容性**：某些API需要特殊处理
3. **性能开销**：FFI调用有一定开销
4. **集成复杂度**：libuv 线程池与 DAG 调度器的协调需要仔细设计

## 开放问题

- [ ] WASM环境下的事件循环适配方案
- [ ] 文件系统事件的跨平台一致性
- [ ] 网络I/O的超时机制设计
- [ ] 零拷贝优化的边界
- [ ] 取消操作的语义设计
- [ ] libuv 线程池大小动态调整策略
- [ ] IO 节点优先级与计算节点优先级的协调

## 参考文献

- [libuv官方文档](https://docs.libuv.org/)
- [Node.js事件循环](https://nodejs.org/en/docs/guides/event-loop-timers-and-nexttick/)
- [工作窃取论文](https://ieftimov.com/posts/understanding-stealing-queues/)
- [Rust异步运行时设计](https://smallcultfollowing.com/babysteps/blog/2019/08/22/async-await-simplified/)

---
title: "RFC-002：基于 libuv 的资源类型 IO 实现层"
status: "草案"
author: "晨煦"
created: "2025-01-05"
updated: "2026-07-05"
issue: "#102"


# RFC-002：基于 libuv 的资源类型 IO 实现层

> **参考**:
> - [RFC-024: 基于 spawn 块的并发模型](./024-concurrency-model.md)
> - [RFC-008: Runtime 并发模型与调度器脱耦设计](./008-runtime-concurrency-model.md)
> - [RFC-009: 所有权模型设计](./009-ownership-model.md)
> - [并发模型规范](/reference/language-spec/concurrency.md)

## 摘要

本文档定义 YaoXiang 的 IO 实现层：基于 libuv 提供跨平台 IO 能力，作为 RFC-024 资源类型系统的底层实现。

**核心定位**：

```
RFC-024：资源类型定义（FilePath, HttpUrl, DBUrl, Console）
    ↓ 使用
RFC-002：资源类型 IO 实现（基于 libuv）
    ↓ 底层
libuv：跨平台 IO 引擎（事件循环 + 线程池）
```

**不是什么**：
- ❌ 不是"透明异步"——用户通过 spawn 块显式控制并发
- ❌ 不是"自动异步化"——IO 操作需要在 spawn 块内显式调用
- ❌ 不是"开发者无需关心底层细节"——资源类型系统确保并发安全

**是什么**：
- ✅ 资源类型（FilePath, HttpUrl, DBUrl, Console）的 IO 实现层
- ✅ 跨平台 IO 统一（libuv 处理 Windows/Linux/macOS 差异）
- ✅ 共享事件循环架构（一个 libuv 事件循环处理所有 IO）
- ✅ 与 RFC-024 资源类型系统的集成

## 动机

### 为什么需要 libuv？

RFC-024 定义了资源类型系统：
- `FilePath` - 文件系统路径
- `HttpUrl` - HTTP 端点
- `DBUrl` - 数据库连接
- `Console` - 标准输出

这些资源类型需要底层 IO 实现。libuv 提供：

| 需求 | libuv 提供 |
|------|-----------|
| 跨平台 IO | 统一 Windows/Linux/macOS API |
| 异步能力 | 共享事件循环，所有 worker 的 IO 集中处理 |
| 线程池 | 阻塞操作专用线程池 |
| 并发安全 | 单线程事件循环，天然无竞争 |

### 与 RFC-024 的关系

```
┌─────────────────────────────────────────────────────────┐
│  RFC-024：并发模型                                       │
│  - spawn {} 块（显式并发）                               │
│  - 资源类型定义（FilePath, HttpUrl, DBUrl, Console）     │
│  - 资源冲突检测（同路径自动串行）                         │
└─────────────────────────────────────────────────────────┘
                          ↓ 使用
┌─────────────────────────────────────────────────────────┐
│  RFC-002：资源类型 IO 实现                               │
│  - FilePath → libuv 文件 IO                              │
│  - HttpUrl → libuv 网络 IO                               │
│  - DBUrl → 数据库连接池                                  │
│  - Console → 标准输出串行化                              │
└─────────────────────────────────────────────────────────┘
                          ↓ 底层
┌─────────────────────────────────────────────────────────┐
│  libuv：跨平台 IO 引擎                                   │
│  - 事件循环                                              │
│  - 线程池                                                │
│  - 跨平台统一 API                                        │
└─────────────────────────────────────────────────────────┘
```

---

## 提案

### 1. libuv 架构

#### 1.1 共享事件循环架构

```
┌─────────────────────────────────────────────────────────┐
│                    Runtime                               │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │  Worker 0   │  │  Worker 1   │  │  Worker N   │    │
│  │  计算任务    │  │  计算任务    │  │  计算任务    │    │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘    │
│         │                │                │            │
│         └────────────────┼────────────────┘            │
│                          ↓                              │
│  ┌─────────────────────────────────────────────────┐  │
│  │          libuv 事件循环（专用线程）               │  │
│  │          处理所有 IO 操作                         │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**关键特性**：
- 一个共享的 libuv 事件循环（在专用线程上运行）
- 所有 worker 的 IO 操作提交到这个共享事件循环
- 单线程事件循环天然避免竞争
- 资源效率高，不需要为每个 worker 创建事件循环

#### 1.2 并发安全机制

| libuv 特性 | YaoXiang 对应 | 并发安全 |
|------------|---------------|----------|
| 单线程事件循环 | spawn 块内顺序执行 | 天然无竞争 |
| 线程池隔离 | 阻塞操作不阻塞主线程 | 无共享状态 |
| 异步回调 | DAG 调度器管理依赖 | 确定性执行 |

### 2. 资源类型 IO 映射

#### 2.1 FilePath → libuv 文件 IO

```rust
// std.io 模块（基于 libuv）
pub struct IoModule;

impl StdModule for IoModule {
    fn exports(&self) -> Vec<NativeExport> {
        vec![
            // 文件操作 → libuv fs_* API
            NativeExport::new("read_file", "std.io.read_file", 
                "(path: FilePath) -> String", native_read_file),
            NativeExport::new("write_file", "std.io.write_file", 
                "(path: FilePath, content: String) -> Bool", native_write_file),
            NativeExport::new("append_file", "std.io.append_file", 
                "(path: FilePath, content: String) -> Bool", native_append_file),
            // Console 操作 → libuv tty API
            NativeExport::new("print", "std.io.print", 
                "(...args) -> ()", native_print),
            NativeExport::new("println", "std.io.println", 
                "(...args) -> ()", native_println),
        ]
    }
}

// libuv 文件 IO 实现
fn native_read_file(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let path = extract_file_path(args)?;
    
    // 提交到 libuv 事件循环
    // libuv 异步读取文件
    // 返回结果
    ctx.uv_loop.fs_read(path)
}
```

#### 2.2 HttpUrl → libuv 网络 IO

```rust
// std.net 模块（基于 libuv）
pub struct NetModule;

impl StdModule for NetModule {
    fn exports(&self) -> Vec<NativeExport> {
        vec![
            // HTTP 操作 → libuv http API
            NativeExport::new("http_get", "std.net.http_get", 
                "(url: HttpUrl) -> Response", native_http_get),
            NativeExport::new("http_post", "std.net.http_post", 
                "(url: HttpUrl, body: String) -> Response", native_http_post),
        ]
    }
}

// libuv 网络 IO 实现
fn native_http_get(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let url = extract_http_url(args)?;
    
    // 提交到 libuv 事件循环
    // libuv 异步 HTTP 请求
    // 返回结果
    ctx.uv_loop.http_get(url)
}
```

#### 2.3 DBUrl → 数据库连接池

```rust
// std.db 模块（基于 libuv）
pub struct DbModule;

impl StdModule for DbModule {
    fn exports(&self) -> Vec<NativeExport> {
        vec![
            // 数据库操作 → libuv 线程池
            NativeExport::new("query", "std.db.query", 
                "(url: DBUrl, sql: String) -> Rows", native_query),
        ]
    }
}

// libuv 数据库 IO 实现
fn native_query(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let url = extract_db_url(args)?;
    let sql = extract_sql(args)?;
    
    // 提交到 libuv 线程池
    // 数据库查询在线程池执行
    // 完成后回调通知主线程
    ctx.uv_loop.db_query(url, sql)
}
```

#### 2.4 Console → 标准输出串行化

```rust
// Console 操作自动串行化（RFC-024 资源类型规则）
// 所有 Console 操作在同一线程内顺序执行
fn native_print(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let output = format_args(args);
    
    // Console 操作串行化
    // libuv tty 写入
    ctx.uv_loop.tty_write(output)
}
```

### 3. 与 spawn 块的集成

#### 3.1 用户视角

```yaoxiang
# 资源类型定义（RFC-024）
FilePath: Resource
HttpUrl: Resource

# IO 操作（RFC-002 实现）
File.read: (FilePath) -> String
HTTP.get: (HttpUrl) -> Response

# 用户显式并发（RFC-024）
(a, b) = spawn {
    read_file("data.txt"),      # 资源类型 FilePath，底层 libuv
    fetch("http://example.com") # 资源类型 HttpUrl，底层 libuv
}
# 编译器：FilePath 和 HttpUrl 无冲突，可以并行
```

#### 3.2 编译期分析

```
编译器分析 spawn 块：
1. 识别资源类型操作
2. 检测资源冲突（同路径/同 URL 自动串行）
3. 生成 DAG 执行计划
4. 标记 IO 节点（提交到 libuv）
```

#### 3.3 运行时执行

```
运行时执行 spawn 块：
1. Worker 0 提交 IO 任务 → 共享事件循环
2. Worker 1 提交 IO 任务 → 共享事件循环
3. 事件循环统一处理所有 IO 操作
4. IO 完成后通知对应的 Worker
5. Worker 继续执行后续任务
```

### 4. Runtime 三层架构与 libuv

| 层级 | libuv 使用 | 异步能力 | 适用场景 |
|------|-----------|----------|----------|
| Embedded Runtime | 无 libuv | 无异步 | WASM、游戏脚本 |
| Standard Runtime | 共享事件循环 | IO 异步 | Web 服务、数据管道 |
| Full Runtime | 共享事件循环 | IO 异步 + 并行 | 科学计算、大规模并行 |

**Embedded Runtime**：无 libuv，即时执行，无异步能力。

**Standard Runtime**：共享 libuv 事件循环，所有 IO 操作异步处理。

**Full Runtime**：共享 libuv 事件循环，多线程并行 + IO 异步。

---

## 详细设计

### 1. Rust 绑定结构

```rust
// libuv 绑定模块
pub mod uv {
    // 事件循环
    pub struct UvLoop {
        loop_handle: *mut uv_loop_t,
    }
    
    // 文件操作
    pub trait FileOps {
        fn fs_read(&self, path: &str) -> Result<String, UvError>;
        fn fs_write(&self, path: &str, content: &str) -> Result<(), UvError>;
        fn fs_append(&self, path: &str, content: &str) -> Result<(), UvError>;
    }
    
    // 网络操作
    pub trait NetOps {
        fn http_get(&self, url: &str) -> Result<Response, UvError>;
        fn http_post(&self, url: &str, body: &str) -> Result<Response, UvError>;
    }
    
    // 数据库操作
    pub trait DbOps {
        fn db_query(&self, url: &str, sql: &str) -> Result<Rows, UvError>;
    }
    
    // Console 操作
    pub trait ConsoleOps {
        fn tty_write(&self, data: &str) -> Result<(), UvError>;
    }
}
```

### 2. 标准库模块结构

```
src/std/
├── io.rs          # FilePath IO（基于 libuv）
├── net.rs         # HttpUrl IO（基于 libuv）
├── db.rs          # DBUrl IO（基于 libuv）
├── console.rs     # Console IO（基于 libuv）
└── mod.rs         # 模块注册
```

### 3. 与 DAG 调度器的集成

```rust
// IO 节点接口（RFC-008 定义）
trait IoScheduler {
    // 提交 IO 任务，返回句柄
    fn submit_io(&self, task: IoTask) -> IoHandle;
    
    // IO 完成时由 libuv 调用，唤醒 DAG 节点
    fn on_io_complete(&self, handle: IoHandle);
}

// libuv 实现
impl IoScheduler for UvLoop {
    fn submit_io(&self, task: IoTask) -> IoHandle {
        match task.resource_type {
            ResourceType::FilePath => self.fs_read(task.path),
            ResourceType::HttpUrl => self.http_get(task.url),
            ResourceType::DBUrl => self.db_query(task.url, task.sql),
            ResourceType::Console => self.tty_write(task.data),
        }
    }
    
    fn on_io_complete(&self, handle: IoHandle) {
        // 通知 DAG 调度器唤醒下游节点
        self.dag_scheduler.wake_dependents(handle.node_id);
    }
}
```

---

## 权衡

### 优点

1. **跨平台统一**：libuv 处理 Windows/Linux/macOS 差异
2. **IO 异步能力**：共享事件循环处理所有 IO，无需 async/await
3. **并发安全**：单线程事件循环天然无竞争
4. **资源效率**：一个事件循环，内存开销小
5. **与 RFC-024 契合**：资源类型系统确保并发安全
6. **成熟稳定**：libuv 经过 Node.js 大规模验证

### 缺点

1. **C 库依赖**：需要绑定 libuv C 库
2. **自举限制**：自举后可能需要替换为 YaoXiang 原生实现
3. **WASM 支持**：需要额外适配工作

---

## 替代方案

| 方案 | 为什么不选择 |
|------|--------------|
| Rust std::io | 同步阻塞，无法与 spawn 块配合实现异步 |
| tokio | 为 Rust async/await 设计，与 YaoXiang 显式并发模型不契合 |
| mio | 仅提供原始异步原语，缺乏高级 IO 功能 |
| 从零实现 | 复杂且易出错，无法与 libuv 成熟度相比 |

---

## 实现策略

### 阶段划分

1. **阶段 1（v0.3）**：libuv 绑定、基础文件 IO
2. **阶段 2（v0.5）**：网络 IO、HTTP 支持
3. **阶段 3（v0.7）**：数据库 IO、连接池
4. **阶段 4（v1.0）**：WASM 适配、性能优化

### 依赖关系

- RFC-024（并发模型）→ 已完成
- RFC-008（Runtime 架构）→ 已完成
- RFC-009（所有权模型）→ 已完成
- RFC-011（泛型系统）→ 已完成

---

## 设计决策记录

| 决策 | 决定 | 原因 | 日期 |
|------|------|------|------|
| IO 实现层 | libuv | 跨平台、异步能力、并发安全 | 2025-01-05 |
| 定位 | 资源类型 IO 实现层 | 与 RFC-024 资源类型系统集成 | 2026-06-16 |
| 事件循环架构 | 共享事件循环 | 资源效率高，避免重复创建 | 2026-06-16 |
| 并发安全 | 单线程事件循环 | 天然无竞争，与 RFC-024 契合 | 2026-06-16 |
| 标准库重写 | std.io/std.net 基于 libuv | 跨平台统一、异步能力 | 2026-06-16 |

---

## 开放问题

- [ ] WASM 环境下的 libuv 适配方案
- [ ] 数据库连接池的设计
- [ ] HTTP 客户端的完整实现
- [ ] 文件系统事件的跨平台一致性
- [ ] 网络 IO 的超时机制设计
- [ ] 自举后 libuv 的替换策略

---

## 参考文献

### YaoXiang 官方文档

- [RFC-024 并发模型](./024-concurrency-model.md)
- [RFC-008 Runtime 架构](./008-runtime-concurrency-model.md)
- [RFC-009 所有权模型](./009-ownership-model.md)
- [并发模型规范](/reference/language-spec/concurrency.md)

### 外部参考

- [libuv 官方文档](https://docs.libuv.org/)
- [Node.js 事件循环](https://nodejs.org/en/docs/guides/event-loop-timers-and-nexttick/)
- [Rust libuv 绑定](https://github.com/libuv/libuv)

---

## 生命周期与归宿

| 状态 | 位置 | 说明 |
|------|------|------|
| **草案** | `docs/design/rfc/draft/` | 重新审核中 |

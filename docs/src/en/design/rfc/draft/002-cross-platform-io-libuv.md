---
title: "RFC-002: libuv-based Resource Type IO Implementation Layer"
status: "Draft"
author: "晨煦"
created: "2025-01-05"
updated: "2026-07-05"
issue: "#102"


# RFC-002: libuv-based Resource Type IO Implementation Layer

> **References**:
> - [RFC-024: Concurrency Model Based on spawn blocks](./024-concurrency-model.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [Concurrency Model Specification](/reference/language-spec/concurrency.md)

## Summary

This document defines the IO implementation layer of YaoXiang: providing cross-platform IO capabilities based on libuv, serving as the underlying implementation of the resource type system defined in RFC-024.

**Core Positioning**:

```
RFC-024: Resource Type Definitions (FilePath, HttpUrl, DBUrl, Console)
    ↓ uses
RFC-002: Resource Type IO Implementation (based on libuv)
    ↓ underneath
libuv: Cross-platform IO Engine (event loop + thread pool)
```

**What it is not**:
- ❌ Not "transparent async" — users explicitly control concurrency through spawn blocks
- ❌ Not "automatic async transformation" — IO operations must be explicitly invoked within spawn blocks
- ❌ Not "developers need not care about low-level details" — the resource type system ensures concurrency safety

**What it is**:
- ✅ The IO implementation layer for resource types (FilePath, HttpUrl, DBUrl, Console)
- ✅ Unified cross-platform IO (libuv handles Windows/Linux/macOS differences)
- ✅ Shared event loop architecture (a single libuv event loop handles all IO)
- ✅ Integration with the resource type system from RFC-024

## Motivation

### Why libuv?

RFC-024 defines the resource type system:
- `FilePath` - Filesystem path
- `HttpUrl` - HTTP endpoint
- `DBUrl` - Database connection
- `Console` - Standard output

These resource types require low-level IO implementation. libuv provides:

| Requirement | What libuv provides |
|------|-----------|
| Cross-platform IO | Unified Windows/Linux/macOS API |
| Asynchronous capabilities | Shared event loop, all workers' IO handled centrally |
| Thread pool | Dedicated thread pool for blocking operations |
| Concurrency safety | Single-threaded event loop, naturally race-free |

### Relationship with RFC-024

```
┌─────────────────────────────────────────────────────────┐
│  RFC-024: Concurrency Model                             │
│  - spawn {} blocks (explicit concurrency)               │
│  - Resource type definitions (FilePath, HttpUrl,        │
│    DBUrl, Console)                                      │
│  - Resource conflict detection (auto-serialize          │
│    same path)                                           │
└─────────────────────────────────────────────────────────┘
                          ↓ uses
┌─────────────────────────────────────────────────────────┐
│  RFC-002: Resource Type IO Implementation               │
│  - FilePath → libuv file IO                             │
│  - HttpUrl → libuv network IO                           │
│  - DBUrl → database connection pool                     │
│  - Console → serialized standard output                 │
└─────────────────────────────────────────────────────────┘
                          ↓ underneath
┌─────────────────────────────────────────────────────────┐
│  libuv: Cross-platform IO Engine                        │
│  - Event loop                                           │
│  - Thread pool                                          │
│  - Cross-platform unified API                           │
└─────────────────────────────────────────────────────────┘
```

---

## Proposal

### 1. libuv Architecture

#### 1.1 Shared Event Loop Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Runtime                               │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │  Worker 0   │  │  Worker 1   │  │  Worker N   │    │
│  │ Compute task│  │ Compute task│  │ Compute task│    │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘    │
│         │                │                │            │
│         └────────────────┼────────────────┘            │
│                          ↓                              │
│  ┌─────────────────────────────────────────────────┐  │
│  │     libuv Event Loop (dedicated thread)          │  │
│  │     Handles all IO operations                    │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**Key characteristics**:
- A single shared libuv event loop (running on a dedicated thread)
- IO operations from all workers are submitted to this shared event loop
- Single-threaded event loop naturally avoids race conditions
- Resource-efficient; no need to create an event loop per worker

#### 1.2 Concurrency Safety Mechanism

| libuv Feature | YaoXiang Counterpart | Concurrency Safety |
|------------|---------------|----------|
| Single-threaded event loop | Sequential execution within spawn block | Naturally race-free |
| Thread pool isolation | Blocking operations don't block main thread | No shared state |
| Async callbacks | DAG scheduler manages dependencies | Deterministic execution |

### 2. Resource Type IO Mapping

#### 2.1 FilePath → libuv File IO

```rust
// std.io module (based on libuv)
pub struct IoModule;

impl StdModule for IoModule {
    fn exports(&self) -> Vec<NativeExport> {
        vec![
            // File operations → libuv fs_* API
            NativeExport::new("read_file", "std.io.read_file",
                "(path: FilePath) -> String", native_read_file),
            NativeExport::new("write_file", "std.io.write_file",
                "(path: FilePath, content: String) -> Bool", native_write_file),
            NativeExport::new("append_file", "std.io.append_file",
                "(path: FilePath, content: String) -> Bool", native_append_file),
            // Console operations → libuv tty API
            NativeExport::new("print", "std.io.print",
                "(...args) -> ()", native_print),
            NativeExport::new("println", "std.io.println",
                "(...args) -> ()", native_println),
        ]
    }
}

// libuv file IO implementation
fn native_read_file(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let path = extract_file_path(args)?;

    // Submit to libuv event loop
    // libuv asynchronously reads the file
    // Return result
    ctx.uv_loop.fs_read(path)
}
```

#### 2.2 HttpUrl → libuv Network IO

```rust
// std.net module (based on libuv)
pub struct NetModule;

impl StdModule for NetModule {
    fn exports(&self) -> Vec<NativeExport> {
        vec![
            // HTTP operations → libuv http API
            NativeExport::new("http_get", "std.net.http_get",
                "(url: HttpUrl) -> Response", native_http_get),
            NativeExport::new("http_post", "std.net.http_post",
                "(url: HttpUrl, body: String) -> Response", native_http_post),
        ]
    }
}

// libuv network IO implementation
fn native_http_get(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let url = extract_http_url(args)?;

    // Submit to libuv event loop
    // libuv asynchronous HTTP request
    // Return result
    ctx.uv_loop.http_get(url)
}
```

#### 2.3 DBUrl → Database Connection Pool

```rust
// std.db module (based on libuv)
pub struct DbModule;

impl StdModule for DbModule {
    fn exports(&self) -> Vec<NativeExport> {
        vec![
            // Database operations → libuv thread pool
            NativeExport::new("query", "std.db.query",
                "(url: DBUrl, sql: String) -> Rows", native_query),
        ]
    }
}

// libuv database IO implementation
fn native_query(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let url = extract_db_url(args)?;
    let sql = extract_sql(args)?;

    // Submit to libuv thread pool
    // Database query executes in the thread pool
    // Notify main thread on completion via callback
    ctx.uv_loop.db_query(url, sql)
}
```

#### 2.4 Console → Serialized Standard Output

```rust
// Console operations are auto-serialized (RFC-024 resource type rules)
// All Console operations execute sequentially within the same thread
fn native_print(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let output = format_args(args);

    // Console operations are serialized
    // libuv tty write
    ctx.uv_loop.tty_write(output)
}
```

### 3. Integration with spawn Blocks

#### 3.1 User Perspective

```yaoxiang
# Resource type definitions (RFC-024)
FilePath: Resource
HttpUrl: Resource

# IO operations (RFC-002 implementation)
File.read: (FilePath) -> String
HTTP.get: (HttpUrl) -> Response

# User explicit concurrency (RFC-024)
(a, b) = spawn {
    read_file("data.txt"),      # Resource type FilePath, libuv underneath
    fetch("http://example.com") # Resource type HttpUrl, libuv underneath
}
# Compiler: FilePath and HttpUrl have no conflict, can run in parallel
```

#### 3.2 Compile-time Analysis

```
Compiler analyzes spawn block:
1. Identify resource type operations
2. Detect resource conflicts (auto-serialize same path/URL)
3. Generate DAG execution plan
4. Mark IO nodes (submit to libuv)
```

#### 3.3 Runtime Execution

```
Runtime executes spawn block:
1. Worker 0 submits IO task → shared event loop
2. Worker 1 submits IO task → shared event loop
3. Event loop handles all IO operations uniformly
4. On IO completion, notify the corresponding Worker
5. Worker continues executing subsequent tasks
```

### 4. Runtime Three-layer Architecture and libuv

| Layer | libuv Usage | Async Capability | Applicable Scenarios |
|------|-----------|----------|----------|
| Embedded Runtime | No libuv | No async | WASM, game scripts |
| Standard Runtime | Shared event loop | IO async | Web services, data pipelines |
| Full Runtime | Shared event loop | IO async + parallel | Scientific computing, large-scale parallelism |

**Embedded Runtime**: No libuv, executes immediately, no async capabilities.

**Standard Runtime**: Shared libuv event loop, all IO operations handled asynchronously.

**Full Runtime**: Shared libuv event loop, multi-threaded parallelism + IO async.

---

## Detailed Design

### 1. Rust Binding Structure

```rust
// libuv binding module
pub mod uv {
    // Event loop
    pub struct UvLoop {
        loop_handle: *mut uv_loop_t,
    }

    // File operations
    pub trait FileOps {
        fn fs_read(&self, path: &str) -> Result<String, UvError>;
        fn fs_write(&self, path: &str, content: &str) -> Result<(), UvError>;
        fn fs_append(&self, path: &str, content: &str) -> Result<(), UvError>;
    }

    // Network operations
    pub trait NetOps {
        fn http_get(&self, url: &str) -> Result<Response, UvError>;
        fn http_post(&self, url: &str, body: &str) -> Result<Response, UvError>;
    }

    // Database operations
    pub trait DbOps {
        fn db_query(&self, url: &str, sql: &str) -> Result<Rows, UvError>;
    }

    // Console operations
    pub trait ConsoleOps {
        fn tty_write(&self, data: &str) -> Result<(), UvError>;
    }
}
```

### 2. Standard Library Module Structure

```
src/std/
├── io.rs          # FilePath IO (based on libuv)
├── net.rs         # HttpUrl IO (based on libuv)
├── db.rs          # DBUrl IO (based on libuv)
├── console.rs     # Console IO (based on libuv)
└── mod.rs         # Module registration
```

### 3. Integration with the DAG Scheduler

```rust
// IO node interface (defined in RFC-008)
trait IoScheduler {
    // Submit an IO task, return a handle
    fn submit_io(&self, task: IoTask) -> IoHandle;

    // Called by libuv on IO completion, wakes up the DAG node
    fn on_io_complete(&self, handle: IoHandle);
}

// libuv implementation
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
        // Notify the DAG scheduler to wake up downstream nodes
        self.dag_scheduler.wake_dependents(handle.node_id);
    }
}
```

---

## Trade-offs

### Advantages

1. **Cross-platform unification**: libuv handles Windows/Linux/macOS differences
2. **IO async capabilities**: shared event loop handles all IO, no need for async/await
3. **Concurrency safety**: single-threaded event loop naturally race-free
4. **Resource efficiency**: one event loop, small memory overhead
5. **Alignment with RFC-024**: resource type system ensures concurrency safety
6. **Mature and stable**: libuv battle-tested at scale by Node.js

### Disadvantages

1. **C library dependency**: requires binding to the libuv C library
2. **Bootstrap limitation**: may need to replace with a YaoXiang native implementation after bootstrap
3. **WASM support**: requires additional adaptation work

---

## Alternatives

| Alternative | Why Not Chosen |
|------|--------------|
| Rust std::io | Synchronous blocking, cannot coordinate with spawn blocks to implement async |
| tokio | Designed for Rust async/await, doesn't fit YaoXiang's explicit concurrency model |
| mio | Provides only raw async primitives, lacks high-level IO features |
| Implement from scratch | Complex and error-prone, cannot match libuv's maturity |

---

## Implementation Strategy

### Phasing

1. **Phase 1 (v0.3)**: libuv binding, basic file IO
2. **Phase 2 (v0.5)**: Network IO, HTTP support
3. **Phase 3 (v0.7)**: Database IO, connection pool
4. **Phase 4 (v1.0)**: WASM adaptation, performance optimization

### Dependencies

- RFC-024 (Concurrency Model) → Completed
- RFC-008 (Runtime Architecture) → Completed
- RFC-009 (Ownership Model) → Completed
- RFC-011 (Generics System) → Completed

---

## Design Decision Records

| Decision | Resolution | Reason | Date |
|------|------|------|------|
| IO implementation layer | libuv | Cross-platform, async capability, concurrency safety | 2025-01-05 |
| Positioning | Resource type IO implementation layer | Integration with RFC-024 resource type system | 2026-06-16 |
| Event loop architecture | Shared event loop | Resource-efficient, avoids redundant creation | 2026-06-16 |
| Concurrency safety | Single-threaded event loop | Naturally race-free, aligns with RFC-024 | 2026-06-16 |
| Standard library rewrite | std.io/std.net based on libuv | Cross-platform unification, async capability | 2026-06-16 |

---

## Open Questions

- [ ] libuv adaptation approach in WASM environments
- [ ] Design of the database connection pool
- [ ] Complete implementation of the HTTP client
- [ ] Cross-platform consistency of filesystem events
- [ ] Timeout mechanism design for network IO
- [ ] Strategy for replacing libuv after bootstrap

---

## References

### YaoXiang Official Documentation

- [RFC-024 Concurrency Model](./024-concurrency-model.md)
- [RFC-008 Runtime Architecture](./008-runtime-concurrency-model.md)
- [RFC-009 Ownership Model](./009-ownership-model.md)
- [Concurrency Model Specification](/reference/language-spec/concurrency.md)

### External References

- [libuv Official Documentation](https://docs.libuv.org/)
- [Node.js Event Loop](https://nodejs.org/en/docs/guides/event-loop-timers-and-nexttick/)
- [Rust libuv Bindings](https://github.com/libuv/libuv)

---

## Lifecycle and Destination

| Status | Location | Description |
|------|------|------|
| **Draft** | `docs/design/rfc/draft/` | Under re-review |
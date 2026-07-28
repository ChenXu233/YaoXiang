---
title: 'RFC-002: libuv-based IO Implementation Layer for Resource Types'
status: 'Draft'
author: '晨煦'
created: '2025-01-05'
updated: '2026-07-05'
issue: '#102'
---

# RFC-002: libuv-based IO Implementation Layer for Resource Types

> **References**:
>
> - [RFC-024: Concurrency Model Based on Spawn Blocks](./024-concurrency-model.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [Concurrency Model Specification](/reference/language-spec/concurrency.md)

## Abstract

This document defines the IO implementation layer for YaoXiang: providing cross-platform IO
capabilities based on libuv, serving as the underlying implementation for the RFC-024 resource type
system.

**Core Positioning**:

```
RFC-024: Resource Type Definitions (FilePath, HttpUrl, DBUrl, Console)
    ↓ uses
RFC-002: Resource Type IO Implementation (based on libuv)
    ↓ underlying
libuv: Cross-platform IO Engine (event loop + thread pool)
```

**What it is NOT**:

- ❌ Not "transparent async" — users explicitly control concurrency through spawn blocks
- ❌ Not "auto-async" — IO operations must be explicitly called within spawn blocks
- ❌ Not "developers don't need to care about underlying details" — the resource type system ensures
  concurrency safety

**What it IS**:

- ✅ IO implementation layer for resource types (FilePath, HttpUrl, DBUrl, Console)
- ✅ Cross-platform IO unification (libuv handles Windows/Linux/macOS differences)
- ✅ Shared event loop architecture (one libuv event loop handles all IO)
- ✅ Integration with RFC-024 resource type system

## Motivation

### Why libuv?

RFC-024 defines the resource type system:

- `FilePath` - filesystem path
- `HttpUrl` - HTTP endpoint
- `DBUrl` - database connection
- `Console` - standard output

These resource types require underlying IO implementation. libuv provides:

| Requirement        | libuv Provides                                         |
| ------------------ | ------------------------------------------------------ |
| Cross-platform IO  | Unified Windows/Linux/macOS API                        |
| Async capabilities | Shared event loop, all workers' IO centrally processed |
| Thread pool        | Dedicated thread pool for blocking operations          |
| Concurrency safety | Single-threaded event loop, inherently race-free       |

### Relationship with RFC-024

```
┌─────────────────────────────────────────────────────────┐
│  RFC-024: Concurrency Model                              │
│  - spawn {} blocks (explicit concurrency)               │
│  - Resource type definitions (FilePath, HttpUrl, DBUrl, Console)
│  - Resource conflict detection (auto-serialization for same path)
└─────────────────────────────────────────────────────────┘
                          ↓ uses
┌─────────────────────────────────────────────────────────┐
│  RFC-002: Resource Type IO Implementation                │
│  - FilePath → libuv file IO                              │
│  - HttpUrl → libuv network IO                            │
│  - DBUrl → database connection pool                     │
│  - Console → standard output serialization               │
└─────────────────────────────────────────────────────────┘
                          ↓ underlying
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
│  │  Compute    │  │  Compute    │  │  Compute    │    │
│  │  Tasks      │  │  Tasks      │  │  Tasks      │    │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘    │
│         │                │                │            │
│         └────────────────┼────────────────┘            │
│                          ↓                              │
│  ┌─────────────────────────────────────────────────┐  │
│  │          libuv Event Loop (dedicated thread)    │  │
│  │          Handles all IO operations               │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**Key Features**:

- One shared libuv event loop (running on a dedicated thread)
- All workers' IO operations are submitted to this shared event loop
- Single-threaded event loop inherently avoids races
- High resource efficiency — no need to create event loops for each worker

#### 1.2 Concurrency Safety Mechanisms

| libuv Feature              | YaoXiang Corresponding                   | Concurrency Safety      |
| -------------------------- | ---------------------------------------- | ----------------------- |
| Single-threaded event loop | Sequential execution within spawn blocks | Inherently race-free    |
| Thread pool isolation      | Blocking ops don't block main thread     | No shared state         |
| Async callbacks            | DAG scheduler manages dependencies       | Deterministic execution |

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
    // libuv reads file asynchronously
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
    // libuv async HTTP request
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
    // Database query executes in thread pool
    // Callback notifies main thread on completion
    ctx.uv_loop.db_query(url, sql)
}
```

#### 2.4 Console → Standard Output Serialization

```rust
// Console operations auto-serialized (RFC-024 resource type rules)
// All Console operations execute sequentially within the same thread
fn native_print(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let output = format_args(args);

    // Console operation serialization
    // libuv tty write
    ctx.uv_loop.tty_write(output)
}
```

### 3. Integration with Spawn Blocks

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
    read_file("data.txt"),      # Resource type FilePath, underlying libuv
    fetch("http://example.com") # Resource type HttpUrl, underlying libuv
}
# Compiler: FilePath and HttpUrl have no conflict, can execute in parallel
```

#### 3.2 Compile-time Analysis

```
Compiler analyzes spawn block:
1. Identify resource type operations
2. Detect resource conflicts (same path/URL auto-serialized)
3. Generate DAG execution plan
4. Mark IO nodes (submit to libuv)
```

#### 3.3 Runtime Execution

```
Runtime executes spawn block:
1. Worker 0 submits IO task → shared event loop
2. Worker 1 submits IO task → shared event loop
3. Event loop uniformly processes all IO operations
4. IO completion notifies corresponding Worker
5. Worker continues executing subsequent tasks
```

### 4. Runtime Three-Layer Architecture and libuv

| Layer            | libuv Usage       | Async Capability    | Applicable Scenarios                          |
| ---------------- | ----------------- | ------------------- | --------------------------------------------- |
| Embedded Runtime | No libuv          | No async            | WASM, game scripts                            |
| Standard Runtime | Shared event loop | IO async            | Web services, data pipelines                  |
| Full Runtime     | Shared event loop | IO async + parallel | Scientific computing, large-scale parallelism |

**Embedded Runtime**: No libuv, immediate execution, no async capability.

**Standard Runtime**: Shared libuv event loop, all IO operations handled asynchronously.

**Full Runtime**: Shared libuv event loop, multi-threaded parallelism + IO async.

---

## Detailed Design

### 1. Rust Bindings Structure

```rust
// libuv bindings module
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

### 3. Integration with DAG Scheduler

```rust
// IO node interface (defined in RFC-008)
trait IoScheduler {
    // Submit IO task, return handle
    fn submit_io(&self, task: IoTask) -> IoHandle;

    // Called by libuv when IO completes, wake DAG node
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
        // Notify DAG scheduler to wake downstream nodes
        self.dag_scheduler.wake_dependents(handle.node_id);
    }
}
```

---

## Trade-offs

### Advantages

1. **Cross-platform unification**: libuv handles Windows/Linux/macOS differences
2. **IO async capability**: Shared event loop handles all IO, no async/await needed
3. **Concurrency safety**: Single-threaded event loop inherently race-free
4. **Resource efficiency**: One event loop, small memory overhead
5. **Aligned with RFC-024**: Resource type system ensures concurrency safety
6. **Mature and stable**: libuv validated at scale by Node.js

### Disadvantages

1. **C library dependency**: Requires binding to libuv C library
2. **Bootstrap limitation**: May need replacement with YaoXiang native implementation after
   bootstrap
3. **WASM support**: Requires additional adaptation work

---

## Alternative Approaches

| Approach           | Why Not Chosen                                                                        |
| ------------------ | ------------------------------------------------------------------------------------- |
| Rust std::io       | Synchronous blocking, cannot cooperate with spawn blocks for async                    |
| tokio              | Designed for Rust async/await, not aligned with YaoXiang's explicit concurrency model |
| mio                | Only provides raw async primitives, lacks high-level IO functionality                 |
| Build from scratch | Complex and error-prone, cannot match libuv's maturity                                |

---

## Implementation Strategy

### Phase Breakdown

1. **Phase 1 (v0.3)**: libuv bindings, basic file IO
2. **Phase 2 (v0.5)**: Network IO, HTTP support
3. **Phase 3 (v0.7)**: Database IO, connection pool
4. **Phase 4 (v1.0)**: WASM adaptation, performance optimization

### Dependencies

- RFC-024 (concurrency model) → Completed
- RFC-008 (Runtime architecture) → Completed
- RFC-009 (ownership model) → Completed
- RFC-011 (generics system) → Completed

---

## Design Decision Records

| Decision                 | Decision                              | Rationale                                            | Date       |
| ------------------------ | ------------------------------------- | ---------------------------------------------------- | ---------- |
| IO implementation layer  | libuv                                 | Cross-platform, async capability, concurrency safety | 2025-01-05 |
| Positioning              | Resource type IO implementation layer | Integration with RFC-024 resource type system        | 2026-06-16 |
| Event loop architecture  | Shared event loop                     | High resource efficiency, avoid duplicate creation   | 2026-06-16 |
| Concurrency safety       | Single-threaded event loop            | Inherently race-free, aligned with RFC-024           | 2026-06-16 |
| Standard library rewrite | std.io/std.net based on libuv         | Cross-platform unification, async capability         | 2026-06-16 |

---

## Open Questions

- [ ] libuv adaptation solution for WASM environment
- [ ] Database connection pool design
- [ ] Complete HTTP client implementation
- [ ] Cross-platform consistency for filesystem events
- [ ] Timeout mechanism design for network IO
- [ ] libuv replacement strategy after bootstrap

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

## Lifecycle and Disposition

| Status    | Location                 | Description  |
| --------- | ------------------------ | ------------ |
| **Draft** | `docs/design/rfc/draft/` | Under review |

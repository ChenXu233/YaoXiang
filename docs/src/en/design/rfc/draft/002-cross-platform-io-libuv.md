---
title: "RFC-002: libuv-based IO Implementation Layer for Resource Types"
status: "Draft"
author: "Chenxu"
created: "2025-01-05"
updated: "2026-06-16 (Revision: positioned as resource type IO implementation layer, removed transparent asynchrony, aligned with RFC-024; shared event loop architecture)"
---

# RFC-002: libuv-based IO Implementation Layer for Resource Types

> **References**:
> - [RFC-024: Concurrency Model Based on spawn Blocks](./024-concurrency-model.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [Concurrency Model Specification](/reference/language-spec/concurrency.md)

## Abstract

This document defines the IO implementation layer of YaoXiang: providing cross-platform IO capabilities based on libuv, serving as the underlying implementation of the resource type system defined in RFC-024.

**Core Positioning**:

```
RFC-024: Resource Type Definition (FilePath, HttpUrl, DBUrl, Console)
    ↓ uses
RFC-002: Resource Type IO Implementation (based on libuv)
    ↓ underlying
libuv: Cross-platform IO engine (event loop + thread pool)
```

**What this is NOT**:
- ❌ NOT "transparent asynchrony" — users explicitly control concurrency through spawn blocks
- ❌ NOT "automatic asynchronization" — IO operations must be explicitly invoked within spawn blocks
- ❌ NOT "developers need not care about underlying details" — the resource type system ensures concurrency safety

**What this IS**:
- ✅ The IO implementation layer for resource types (FilePath, HttpUrl, DBUrl, Console)
- ✅ Cross-platform IO unification (libuv handles Windows/Linux/macOS differences)
- ✅ Shared event loop architecture (a single libuv event loop handles all IO)
- ✅ Integration with the RFC-024 resource type system

## Motivation

### Why libuv?

RFC-024 defines the resource type system:
- `FilePath` - Filesystem path
- `HttpUrl` - HTTP endpoint
- `DBUrl` - Database connection
- `Console` - Standard output

These resource types require underlying IO implementation. libuv provides:

| Requirement | libuv Provides |
|------|-----------|
| Cross-platform IO | Unified Windows/Linux/macOS API |
| Asynchronous capability | Shared event loop, centralized IO processing for all workers |
| Thread pool | Dedicated thread pool for blocking operations |
| Concurrency safety | Single-threaded event loop, inherently race-free |

### Relationship with RFC-024

```
┌─────────────────────────────────────────────────────────┐
│  RFC-024: Concurrency Model                              │
│  - spawn {} blocks (explicit concurrency)                │
│  - Resource type definition (FilePath, HttpUrl, DBUrl, Console) │
│  - Resource conflict detection (auto-serialize on same path) │
└─────────────────────────────────────────────────────────┘
                          ↓ uses
┌─────────────────────────────────────────────────────────┐
│  RFC-002: Resource Type IO Implementation                │
│  - FilePath → libuv file IO                              │
│  - HttpUrl → libuv network IO                            │
│  - DBUrl → Database connection pool                      │
│  - Console → Standard output serialization               │
└─────────────────────────────────────────────────────────┘
                          ↓ underlying
┌─────────────────────────────────────────────────────────┐
│  libuv: Cross-platform IO Engine                         │
│  - Event loop                                             │
│  - Thread pool                                            │
│  - Cross-platform unified API                             │
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
│  │  tasks      │  │  tasks      │  │  tasks      │    │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘    │
│         │                │                │            │
│         └────────────────┼────────────────┘            │
│                          ↓                              │
│  ┌─────────────────────────────────────────────────┐  │
│  │       libuv Event Loop (dedicated thread)        │  │
│  │       Handles all IO operations                  │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**Key characteristics**:
- A single shared libuv event loop (running on a dedicated thread)
- IO operations from all workers are submitted to this shared event loop
- Single-threaded event loop inherently avoids races
- High resource efficiency — no need to create an event loop per worker

#### 1.2 Concurrency Safety Mechanism

| libuv Feature | YaoXiang Equivalent | Concurrency Safety |
|------------|---------------|----------|
| Single-threaded event loop | Sequential execution within spawn blocks | Inherently race-free |
| Thread pool isolation | Blocking operations do not block the main thread | No shared state |
| Asynchronous callbacks | DAG scheduler manages dependencies | Deterministic execution |

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
    // Notify main thread via callback upon completion
    ctx.uv_loop.db_query(url, sql)
}
```

#### 2.4 Console → Standard Output Serialization

```rust
// Console operations are automatically serialized (RFC-024 resource type rules)
// All Console operations execute sequentially within the same thread
fn native_print(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let output = format_args(args);
    
    // Serialize Console operations
    // libuv tty write
    ctx.uv_loop.tty_write(output)
}
```

### 3. Integration with spawn Blocks

#### 3.1 User Perspective

```yaoxiang
# Resource type definition (RFC-024)
FilePath: Resource
HttpUrl: Resource

# IO operations (RFC-002 implementation)
File.read: (FilePath) -> String
HTTP.get: (HttpUrl) -> Response

# User-explicit concurrency (RFC-024)
(a, b) = spawn {
    read_file("data.txt"),      # resource type FilePath, backed by libuv
    fetch("http://example.com") # resource type HttpUrl, backed by libuv
}
# Compiler: FilePath and HttpUrl have no conflict — can run in parallel
```

#### 3.2 Compile-time Analysis

```
Compiler analyzes spawn block:
1. Identify resource type operations
2. Detect resource conflicts (auto-serialize on same path/URL)
3. Generate DAG execution plan
4. Mark IO nodes (to be submitted to libuv)
```

#### 3.3 Runtime Execution

```
Runtime executes spawn block:
1. Worker 0 submits IO task → shared event loop
2. Worker 1 submits IO task → shared event loop
3. Event loop handles all IO operations uniformly
4. Notify corresponding worker upon IO completion
5. Worker continues with subsequent tasks
```

### 4. Runtime Three-tier Architecture and libuv

| Tier | libuv Usage | Asynchronous Capability | Applicable Scenarios |
|------|-----------|----------|----------|
| Embedded Runtime | No libuv | No asynchrony | WASM, game scripts |
| Standard Runtime | Shared event loop | IO asynchrony | Web services, data pipelines |
| Full Runtime | Shared event loop | IO asynchrony + parallelism | Scientific computing, large-scale parallelism |

**Embedded Runtime**: No libuv, immediate execution, no asynchronous capability.

**Standard Runtime**: Shared libuv event loop, all IO operations handled asynchronously.

**Full Runtime**: Shared libuv event loop, multi-threaded parallelism + IO asynchrony.

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

### 3. Integration with DAG Scheduler

```rust
// IO node interface (defined by RFC-008)
trait IoScheduler {
    // Submit IO task, return handle
    fn submit_io(&self, task: IoTask) -> IoHandle;
    
    // Called by libuv upon IO completion, wakes DAG node
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
2. **IO asynchrony**: shared event loop handles all IO without async/await
3. **Concurrency safety**: single-threaded event loop is inherently race-free
4. **Resource efficiency**: a single event loop, small memory overhead
5. **Alignment with RFC-024**: resource type system ensures concurrency safety
6. **Maturity and stability**: libuv has been battle-tested by Node.js at scale

### Disadvantages

1. **C library dependency**: requires binding the libuv C library
2. **Bootstrapping limitation**: may need to be replaced with a native YaoXiang implementation after bootstrapping
3. **WASM support**: requires additional adaptation work

---

## Alternatives

| Alternative | Why Not Chosen |
|------|--------------|
| Rust std::io | Synchronous blocking; cannot be combined with spawn blocks for asynchrony |
| tokio | Designed for Rust async/await; does not fit YaoXiang's explicit concurrency model |
| mio | Provides only raw asynchronous primitives; lacks higher-level IO features |
| Build from scratch | Complex and error-prone; cannot match libuv's maturity |

---

## Implementation Strategy

### Phasing

1. **Phase 1 (v0.3)**: libuv bindings, basic file IO
2. **Phase 2 (v0.5)**: Network IO, HTTP support
3. **Phase 3 (v0.7)**: Database IO, connection pool
4. **Phase 4 (v1.0)**: WASM adaptation, performance optimization

### Dependencies

- RFC-024 (Concurrency Model) → Completed
- RFC-008 (Runtime Architecture) → Completed
- RFC-009 (Ownership Model) → Completed
- RFC-011 (Generics System) → Completed

---

## Design Decision Log

| Decision | Resolution | Reason | Date |
|------|------|------|------|
| IO implementation layer | libuv | Cross-platform, asynchronous capability, concurrency safety | 2025-01-05 |
| Positioning | Resource type IO implementation layer | Integration with RFC-024 resource type system | 2026-06-16 |
| Event loop architecture | Shared event loop | High resource efficiency, avoids redundant creation | 2026-06-16 |
| Concurrency safety | Single-threaded event loop | Inherently race-free, aligns with RFC-024 | 2026-06-16 |
| Standard library rewrite | std.io/std.net based on libuv | Cross-platform unification, asynchronous capability | 2026-06-16 |

---

## Open Questions

- [ ] libuv adaptation strategy in the WASM environment
- [ ] Database connection pool design
- [ ] Complete implementation of the HTTP client
- [ ] Cross-platform consistency of filesystem events
- [ ] Timeout mechanism design for network IO
- [ ] Strategy for replacing libuv after bootstrapping

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
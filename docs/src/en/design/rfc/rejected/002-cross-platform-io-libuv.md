---
title: RFC-002: Cross-Platform I/O and libuv Integration (Rejected)
---

# RFC-002: Cross-Platform I/O and libuv Integration

> **Status**: Rejected
> **Author**: Chen Xu
> **Created**: 2025-01-05
> **Last Updated**: 2026-02-15

## Rejection Reasons

This RFC is rejected for the following reasons:

### 1. libuv is a C library and cannot be used after YaoXiang self-hosts

YaoXiang ultimately needs to self-host (implement an interpreter in YaoXiang itself), at which point it cannot depend on C libraries.
libuv as a C library requires FFI calls, which would hinder the self-hosting process.

### 2. tokio is a more suitable choice

In the Rust ecosystem, tokio is the dominant async runtime (>90% market share), it's pure Rust,
and after self-hosting it can continue to be used through bindings, making it more aligned with the long-term architecture than libuv.

### 3. Pragmatic considerations

At the current stage, priority should be given to getting the YaoXiang language working end-to-end; I/O can be implemented quickly using Rust std,
and a proper async runtime can be developed after self-hosting using tokio bindings or custom development.

---

## Summary

This proposal presents a cross-platform async I/O solution for YaoXiang, integrating libuv to implement a unified async abstraction. The core design goal is to automatically and transparently async-ify blocking I/O operations, freeing developers from worrying about low-level details.

## Motivation

### Why libuv?

YaoXiang's spawn model requires efficient async I/O support:

| Requirement | Problems with Traditional Solutions |
|-------------|-------------------------------------|
| Cross-platform I/O | Inconsistent APIs across platforms (Windows IOCP, Linux epoll, macOS kqueue) |
| Async event loop | Complex and error-prone to implement from scratch |
| Thread pool management | Blocking operations require dedicated thread pools |
| Performance requirements | Need zero-overhead async abstractions |

### Advantages of libuv

```
libuv ✓ Mature and stable - Node.js underlying runtime, proven at scale
libuv ✓ Cross-platform - Unified I/O API for Windows, Linux, macOS
libuv ✓ High performance - Event-driven, non-blocking I/O
libuv ✓ Thread pool - Built-in blocking operation thread pool management
```

## Proposal

### 1. Technology Selection Decision

| Component | Selection | Rationale |
|-----------|-----------|-----------|
| I/O Runtime | libuv | Cross-platform mature, Node.js verified |
| Event Loop | libuv loop | Lightweight, efficient |
| Thread Pool | libuv + custom | Dedicated for blocking operations |
| Scheduling Algorithm | Work-stealing + DAG optimization | High performance, load balancing |
| Memory Management | Ownership + stack allocation | No GC, zero-cost abstractions |

### 2. Architecture Design

#### 2.1 Overall Runtime Structure

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
│          │   (Cross-platform I/O) │                       │
│          └───────────┬───────────┘                        │
│                      │                                    │
│          ┌───────────▼───────────┐                        │
│          │   Thread Pool         │  ← Blocking ops only   │
│          │   (libuv threads)     │                        │
│          └───────────────────────┘                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 2.2 Runtime Structure Definitions

```rust
struct YaoXiangRuntime {
    // libuv event loop (cross-platform I/O core)
    uv_loop: *mut uv_loop_t,

    // Work-stealing scheduler
    scheduler: WorkStealingScheduler,

    // Blocking operation thread pool
    io_thread_pool: ThreadPool,

    // Task queues
    task_queues: Vec<Deque<Task>>,

    // Statistics
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

### 3. Unified Async Abstraction

#### 3.1 Blocking to Transparent Conversion

```
┌─────────────────────────────────────────────────────────────┐
│  Blocking C function  →  Auto-wrap  →  Transparent Async[T] │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  // Original blocking API                                   │
│  data = File.read("file.txt")  // Blocking call             │
│                                                             │
│  // YaoXiang auto-conversion                                │
│  // 1. Detect blocking call                                 │
│  // 2. Auto-submit to thread pool                          │
│  // 3. Return Async[T] proxy                               │
│  // 4. Auto-await when used                                │
│                                                             │
│  // Developer perspective                                  │
│  content = File.read("config.yaml")  // Async[String]       │
│  data = parse(content)               // Auto-await         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 3.2 I/O Operation Examples

```yaoxiang
# Async file read (developer perspective: synchronous syntax, auto-async)
read_config: (String) -> Config spawn = (path) => {
    content = File.read(path)  # Auto-asyncified
    config = parse_yaml(content)
    config
}

# Async network request
fetch_user: (Int) -> User spawn = (user_id) => {
    response = HTTP.get("/users/" + user_id.to_string())
    parse_user(response.body())
}

# Concurrent file processing
process_files: ([String]) -> [Result[FileData, Error]] = (paths) => {
    # Auto-parallel read all files
    data = paths.map(path => {
        File.read(path)  # spawn auto-inserted
    })
    data.map(d => process_content(d))
}

# Streaming processing (chunked read)
stream_large_file: (String) -> Void = (path) => {
    stream = File.open_stream(path)
    for chunk in stream.chunks(8192) {  # Auto-async iteration
        process(chunk)
    }
}
```

#### 3.3 Network I/O

```yaoxiang
# HTTP server
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
    server.serve(router)  # Auto-handle concurrent requests
}

# WebSocket
chat_server: (String) -> Void spawn = (port) => {
    ws = WebSocket.new("ws://localhost:" + port.to_string())
    for message in ws.incoming() {  # Auto-streaming
        broadcast(message)
    }
}
```

### 4. Cross-Platform Guarantees

#### 4.1 Platform Support Matrix

| Platform | Status | Event Mechanism | Notes |
|----------|--------|-----------------|-------|
| **Linux** | ✅ Supported | epoll | Primary development platform |
| **macOS** | ✅ Supported | kqueue | Native libuv support |
| **Windows** | ✅ Supported | IOCP | Native libuv support |
| **WASM** | ⚠️ TBD | Browser API | Requires additional adaptation |
| **WASI** | ⚠️ TBD | WASI calls | Long-term goal |

#### 4.2 Cross-Platform API Unification

```yaoxiang
# File I/O - Unified API
file_api: () -> Void = () => {
    # Same API across all platforms
    content = File.read("data.txt")      # Read
    File.write("output.txt", content)    # Write
    exists = File.exists("data.txt")     # Check
    File.delete("temp.txt")              # Delete
}

# Network I/O - Unified API
network_api: () -> Void = () => {
    socket = Net.Socket.new(Net.IP.v4(127, 0, 0, 1), 8080)
    socket.connect()
    socket.send("Hello")
    response = socket.recv()
    socket.close()
}

# Process I/O - Unified API
process_api: () -> Void = () => {
    output = Process.run("ls", ["-la"])  # Cross-platform execution
    print(output.stdout)
}
```

#### 4.3 Platform-Specific Optimizations

```yaoxiang
# Windows-specific optimization
when os() == "windows" {
    use_windows_registry()
}

# Linux-specific optimization
when os() == "linux" {
    use_inotify()
}

# macOS-specific optimization
when os() == "macos" {
    use_fsevents()
}
```

### 5. Performance Considerations

#### 5.1 Thread Pool Configuration

```yaoxiang
# Script header configures thread pool size
# @thread_pool: 4

# Or runtime configuration
configure_runtime: () -> Void = () => {
    Runtime.set_thread_pool_size(8)
    Runtime.set_max_concurrent_tasks(100)
}
```

#### 5.2 I/O Batch Optimization

```yaoxiang
# Batch file operations (reduce system calls)
batch_read: ([String]) -> [String] = (paths) => {
    # libuv batch submit, reduce context switching
    File.batch_read(paths)
}

# Zero-copy optimization
zero_copy_transfer: (String, String) -> Void = (src, dst) => {
    # Use sendfile/splice on supported platforms
    File.transfer(src, dst)
}
```

## Detailed Design

### 1. Rust Binding Structure

```rust
// libuv binding module
pub mod uv {
    use std::ffi::c_void;
    use std::ptr::null_mut;

    // Base types
    pub struct UvLoop(uv_loop_t);

    // File operations
    pub trait FileOps {
        fn fs_open(path: &str, flags: i32, mode: i32) -> Result<RawFd, Errno>;
        fn fs_read(fd: RawFd, buf: &mut [u8], offset: i64) -> Result<usize, Errno>;
        fn fs_write(fd: RawFd, buf: &[u8], offset: i64) -> Result<usize, Errno>;
        fn fs_close(fd: RawFd) -> Result<(), Errno>;
    }

    // Network operations
    pub trait NetOps {
        fn tcp_new() -> Result<RawTcpSocket, Errno>;
        fn tcp_connect(socket: RawTcpSocket, addr: &SocketAddr) -> Result<(), Errno>;
        fn tcp_read(socket: RawTcpSocket, buf: &mut [u8]) -> Result<usize, Errno>;
        fn tcp_write(socket: RawTcpSocket, buf: &[u8]) -> Result<usize, Errno>;
    }

    // Thread pool
    pub struct ThreadPool {
        size: usize,
        queue: Channel<Task>,
    }
}
```

### 2. Scheduler Design

```rust
// Work-stealing scheduler
pub struct WorkStealingScheduler {
    workers: Vec<Worker>,
    global_queue: ConcurrentDeque<Task>,
    victim_queue: AtomicUsize,
}

impl WorkStealingScheduler {
    pub fn schedule(&self, task: Task) {
        // Prefer local queue
        if let Ok(worker) = self.current_worker() {
            worker.local_queue.push_back(task);
        } else {
            // Add to global queue when no worker
            self.global_queue.push_back(task);
        }
    }

    pub fn steal(&self, victim: &Worker) -> Option<Task> {
        // Steal task from another worker's queue
        victim.local_queue.pop_back()
    }
}
```

### 3. Async Task Lifecycle

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

### 4. Error Handling Integration

```rust
// I/O error propagation
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

## Trade-offs

### Advantages

1. **Cross-platform consistency**: One API covers all major platforms
2. **High performance**: Event-driven + work-stealing, near hand-written async performance
3. **Transparent async**: Developers don't need to handle async details manually
4. **Blocking-safe**: Blocking operations auto-enter thread pool, don't block event loop
5. **Mature and stable**: libuv proven at scale by Node.js

### Disadvantages

1. **Dependency introduction**: Requires binding to libuv C library
2. **Windows compatibility**: Some APIs have slightly different behavior on Windows
3. **WASM support**: Requires additional adaptation work
4. **Debugging difficulty**: Async stack traces may be incomplete

## Alternative Solutions

| Solution | Why Not Chosen |
|----------|----------------|
| Implement event loop from scratch | Complex and error-prone, cannot match libuv maturity |
| Use mio | Only provides raw async primitives, lacks thread pool |
| Use async-std/tokio | Rust ecosystem, but YaoXiang needs its own runtime |
| Directly use libc epoll | Cannot cross-platform |

## Implementation Strategy

### Phase Division

1. **Phase 1 (v0.1)**: Basic libuv bindings, simple file I/O
2. **Phase 2 (v0.3)**: Network I/O, thread pool integration
3. **Phase 3 (v0.5)**: Advanced features, streaming API
4. **Phase 4 (v1.0)**: WASM adaptation, performance optimization

### Dependencies

- No external RFC dependencies
- **RFC-001 Concurrency Model**: Defines DAG scheduler; RFC-002 provides I/O abstraction

## Integration with RFC-001 Concurrency Model

RFC-001 defines the **DAG Scheduler** (scheduling layer), and RFC-002 defines **libuv + Thread Pool** (I/O layer). The two collaborate to implement "synchronous syntax, automatic concurrency".

### Layered Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    YaoXiang Runtime                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐    ┌─────────────────────┐        │
│  │   RFC-001: DAG      │    │  RFC-002: libuv     │        │
│  │   Scheduling Layer  │    │  I/O Layer          │        │
│  │                     │    │                     │        │
│  │  • Topological sort │    │  • Cross-platform I/O│       │
│  │  • Work stealing    │    │  • Event loop       │        │
│  │  • Dependency       │    │  • Thread pool      │        │
│  │    analysis         │    │                     │        │
│  └──────────┬──────────┘    └──────────┬──────────┘        │
│             │                         │                    │
│             │     ┌───────────────────┘                    │
│             │     │                                         │
│             ▼     ▼                                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Runtime Interface Layer                 │   │
│  │  • spawn/suspend/resume protocol                     │   │
│  │  • IO Completion callbacks                           │   │
│  │  • Task submission and wake-up                       │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Collaboration Flow

```markdown
1. **Compile time**: Resource type operations are identified as IO nodes
   - File.read, HTTP.get etc. are marked as "need async execution"
   - Create DAG node, mark as IO type

2. **Runtime**: DAG scheduler encounters IO node
   - Identified as non-compute node, submit to libuv
   - Scheduler continues executing other executable nodes

3. **IO complete**: libuv callback triggers
   - libuv thread pool completes blocking operation
   - Completion callback notifies DAG scheduler
   - Downstream nodes become executable
```

### Interface Protocol

```rust
// IO node interface defined by RFC-001
trait IoScheduler {
    // Submit IO task, return future/handle
    fn submit_io(&self, task: IoTask) -> IoHandle;

    // Called by libuv when IO completes, wake up DAG node
    fn on_io_complete(&self, handle: IoHandle);
}

// libuv integration implemented by RFC-002
impl IoScheduler for LibUvRuntime {
    fn submit_io(&self, task: IoTask) -> IoHandle {
        // 1. Submit task to libuv thread pool
        let handle = self.thread_pool.submit(|| {
            // Blocking execute actual IO
            let result = perform_blocking_io(&task);
            // 2. IO complete, call callback
            self.on_io_complete(handle);
        });
        handle
    }

    fn on_io_complete(&self, handle: IoHandle) {
        // Notify DAG scheduler to wake up downstream nodes
        self.dag_scheduler.wake_dependents(handle.node_id);
    }
}
```

### Transparent Async Mechanism

#### Compile-time Processing

```yaoxiang
# User code (synchronous syntax)
read_config: String -> Config = (path) => {
    content = File.read(path)  # Resource operation
    parse_yaml(content)
}

# Compile-time auto-conversion
# 1. Identify File.read as resource type operation
# 2. Create DAG node, mark as IO type
# 3. Add implicit await point
```

#### Runtime Processing

```markdown
| Step | Operation | Description |
|------|-----------|-------------|
| 1 | Parse DAG | Discover IO node |
| 2 | Submit IO | Add task to libuv thread pool |
| 3 | Continue scheduling | Execute other executable nodes |
| 4 | IO complete | libuv callback triggers |
| 5 | Wake up downstream | DAG scheduler resumes waiting node |
```

### Resource Type to IO Operation Mapping

```yaoxiang
# RFC-001 defines: Resource types
FilePath: Resource
HttpUrl: Resource

# RFC-002 implements: IO semantics for resource operations
File.read: (FilePath) -> String = path => {
    # Marked as IO operation, auto-enter libuv thread pool
}

HTTP.get: (HttpUrl) -> Response = url => {
    # Marked as IO operation, use libuv async network API
}
```

**Processing Rules**:
- Operations with resource type parameters → Marked as IO nodes
- IO nodes submitted to libuv thread pool for execution
- Completion callback wakes up DAG downstream nodes

### Risks

1. **libuv binding completeness**: Complete binding requires significant work
2. **Windows compatibility**: Some APIs need special handling
3. **Performance overhead**: FFI calls have some overhead
4. **Integration complexity**: Coordination between libuv thread pool and DAG scheduler requires careful design

## Open Questions

- [ ] Event loop adaptation strategy for WASM environment
- [ ] Cross-platform consistency of filesystem events
- [ ] Network I/O timeout mechanism design
- [ ] Boundaries for zero-copy optimization
- [ ] Cancellation operation semantics design
- [ ] Dynamic adjustment strategy for libuv thread pool size
- [ ] Coordination between IO node priority and compute node priority

## References

- [libuv Official Documentation](https://docs.libuv.org/)
- [Node.js Event Loop](https://nodejs.org/en/docs/guides/event-loop-timers-and-nexttick/)
- [Work-stealing Paper](https://ieftimov.com/posts/understanding-stealing-queues/)
- [Rust Async Runtime Design](https://smallcultfollowing.com/babysteps/blog/2019/08/22/async-await-simplified/)
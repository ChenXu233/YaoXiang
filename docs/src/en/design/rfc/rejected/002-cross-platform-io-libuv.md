---
title: 'RFC-002: Cross-Platform I/O with libuv Integration (Rejected)'
---

# RFC-002: Cross-Platform I/O and libuv Integration

> **Status**: Rejected
> **Author**: ChenXu
> **Created Date**: 2025-01-05
> **Last Updated**: 2026-02-15

## Rejection Reason

This RFC is rejected for the following reasons:

### 1. libuv is a C library and cannot be used after YaoXiang bootstraps

YaoXiang ultimately needs to bootstrap (implement the interpreter in YaoXiang itself),
at which point it cannot depend on C libraries. libuv as a C library requires FFI calls,
which would hinder the bootstrapping process.

### 2. tokio is a better choice

In the Rust ecosystem, tokio is the dominant async runtime (>90% market share). It is
pure Rust implementation and can continue to be used through bindings after bootstrapping,
which is more aligned with the long-term architecture than libuv.

### 3. Pragmatism Consideration

At the current stage, priority should be given to making YaoXiang language runnable.
I/O can be quickly implemented using Rust std. The actual async runtime can use tokio
bindings or be self-developed after bootstrapping.

---

## Summary

This RFC proposes a cross-platform asynchronous I/O solution for YaoXiang, integrating libuv to achieve a unified asynchronous abstraction. The core design goal is to automatically and transparently convert blocking I/O operations, freeing developers from worrying about low-level details.

## Motivation

### Why libuv is needed?

YaoXiang's concurrency model requires efficient asynchronous I/O support:

| Requirement | Problems with Traditional Solutions |
|-------------|-----------------------------------|
| Cross-platform I/O | Different platform APIs (Windows IOCP, Linux epoll, macOS kqueue) |
| Async event loop | Complex and error-prone to implement from scratch |
| Thread pool management | Blocking operations require dedicated thread pools |
| Performance requirements | Zero-overhead async abstraction needed |

### Advantages of libuv

```
libuv ✓ Mature and stable - Node.js underlying runtime, proven at scale
libuv ✓ Cross-platform - Unified I/O APIs for Windows, Linux, macOS
libuv ✓ High-performance - Event-driven, non-blocking I/O
libuv ✓ Thread pool - Built-in blocking operation thread pool management
```

## Proposal

### 1. Technology Selection Decisions

| Component | Choice | Reason |
|-----------|--------|--------|
| I/O runtime | libuv | Cross-platform mature, verified by Node.js |
| Event loop | libuv loop | Lightweight, efficient |
| Thread pool | libuv + custom | Blocking operations dedicated |
| Scheduling algorithm | Work stealing + DAG optimization | High performance, load balancing |
| Memory management | Ownership + stack allocation | No GC, zero-cost abstraction |

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
│          │   (Cross-platform I/O  │                        │
│          │    Abstraction)        │                        │
│          └───────────┬───────────┘                        │
│                      │                                    │
│          ┌───────────▼───────────┐                        │
│          │   Thread Pool         │  ← Blocking ops only   │
│          │   (libuv threads)     │                        │
│          └───────────────────────┘                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 2.2 Runtime Structure Definition

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
│  Blocking C function  →  Automatic Wrapper  →  Transparent  │
│                          Async[T]                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  // Original blocking API                                   │
│  data = File.read("file.txt")  // Blocking call             │
│                                                             │
│  // YaoXiang automatic conversion                          │
│  // 1. Detect blocking call                                │
│  // 2. Automatically submit to thread pool                 │
│  // 3. Return Async[T] proxy                              │
│  // 4. Automatically await result when used               │
│                                                             │
│  // Developer perspective                                   │
│  content = File.read("config.yaml")  // Async[String]       │
│  data = parse(content)               // Auto await         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 3.2 I/O Operation Examples

```yaoxiang
# Async file reading (developer perspective: sync syntax, auto async)
read_config: (String) -> Config spawn = (path) => {
    content = File.read(path)  # Auto async
    config = parse_yaml(content)
    config
}

# Async HTTP request
fetch_user: (Int) -> User spawn = (user_id) => {
    response = HTTP.get("/users/" + user_id.to_string())
    parse_user(response.body())
}

# Concurrent file processing
process_files: ([String]) -> [Result[FileData, Error]] = (paths) => {
    # Automatically parallel read all files
    data = paths.map(path => {
        File.read(path)  # spawn auto-inserted
    })
    data.map(d => process_content(d))
}

# Streaming processing (gradual reading)
stream_large_file: (String) -> Void = (path) => {
    stream = File.open_stream(path)
    for chunk in stream.chunks(8192) {  # Auto async iteration
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
    server.serve(router)  # Auto handle concurrent requests
}

# WebSocket
chat_server: (String) -> Void spawn = (port) => {
    ws = WebSocket.new("ws://localhost:" + port.to_string())
    for message in ws.incoming() {  # Auto streaming processing
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
    # Same API on all platforms
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
# Script header configuration for thread pool size
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
    # libuv batch submission, reduce context switching
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

    // Basic types
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
        // Prefer adding to local queue
        if let Ok(worker) = self.current_worker() {
            worker.local_queue.push_back(task);
        } else {
            // Add to global queue when no worker available
            self.global_queue.push_back(task);
        }
    }

    pub fn steal(&self, victim: &Worker) -> Option<Task> {
        // Steal tasks from other workers' queues
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

1. **Cross-platform consistency**: Same API covers all major platforms
2. **High performance**: Event-driven + work-stealing, close to hand-written async
3. **Transparent async**: Developers don't need to manually handle async details
4. **Blocking safety**: Blocking operations automatically enter thread pool, don't block event loop
5. **Mature and stable**: libuv verified at Node.js scale

### Disadvantages

1. **Dependency introduction**: Need to bind libuv C library
2. **Windows compatibility**: Some APIs have slightly different behavior on Windows
3. **WASM support**: Requires additional adaptation work
4. **Debugging difficulty**: Async stack traces may be incomplete

## Alternative Solutions

| Solution | Why Not Chosen |
|----------|----------------|
| Implement event loop from scratch | Complex and error-prone, can't match libuv maturity |
| Use mio | Only provides raw async primitives, lacks thread pool |
| Use async-std/tokio | Rust ecosystem, but YaoXiang needs its own runtime |
| Use libc epoll directly | Not cross-platform |

## Implementation Strategy

### Phase Division

1. **Phase 1 (v0.1)**: Basic libuv bindings, simple file I/O
2. **Phase 2 (v0.3)**: Network I/O, thread pool integration
3. **Phase 3 (v0.5)**: Advanced features, streaming API
4. **Phase 4 (v1.0)**: WASM adaptation, performance optimization

### Dependencies

- No external RFC dependencies
- **RFC-001 Concurrency Model**: Defines DAG scheduler, RFC-002 provides IO abstraction

## Integration with RFC-001 Concurrency Model

RFC-001 defines **DAG Scheduler** (scheduling layer), RFC-002 defines **libuv + Thread Pool** (IO layer). Both collaborate to achieve "sync syntax, auto concurrent".

### Layered Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    YaoXiang Runtime                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐    ┌─────────────────────┐        │
│  │   RFC-001: DAG      │    │  RFC-002: libuv     │        │
│  │   Scheduling Layer  │    │  IO Layer           │        │
│  │                     │    │                     │        │
│  │  • Topological sort│    │  • Cross-platform  │        │
│  │    scheduling       │    │    I/O             │        │
│  │  • Work stealing   │    │  • Event loop      │        │
│  │  • Dependency      │    │  • Thread pool     │        │
│  │    analysis         │    │                     │        │
│  └──────────┬──────────┘    └──────────┬──────────┘        │
│             │                         │                    │
│             │     ┌───────────────────┘                    │
│             │     │                                         │
│             ▼     ▼                                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Runtime Interface Layer                 │   │
│  │  • spawn/suspend/resume protocol                     │   │
│  │  • IO Completion callbacks                          │   │
│  │  • Task submission and wakeup                        │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Collaboration Flow

```markdown
1. **Compile-time**: Resource type operations identified as IO nodes
   - File.read, HTTP.get marked as "needs async execution"
   - Create DAG nodes, marked as IO type

2. **Runtime**: DAG scheduler encounters IO node
   - Identified as non-computing node, submitted to libuv
   - Scheduler continues executing other executable nodes

3. **IO Complete**: libuv callback triggers
   - libuv thread pool completes blocking operation
   - Completion callback notifies DAG scheduler
   - Downstream nodes become executable
```

### Interface Protocol

```rust
// RFC-001 defined IO node interface
trait IoScheduler {
    // Submit IO task, return future/handle
    fn submit_io(&self, task: IoTask) -> IoHandle;

    // Called by libuv when IO complete, wake DAG node
    fn on_io_complete(&self, handle: IoHandle);
}

// RFC-002 implemented libuv integration
impl IoScheduler for LibUvRuntime {
    fn submit_io(&self, task: IoTask) -> IoHandle {
        // 1. Submit task to libuv thread pool
        let handle = self.thread_pool.submit(|| {
            // Execute actual IO blocking
            let result = perform_blocking_io(&task);
            // 2. IO complete, call callback
            self.on_io_complete(handle);
        });
        handle
    }

    fn on_io_complete(&self, handle: IoHandle) {
        // Notify DAG scheduler to wake downstream nodes
        self.dag_scheduler.wake_dependents(handle.node_id);
    }
}
```

### Transparent Async Mechanism

#### Compile-time Processing

```yaoxiang
# User code (sync syntax)
read_config: String -> Config = (path) => {
    content = File.read(path)  # Resource operation
    parse_yaml(content)
}

# Compile-time automatic conversion
# 1. Recognize File.read as resource type operation
# 2. Create DAG node, mark as IO type
# 3. Add implicit await points
```

#### Runtime Processing

```markdown
| Step | Operation | Description |
|------|-----------|-------------|
| 1 | Parse DAG | Find IO nodes |
| 2 | Submit IO | Add task to libuv thread pool |
| 3 | Continue scheduling | Execute other executable nodes |
| 4 | IO complete | libuv callback triggers |
| 5 | Wake downstream | DAG scheduler resumes waiting nodes |
```

### Resource Type to IO Operation Mapping

```yaoxiang
# RFC-001 defined: Resource types
FilePath: Resource
HttpUrl: Resource

# RFC-002 implemented: IO semantics for resource operations
File.read: (FilePath) -> String = path => {
    # Marked as IO operation, automatically enters libuv thread pool
}

HTTP.get: (HttpUrl) -> Response = url => {
    # Marked as IO operation, uses libuv async network API
}
```

**Processing Rules**:
- Operations with resource type parameters → Marked as IO nodes
- IO nodes submitted to libuv thread pool for execution
- Completion callback wakes DAG downstream nodes

### Risks

1. **libuv binding completeness**: Complete bindings require significant work
2. **Windows compatibility**: Some APIs need special handling
3. **Performance overhead**: FFI calls have some overhead
4. **Integration complexity**: Coordination between libuv thread pool and DAG scheduler needs careful design

## Open Questions

- [ ] Event loop adaptation scheme for WASM environment
- [ ] Cross-platform consistency of file system events
- [ ] Timeout mechanism design for network I/O
- [ ] Boundaries of zero-copy optimization
- [ ] Cancellation semantics design
- [ ] Dynamic adjustment strategy for libuv thread pool size
- [ ] Coordination between IO node priority and compute node priority

## References

- [libuv Official Documentation](https://docs.libuv.org/)
- [Node.js Event Loop](https://nodejs.org/en/docs/guides/event-loop-timers-and-nexttick/)
- [Work Stealing Paper](https://ieftimov.com/posts/understanding-stealing-queues/)
- [Rust Async Runtime Design](https://smallcultfollowing.com/babysteps/blog/2019/08/22/async-await-simplified/)

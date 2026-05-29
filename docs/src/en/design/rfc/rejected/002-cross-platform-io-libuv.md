```markdown
---
title: RFC-002: Cross-Platform I/O and libuv Integration (Rejected)
---

# RFC-002: Cross-Platform I/O and libuv Integration

> **Status**: Rejected
> **Author**: Chen Xu
> **Created**: 2025-01-05
> **Last Updated**: 2026-02-15

## Rejection Reasons

This RFC was rejected for the following reasons:

### 1. libuv is a C library and cannot be used after YaoXiang self-hosts

YaoXiang ultimately needs to bootstrap (implement an interpreter in YaoXiang itself). At this point, it cannot depend on C libraries.
libuv, as a C library, requires FFI calls, which would hinder the bootstrap process.

### 2. tokio is a more suitable choice

In the Rust ecosystem, tokio is the dominant async runtime (market share >90%). It is a pure Rust implementation,
and after bootstrapping, it can continue to be used through bindings, making it more aligned with the long-term architecture than libuv.

### 3. Pragmatic considerations

At the current stage, priority should be given to getting the YaoXiang language to run. I/O can be quickly implemented using Rust std,
and a true async runtime can be developed after bootstrapping using tokio bindings or self-developed.

---

## Summary

This proposal introduces YaoXiang's cross-platform async I/O solution, integrating libuv to implement a unified async abstraction. The core design goal is to automatically and transparently asyncify blocking I/O operations, allowing developers to ignore bottom-layer details.

## Motivation

### Why libuv is needed?

YaoXiang's spawn model requires efficient async I/O support:

| Requirement | Problems with traditional approaches |
| ----------- | --------------------------------------|
| Cross-platform I/O | Platform APIs are not unified (Windows IOCP, Linux epoll, macOS kqueue) |
| Async event loop | Implementing from scratch is complex and error-prone |
| Thread pool management | Blocking operations require dedicated thread pools |
| Performance requirements | Zero-overhead async abstractions are needed |

### libuv advantages

```
libuv ✓ Mature and stable - Node.js bottom-layer runtime, validated at scale
libuv ✓ Cross-platform - Unified I/O API for Windows, Linux, macOS
libuv ✓ High performance - Event-driven, non-blocking I/O
libuv ✓ Thread pools - Built-in blocking operation thread pool management
```

## Proposal

### 1. Technical selection decision

| Component | Selection | Reason |
| --------- | --------- | ------ |
| I/O runtime | libuv | Cross-platform mature, Node.js validated |
| Event loop | libuv loop | Lightweight, efficient |
| Thread pool | libuv + custom | Blocking operations dedicated |
| Scheduling algorithm | Work stealing + DAG optimization | High performance, load balancing |
| Memory management | ownership + stack allocation | No GC, zero-cost abstractions |

### 2. Architecture design

#### 2.1 Overall runtime structure

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
│          │   (Cross-platform I/O abstraction) │        │
│          └───────────┬───────────┘                        │
│                      │                                    │
│          ┌───────────▼───────────┐                        │
│          │   Thread Pool         │  ← Blocking ops dedicated │
│          │   (libuv threads)     │                        │
│          └───────────────────────┘                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 2.2 Runtime structure definitions

```rust
struct YaoXiangRuntime {
    // libuv event loop (cross-platform I/O core)
    uv_loop: *mut uv_loop_t,

    // Work-stealing scheduler
    scheduler: WorkStealingScheduler,

    // Blocking operations thread pool
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

### 3. Unified async abstraction

#### 3.1 Blocking-to-transparent conversion

```
┌─────────────────────────────────────────────────────────────┐
│  Blocking C function  →  Auto-wrap  →  Transparent Async[T] │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  // Raw blocking API                                        │
│  data = File.read("file.txt")  // Blocking call            │
│                                                             │
│  // YaoXiang auto-conversion                                │
│  // 1. Detect blocking call                                  │
│  // 2. Auto-submit to thread pool                           │
│  // 3. Return Async[T] proxy                                │
│  // 4. Auto-wait for result on use                         │
│                                                             │
│  // Developer perspective                                   │
│  content = File.read("config.yaml")  // Async[String]       │
│  data = parse(content)               // Auto-wait          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 3.2 I/O operation examples

```yaoxiang
# Async file read (developer perspective: sync syntax, auto-async)
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
    for message in ws.incoming() {  # Auto-streaming processing
        broadcast(message)
    }
}
```

### 4. Cross-platform guarantees

#### 4.1 Platform support matrix

| Platform | Status | Event mechanism | Notes |
| --------- | ------ | --------------- | ----- |
| **Linux** | ✅ Supported | epoll | Primary development platform |
| **macOS** | ✅ Supported | kqueue | libuv native support |
| **Windows** | ✅ Supported | IOCP | libuv native support |
| **WASM** | ⚠️ TBD | Browser API | Requires additional adaptation |
| **WASI** | ⚠️ TBD | WASI calls | Long-term goal |

#### 4.2 Cross-platform API unification

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

#### 4.3 Platform-specific optimizations

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

### 5. Performance considerations

#### 5.1 Thread pool configuration

```yaoxiang
# Script header configures thread pool size
# @thread_pool: 4

# Or runtime configuration
configure_runtime: () -> Void = () => {
    Runtime.set_thread_pool_size(8)
    Runtime.set_max_concurrent_tasks(100)
}
```

#### 5.2 I/O batch optimization

```yaoxiang
# Batch file operations (reduce system calls)
batch_read: ([String]) -> [String] = (paths) => {
    # libuv batch submission, reduce context switches
    File.batch_read(paths)
}

# Zero-copy optimization
zero_copy_transfer: (String, String) -> Void = (src, dst) => {
    # Use sendfile/splice on supported platforms
    File.transfer(src, dst)
}
```

## Detailed design

### 1. Rust binding structure

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

### 2. Scheduler design

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
            // No worker, add to global queue
            self.global_queue.push_back(task);
        }
    }

    pub fn steal(&self, victim: &Worker) -> Option<Task> {
        // Steal tasks from other workers' queues
        victim.local_queue.pop_back()
    }
}
```

### 3. Async task lifecycle

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

### 4. Error handling integration

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
2. **High performance**: Event-driven + work-stealing, close to hand-written async performance
3. **Transparent async**: Developers don't need to manually handle async details
4. **Blocking-safe**: Blocking operations automatically enter thread pool, don't block event loop
5. **Mature and stable**: libuv validated at scale by Node.js

### Disadvantages

1. **Dependency introduction**: Need to bind libuv C library
2. **Windows compatibility**: Some APIs have slightly different behavior on Windows
3. **WASM support**: Requires additional adaptation work
4. **Debugging difficulty**: Async stack traces may be incomplete

## Alternative approaches

| Approach | Why not chosen |
| -------- | -------------- |
| Implement event loop from scratch | Complex and error-prone, cannot match libuv maturity |
| Use mio | Only provides raw async primitives, lacks thread pool |
| Use async-std/tokio | Rust ecosystem, but YaoXiang needs its own runtime |
| Direct use of libc epoll | Cannot cross platforms |

## Implementation strategy

### Phase breakdown

1. **Phase 1 (v0.1)**: Basic libuv bindings, simple file I/O
2. **Phase 2 (v0.3)**: Network I/O, thread pool integration
3. **Phase 3 (v0.5)**: Advanced features, streaming API
4. **Phase 4 (v1.0)**: WASM adaptation, performance optimization

### Dependencies

- No external RFC dependencies
- **RFC-001 Concurrency Model**: Defines DAG scheduler, RFC-002 provides I/O abstraction

## Integration with RFC-001 Concurrency Model

RFC-001 defines the **DAG scheduler** (scheduling layer), and RFC-002 defines **libuv + thread pool** (I/O layer). The two collaborate to implement "sync syntax, auto-concurrent".

### Layered architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    YaoXiang Runtime                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐    ┌─────────────────────┐        │
│  │   RFC-001: DAG      │    │  RFC-002: libuv     │        │
│  │   Scheduling Layer  │    │  I/O Layer          │        │
│  │                     │    │                     │        │
│  │  • Topological sort scheduling │  • Cross-platform I/O │  │
│  │  • Work stealing    │    │  • Event loop       │        │
│  │  • Dependency analysis │   │  • Thread pool     │        │
│  └──────────┬──────────┘    └──────────┬──────────┘        │
│             │                         │                    │
│             │     ┌───────────────────┘                    │
│             │     │                                         │
│             ▼     ▼                                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Runtime Interface Layer                 │   │
│  │  • spawn/suspend/resume protocol                     │   │
│  │  • IO Completion callbacks                            │   │
│  │  • Task submission and wakeup                         │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Collaboration flow

```markdown
1. **Compile-time**: Resource type operations are identified as IO nodes
   - File.read, HTTP.get etc. are marked as "requires async execution"
   - DAG nodes are created, marked as IO type

2. **Runtime**: DAG scheduler encounters IO node
   - Recognized as non-compute node, submitted to libuv
   - Scheduler continues executing other executable nodes

3. **IO Completion**: libuv callback triggers
   - libuv thread pool completes blocking operation
   - Completion callback notifies DAG scheduler
   - Downstream nodes become executable
```

### Interface protocol

```rust
// IO node interface defined by RFC-001
trait IoScheduler {
    // Submit IO task, return future/handle
    fn submit_io(&self, task: IoTask) -> IoHandle;

    // Called by libuv when IO completes, wake DAG node
    fn on_io_complete(&self, handle: IoHandle);
}

// libuv integration implemented by RFC-002
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

### Transparent async mechanism

#### Compile-time processing

```yaoxiang
# User code (sync syntax)
read_config: String -> Config = (path) => {
    content = File.read(path)  # Resource operation
    parse_yaml(content)
}

# Compile-time auto-conversion
# 1. Identify File.read as resource type operation
# 2. Create DAG node, mark as IO type
# 3. Add implicit await points
```

#### Runtime processing

```markdown
| Step | Operation | Description |
|------| --------- | ----------- |
| 1 | Parse DAG | Discover IO nodes |
| 2 | Submit IO | Add tasks to libuv thread pool |
| 3 | Continue scheduling | Execute other executable nodes |
| 4 | IO complete | libuv callback triggers |
| 5 | Wake downstream | DAG scheduler resumes waiting nodes |
```

### Resource type to IO operation mapping

```yaoxiang
# RFC-001 defines: Resource types
FilePath: Resource
HttpUrl: Resource

# RFC-002 implements: IO semantics for resource operations
File.read: (FilePath) -> String = path => {
    # Mark as IO operation, auto-enter libuv thread pool
}

HTTP.get: (HttpUrl) -> Response = url => {
    # Mark as IO operation, use libuv async network API
}
```

**Processing rules**:
- Operations with resource type parameters → Mark as IO nodes
- IO nodes submitted to libuv thread pool for execution
- Completion callbacks wake DAG downstream nodes

### Risks

1. **libuv binding completeness**: Full bindings require significant work
2. **Windows compatibility**: Some APIs need special handling
3. **Performance overhead**: FFI calls have some overhead
4. **Integration complexity**: Coordination between libuv thread pool and DAG scheduler requires careful design

## Open questions

- [ ] Event loop adaptation plan for WASM environment
- [ ] Cross-platform consistency of filesystem events
- [ ] Timeout mechanism design for network I/O
- [ ] Boundaries of zero-copy optimization
- [ ] Semantic design for cancellation operations
- [ ] Dynamic adjustment strategy for libuv thread pool size
- [ ] Coordination between IO node priority and compute node priority

## References

- [libuv Official Documentation](https://docs.libuv.org/)
- [Node.js Event Loop](https://nodejs.org/en/docs/guides/event-loop-timers-and-nexttick/)
- [Work Stealing Paper](https://ieftimov.com/posts/understanding-stealing-queues/)
- [Rust Async Runtime Design](https://smallcultfollowing.com/babysteps/blog/2019/08/22/async-await-simplified/)
```
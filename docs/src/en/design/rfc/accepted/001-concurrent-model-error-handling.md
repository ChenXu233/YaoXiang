---
title: 'RFC-001: Concurrency Model and Error Handling System'
---

# RFC-001: Concurrency Model and Error Handling System

> **Status**: Accepted
> **Author**: ChenXu
> **Created Date**: 2025-01-05
> **Last Updated**: 2026-02-12

## Design Source and Reference

This document's design is based on the following documents and serves as the detailed design source for language-spec:

| Document | Relationship | Description |
|----------|--------------|-------------|
| [async-whitepaper](../async-whitepaper.md) | **Design Source** | Theoretical foundation and core concepts of the concurrency model |
| [language-spec](../language-spec.md) | **Specification Target** | This RFC's design will be integrated into the language specification |

> **Description**: This RFC is the concretization and standardization of the concurrency model proposed in [async-whitepaper](../async-whitepaper.md), transforming it into implementable language design.

## Summary

Proposing YaoXiang's concurrency model (three-layer concurrency architecture), side effect handling mechanism, DAG dependency analysis, Result type system, and error graph visualization. Core design philosophy originates from "All things grow together, and I observe their return" in the Book of Changes — describing logic with synchronous syntax, runtime automatically executes concurrently.

## Quick Reference

| Scenario | Syntax | Description |
|----------|--------|-------------|
| Automatic Parallelism | No annotation | Default behavior, maximum parallelism |
| Explicit Parallel Declaration | `@auto` | Same as default behavior, readability |
| Synchronous Wait | `@eager` | Force wait for dependencies to complete |
| Fully Sequential | `@block` | No concurrency, for debugging |
| Local Concurrency | `spawn` | Concurrency within @block scope |

## Motivation

### Why is a concurrency model needed?

Current mainstream language concurrency models have obvious flaws:

| Language | Concurrency Model | Problem |
|----------|------------------|---------|
| Rust | async/await + tokio | Async contagion, steep learning curve |
| Go | goroutine | No type safety, difficult escape analysis |
| Python | asyncio | GIL limitation, poor performance |
| JavaScript | Promise/async | Callback hell somewhat mitigated but still complex |

### Core Contradictions

1. **Transparency vs Controllability**: Completely transparent but uncontrollable vs completely controllable but opaque
2. **Concurrency vs Debugging**: Concurrent programs are hard to debug vs debuggable programs are hard to concurrent
3. **Safety vs Performance**: GC safety has overhead vs manual management is efficient but dangerous

### Concurrency Model Solution Approach

```
Traditional Dilemma ─────────── Concurrency Model ─────────── Solution
Completely transparent but uncontrollable ───→ Three-layer model ───→ Progressive adoption from L1 to L3
Concurrent programs hard to debug ───→ Graph debugging ───→ Error graph visualization
GC safety but with overhead ───→ Ownership ───→ Rust-style compile-time checking
```

## Proposal

### 1. Concurrency Model: Three-Layer Concurrency Architecture

The concurrency model allows developers to describe logic in synchronous, sequential thinking, while the language runtime makes computational units automatically and efficiently execute concurrently like all things growing together.

#### Three-Layer Abstraction

> **Description**: L1/L2/L3 are **mental models** to help users understand concurrency behavior in different scenarios. Actual implementation has only one mechanism: resource types + automatic DAG analysis + @block/@eager/@auto annotation control.

| Level | Mental Model | Syntax | Execution Mode | Parallelism | Applicable Scenario |
|-------|--------------|--------|---------------|-------------|-------------------|
| **L1** | Disable Concurrency | `@block` | No DAG, pure sequential execution | ❌ None | Debugging, beginners, critical code sections |
| **L2** | Partial Concurrency | `spawn` | Developer-controlled DAG | ⚠️ Partial | Intermediate users, need controlled concurrency |
| **L3** | Full Concurrency | Default | Automatic DAG analysis | ✅ Full | Experts, automatic parallelism optimization |

#### L1: `@block` Synchronous Mode

```yaoxiang
# L1: @block synchronous mode (no DAG, pure sequential execution)
fetch_sync: (String) -> JSON @block = (url) => {
    HTTP.get(url).json()
}

main: () -> Void @block = () => {
    # Strictly sequential execution, no concurrency at all
    data1 = fetch_sync("https://api.example.com/data1")
    data2 = fetch_sync("https://api.example.com/data2")
    process(data1, data2)
}
```

#### L2: Explicit spawn Concurrency

```yaoxiang
# L2: Explicit spawn concurrency
fetch_data: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

process_users_and_posts: () -> Void spawn = () => {
    users = fetch_data("https://api.example.com/users")
    posts = fetch_data("https://api.example.com/posts")
    # users and posts execute automatically in parallel
    print("Users: " + users.length.to_string())
    print("Posts: " + posts.length.to_string())
}
```

#### L3: Fully Transparent (Default)

```yaoxiang
# L3: Fully transparent (default mode)
heavy_calc: (Int) -> Int = (n) => {
    fibonacci(n)
}

auto_parallel: (Int) -> Int = (n) => {
    a = heavy_calc(1)
    b = heavy_calc(2)
    c = heavy_calc(3)
    a + b + c  # Wait for all results here
}
```

### 2. Three Annotations Complete Comparison

| Dimension | `@auto` (Default) | `@eager` | `@block` |
|-----------|---------------------|----------|----------|
| **spawn Execution** | Asynchronous, scheduler responds | Asynchronous + suspend and wait | Force synchronous execution |
| **Normal Call** | Asynchronous | Synchronous | Synchronous |
| **Parallelism** | ✅ Full | ⚠️ Partial | ❌ None |
| **Scheduler Participation** | ✅ Full participation | ✅ Full participation | ❌ No participation |
| **DAG Construction** | ✅ Construct | ✅ Construct | ❌ Not construct |

**Selection Guide**:
- Maximum concurrency optimization → `@auto` (default)
- Need ordered side effects → `@eager`
- Debugging/beginners/critical code → `@block`

```yaoxiang
# @auto (default): Maximum parallelism
heavy_calc: (Int) -> Int = (n) => {
    fibonacci(n)  # Default automatic parallelism
}
```

> **Note**: The `@auto` annotation explicitly indicates automatic parallelism, making code intent clearer. Its behavior is identical to no annotation, only for readability improvement.

### 3. Error Handling: Result Type System

#### 3.1 Result Type Definition

```yaoxiang
# Standard Result type definition
Result: Type[T, E] = {
    ok: (T) -> Self,
    err: (E) -> Self,
    is_ok: (Self) -> Bool,
    is_err: (Self) -> Bool,
    unwrap: (Self) -> T,
    unwrap_err: (Self) -> E,
    map: [U]((T) -> U) -> Result[U, E],
    map_err: [F]((E) -> F) -> Result[T, F],
}
```

#### 3.2 Error Propagation Operator `?`

```yaoxiang
# Error propagation with ?
process_data: (String) -> Result[Data, Error] = (input) => {
    parsed = parse(input)?          # Automatically propagate if error
    validated = validate(parsed)?   # Continue propagating if error
    transform(validated)?           # Continue propagating if error
}
```

#### 3.3 Pattern Matching Error Handling

```yaoxiang
# Explicit error handling with match
result: Result[Int, String] = divide(10, 2)

match result {
    ok(value) => print("Success: " + value.to_string()),
    err(error) => print("Error: " + error),
}
```

### 4. Side Effect Handling

#### 4.1 Side Effect Classification

| Category | Example | Handling Strategy |
|----------|---------|-------------------|
| **Pure Computation** | `fibonacci(n)` | Fully parallelizable |
| **IO Operations** | `HTTP.get(url)` | Spawn, DAG dependent |
| **State Mutation** | `counter += 1` | Require Sync constraint |
| **System Calls** | `file.write()` | Spawn, DAG dependent |

#### 4.2 Side Effect Annotation

```yaoxiang
# Mark side effects explicitly
@side_effect(io)
fetch_user: (Int) -> User spawn = (id) => {
    database.query("SELECT * FROM users WHERE id = " + id.to_string())
}

@side_effect(mutating)
increment: (ref Int) -> Void = (counter) => {
    counter.value = counter.value + 1
}
```

### 5. Resource Type Design

#### 5.1 Send Constraint

Types that can safely be transferred across threads:

```yaoxiang
# Send constraint definition
Send: Type = {
    # Empty marker interface
}

# Derive Send automatically
# All primitive types: Int, Float, Bool, String, etc.
# Immutable structs where all fields are Send
# Result[T, E] where T: Send and E: Send
# Arc[T] where T: Send
```

#### 5.2 Sync Constraint

Types that can be safely shared across threads through references:

```yaoxiang
# Sync constraint definition
Sync: Type = {
    # Empty marker interface
}

# Derive Sync automatically
# Types where &T is Send
# Immutable types are Sync by default
# Mutex[T] provides interior mutability while remaining Sync
```

#### 5.3 Send + Sync Hierarchy

```
Send ──► Can safely transfer across threads
  │
  └──► Sync ──► Can safely share across threads
       │
       └──► Types satisfying Send + Sync can automatically be concurrent
```

### 6. Concurrency Model and Ownership Integration

#### 6.1 Ownership in Concurrent Context

```yaoxiang
# Default Move semantics
process_data: (Data) -> Result[Output, Error] = (data) => {
    # data ownership transferred in
    # Original binding invalid after call
}

# Shared reference with Arc
shared_data: ref Data = ref large_data
spawn(() => process(shared_data))   # ✅ Safe: Arc is Send + Sync
```

#### 6.2 Data Race Prevention

```yaoxiang
# Send constraint prevents data races at compile time
SafeCounter: Type = { value: Int }

# Arc provides thread-safe shared ownership
counter: ref SafeCounter = ref SafeCounter(Mutex.new(0))

spawn(() => {
    guard = counter.value.lock()
    guard.value = guard.value + 1
})
```

### 7. Concurrency Debugging: Error Graph Visualization

#### 7.1 Error Graph Concept

Error graph is a directed acyclic graph (DAG) representing task dependencies and error propagation paths.

#### 7.2 Error Graph Generation

```yaoxiang
# Enable error graph generation
@concurrency(debug: true)
process: (String) -> Result[Output, Error] = (input) => {
    # ... processing logic
}
```

#### 7.3 Error Graph Visualization Output

```
Error Graph for process("input"):
┌─────────────────────────────────────────────────────────────┐
│ nodes:                                                     │
│   [A] parse(input)                    → Result[Data, E1] │
│   [B] validate(data)                   → Result[Data, E2] │
│   [C] transform(validated)              → Result[Output, E3]│
│                                                             │
│ edges:                                                      │
│   A ──► B (B depends on A result)                          │
│   B ──► C (C depends on B result)                           │
│                                                             │
│ error_propagation:                                          │
│   If A fails → B and C not executed                        │
│   If B fails → C not executed, error contains A's error    │
└─────────────────────────────────────────────────────────────┘
```

### 8. Scheduler Design

#### 8.1 Scheduler Responsibilities

| Responsibility | Description |
|---------------|-------------|
| **Task Scheduling** | Manage ready queue, execute tasks |
| **Work Stealing** | Idle threads steal tasks from busy threads |
| **Dependency Resolution** | Build and manage DAG |
| **Error Propagation** | Propagate errors through dependency chain |

#### 8.2 Scheduler Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Scheduler Core                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │ Work Queue  │  │ Wait Queue  │  │ Dependency Analyzer │ │
│  │ (Per Thread)│  │ (Global)    │  │ (DAG Builder)      │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
│                                                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │ Work Stealer│  │ Error       │  │ Performance         │ │
│  │ (Balancing) │  │ Handler     │  │ Profiler           │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### 9. Runtime Design

#### 9.1 Runtime Components

| Component | Responsibility |
|-----------|---------------|
| **Task Descriptor** | Describe task metadata, dependencies |
| **DAG Builder** | Build dependency graph at runtime |
| **Scheduler** | Schedule and execute tasks |
| **Work Stealing Queue** | Load balancing between threads |
| **Error Propagation** | Propagate errors through DAG |
| **Memory Allocator** | Support concurrent allocation |

#### 9.2 Task Lifecycle

```
Created ──► Ready ──► Running ──► Waiting ──► Ready ──► Completed
              │              │            │
              │              ▼            │
              │         Blocked          │
              │              │            │
              └──────────────┴────────────┘
                        (by dependency)
```

### 10. Type System Integration

#### 10.1 Async Function Type

```yaoxiang
# Async function type representation
Async: Type[T] = {
    # Opaque type representing ongoing computation
    # Can be awaited when result needed
}
```

#### 10.2 is_async Flag

```yaoxiang
# Type system tracks async capability
Function: Type = {
    params: TypeList,
    return_type: Type,
    is_async: Bool,    # Whether function can spawn
    send_constraint: Bool,
    sync_constraint: Bool,
}
```

### 11. Exception Handling

#### 11.1 Exception Types

```yaoxiang
# Exception hierarchy
Exception: Type = {
    RuntimeError,
    TypeError,
    ValueError,
    IOError,
    TimeoutError,
}

# Exception as error variant
Result: Type[T, E] = {
    ok: (T) -> Self,
    err: (E) -> Self,    # E can be Exception or custom error
}
```

#### 11.2 Exception Propagation

```yaoxiang
# Exceptions as special error values
catch_exception: (() -> T) -> Result[T, Exception] = (f) => {
    try {
        ok(f())
    } catch (e: Exception) {
        err(e)
    }
}
```

### 12. Compiler Implementation Requirements

#### 12.1 Lexer Additions

| Token | Description |
|-------|-------------|
| `@block` | Synchronous execution annotation |
| `@eager` | Eager evaluation annotation |
| `@auto` | Auto parallelism annotation (default) |
| `spawn` | Concurrent execution keyword |

#### 12.2 Parser Additions

| Grammar Rule | Description |
|--------------|-------------|
| `Annotation` | Parse annotation syntax |
| `SpawnFn` | Parse spawn function definition |
| `SpawnBlock` | Parse concurrent block |
| `SpawnFor` | Parse concurrent for loop |

#### 12.3 Type Checker Additions

| Check | Description |
|-------|-------------|
| `send_check` | Verify Send constraint |
| `sync_check` | Verify Sync constraint |
| `async_signature` | Verify async function signatures |
| `error_propagation` | Verify error propagation paths |

#### 12.4 IR Additions

| Instruction | Description |
|-------------|-------------|
| `CallAsync` | Async function call |
| `Await` | Wait for async result |
| `Spawn` | Create new task |
| `Sync` | Synchronization barrier |

#### 12.5 Code Generator Additions

| Generation | Description |
|------------|-------------|
| `async_codegen` | Generate async function code |
| `dag_builder_codegen` | Generate DAG building code |
| `error_propagation_codegen` | Generate error propagation code |
| `scheduler_codegen` | Generate scheduler interaction code |

### 13. Implementation Phases

#### Phase 1: Core Infrastructure (v0.3)

| Component | Deliverables |
|-----------|--------------|
| Lexer/Parser | Support @block/@eager/@auto, spawn syntax |
| Type System | Send/Sync constraint checking |
| IR | CallAsync, Await instructions |
| Code Generation | Basic async function calls |

#### Phase 2: Scheduler and Runtime (v0.4)

| Component | Deliverables |
|-----------|--------------|
| Scheduler | Work queue, basic scheduling |
| Runtime | Task descriptor, DAG builder |
| Memory | Concurrent memory allocator |
| Integration | Scheduler integration |

#### Phase 3: Advanced Features (v0.5)

| Component | Deliverables |
|-----------|--------------|
| Work Stealing | Load balancing between threads |
| Error Graph | Error propagation and visualization |
| Profiling | Performance profiling tools |
| Optimization | Advanced scheduling optimization |

### 14. Backward Compatibility

#### Compatibility Analysis

| Feature | Backward Compatible | Migration Path |
|---------|---------------------|----------------|
| `@auto` (default) | ✅ Yes | No changes needed |
| `@eager` | ✅ Yes | No changes needed |
| `@block` | ✅ Yes | No changes needed |
| `spawn` | ✅ Yes | No changes needed |
| Result type | ✅ Yes | Optional use |
| Error propagation | ✅ Yes | Optional use |

### 15. Migration Strategy

#### Gradual Adoption

1. **v0.3**: Core infrastructure, opt-in usage
2. **v0.4**: Scheduler complete, default for new code
3. **v0.5**: Full optimization, complete migration

#### Migration Guidelines

```yaoxiang
# Old synchronous code
process: (String) -> JSON = (url) => {
    HTTP.get(url).json()
}

# New concurrent code (gradual migration)
process: (String) -> JSON @auto = (url) => {
    HTTP.get(url).json()
}
```

### 16. Known Limitations

| Limitation | Description | Solution |
|------------|-------------|----------|
| Complex DAG Analysis | Deeply nested dependencies | Limit DAG depth |
| Error Graph Overhead | Visualization has performance cost | Debug-only mode |
| Scheduler Complexity | Full work stealing is complex | Simplified scheduling first |
| FFI Integration | Foreign function interface | Future work |

### 17. References

- [async-whitepaper](../async-whitepaper.md) - Theoretical foundation
- [language-spec](../language-spec.md) - Language specification
- [Rust async book](https://rust-lang.github.io/async-book/) - Reference for async design
- [Go scheduler](https://golang.org/src/runtime/proc.go) - Reference for work stealing

---

## Appendix A: Design Decision Records

| Decision | Decision | Date | Recorder |
|----------|----------|------|----------|
| Three-layer model | L1/L2/L3 mental model with unified mechanism | 2025-01-05 | ChenXu |
| Annotation names | Use @block/@eager/@auto | 2025-01-05 | ChenXu |
| Result type naming | Use ok/err variants | 2025-01-05 | ChenXu |
| Error graph format | DAG with error propagation | 2025-01-06 | ChenXu |

## Appendix B: Glossary

| Term | Definition |
|------|------------|
| Concurrency Model | Programming paradigm where multiple computations execute simultaneously |
| DAG | Directed Acyclic Graph, represents task dependencies |
| Spawn | Create concurrent execution unit |
| Send Constraint | Type can be safely transferred across threads |
| Sync Constraint | Type can be safely shared across threads |
| Work Stealing | Load balancing technique where idle threads steal work |
| Error Graph | Visualization of error propagation paths |
| Async Function | Function that can execute concurrently |

> **⚠️ Attention: This document is outdated and for reference only.**
>
> The content described in this document is no longer applicable. Please refer to the latest documentation.

# **《YaoXiang: A Transparent Asynchronous Concurrency Model Based on Lazy Evaluation》Technical Whitepaper**

## 🏛️ Section 1: Core Definition: YaoXiang Model

The **YaoXiang model**, inspired by the phrase "万物并作，吾以观复" (All things grow together, and I observe the return) from the *I Ching · Fu Hexagram*, is a programming language concurrency paradigm that allows developers to describe logic in a synchronous, sequential manner. The language runtime causes the computational units within to automatically and efficiently execute concurrently, like all things growing together, and finally unify and coordinate at the end.

### Core Design Principles: Default Lazy + Spawn Type Marker

| Design Principle | Description |
|----------|------|
| **Default Lazy Evaluation** | All functions are lazy by default (similar to Haskell), returning Lazy[T] |
| **Core Count Configuration** | Script header declares `// @cores: N` to automatically enable parallelization |
| **Spawn Type Marker** | `-> T spawn` marks functions as strictly async-able and concurrent, others are concurrent by default |
| **Mixed Evaluation Mode** | `@eager` (decorator, forces eager evaluation), `@auto` (decorator, maintains parallelism) |
| **Void Auto-Eager** | Functions returning `Void` are automatically eagerly evaluated (side effects must execute) |

### Core Three Principles

| Core Principle | Description |
|----------|------|
| **Synchronous Syntax** | What you see is what you get with sequential code, what you write is what you get with execution flow |
| **Concurrency by Nature** | Runtime automatically extracts parallelism, mining concurrency opportunities in data dependencies |
| **Unified Coordination** | Results automatically converge when needed, ensuring logical correctness |

**It achieves this through two fundamental transformations:**

1. **Transforming "Control Flow" into "Data Flow"**: The program is viewed as a purely functional lazy-evaluated data flow graph
2. **Transforming "Async Contagion" into "Dependency Resolution"**: Asynchronicity is no longer an effect of the function signature, but becomes a wait operation automatically executed by the runtime at data dependency points

---

## 📚 Section 2: Terminology System: A Unified Conceptual Map

Around "YaoXiang", we have constructed a clear, self-consistent terminology system that connects all designs:

| Official Term | Corresponding Syntax/Concept | Description |
|----------|---------------|------|
| **Spawn function** | `-> T spawn` | Return type marker, indicating this is a computational unit that strictly can participate in concurrent "YaoXiang" execution |
| **Spawn block** | `spawn { a(), b() }` | Developer-explicitly declared concurrency boundary, tasks within the block will "YaoXiang" execute |
| **Spawn loop** | `spawn for x in xs { ... }` | Data parallel paradigm, loop body "YaoXiang" executes on all data elements |
| **Async value** | `Async[T]` proxy type | A "future value" in the process of YaoXiang, automatically waits for its "YaoXiang" to complete when used |
| **YaoXiang graph** | Lazy computation graph (DAG) | The stage where "YaoXiang" happens, describing dependency and parallelism relationships between all computational units |
| **YaoXiang scheduler** | Runtime task scheduler | The intelligent center responsible for coordinating "all things", letting them "YaoXiang" at the right moment |
| **Error graph** | Error Graph | Error propagation path visualization in concurrent environments, similar to call stacks but showing error flow in the DAG |
| **Resource conflict** | Resource Conflict | Conflict when multiple tasks simultaneously access the same writable resource, detected at compile time and automatically serialized |

> **Technical Exchange Example**: "Here we use a spawn block to concurrently call two spawn functions, and we automatically get their async values."

---

## Section 3: Three-Layer Concurrency Architecture: Progressive Transparency

### 3.1 Architecture Overview

The YaoXiang model provides **three layers of progressive concurrency abstractions**, allowing developers of different skill levels to find suitable usage patterns:

| Layer | Pattern | Syntax Marker | Execution Mode | Controllability | Applicable Scenario |
|------|------|----------|----------|--------|----------|
| **L1** | `@blocking` synchronous | `@blocking` | Fully sequential execution | Highest | Debugging, beginner learning, critical code sections |
| **L2** | Explicit spawn | `spawn` | Developer-controllable concurrency | Medium | Intermediate users, fine-grained concurrency control needed |
| **L3** | Fully transparent | None (default) | Automatic optimal parallelism | Lowest | Experts, automatic parallel optimization |

### 3.2 L1: `@blocking` Synchronous Mode

**Core Features**: Disable all concurrency optimizations, fully sequential execution, convenient for debugging and understanding.

```yaoxiang
# L1: @blocking synchronous mode (annotation placed after return type)
fetch_sync: (String) -> JSON @blocking = (url) => {
    HTTP.get(url).json()
}

main: () -> Void @blocking = () => {
    # Strictly sequential execution, no concurrency whatsoever
    data1 = fetch_sync("https://api.example.com/data1")
    data2 = fetch_sync("https://api.example.com/data2")
    process(data1, data2)
}
```

### 3.3 L2: Explicit Spawn Concurrency

**Core Features**: Developer explicitly marks concurrency units, maintaining controllability while gaining concurrency benefits.

```yaoxiang
# L2: Explicit spawn concurrency
fetch_data: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

process_users_and_posts: () -> Void spawn = () => {
    users = fetch_data("https://api.example.com/users")
    posts = fetch_data("https://api.example.com/posts")
    # users and posts execute automatically in parallel
    print(users.length.to_string())
    print(posts.length.to_string())
}

# Explicit concurrency block
compute_all: () -> (Int, Int, Int) spawn = () => {
    (a, b, c) = spawn {
        heavy_calc(1),
        heavy_calc(2),
        heavy_calc(3)
    }
    (a, b, c)
}
```

### 3.4 L3: Fully Transparent (Default)

**Core Features**: No markers needed, compiler automatically analyzes dependencies and generates optimal parallel execution plan.

```yaoxiang
# L3: Fully transparent (default mode)
heavy_calc: (Int) -> Int = (n) => {
    fibonacci(n)
}

auto_parallel: (Int) -> Int = (n) => {
    # System automatically analyzes: a, b, c have no dependencies, can fully parallelize
    a = heavy_calc(1)
    b = heavy_calc(2)
    c = heavy_calc(3)
    a + b + c
}
```

### 3.5 Manual Control Annotations

| Annotation | Behavior | Usage Scenario |
|------|------|----------|
| `@eager` | Forces eager evaluation | Calculations needing immediate results |

---

## Section 4: Core Concepts

### 4.1 YaoXiang Graph: The Stage for All Things to YaoXiang

All programs are transformed into a **directed acyclic computation graph (DAG)** at compile time, which we call the **YaoXiang graph**.

| Element | Description |
|------|------|
| **Node** | Represents an expression computational unit |
| **Edge** | Represents data dependency relationship (A → B means B depends on A's result) |
| **Lazy** | Nodes are only evaluated when their output is **truly needed** |

### 4.2 Default Lazy Evaluation

All functions use **lazy evaluation** strategy by default:

```yaoxiang
# Script header configures parallel core count
# @cores: 4

# All functions are lazy by default (concurrent by default)
heavy_computation: (Int) -> Int = (x) => {
    # This function will not execute immediately
    # It only executes when the result is used
    fibonacci(x)
}

main: () -> Void = () => {
    # heavy_computation returns Int, the type is Lazy[Int]
    result = heavy_computation(100)

    # Here, result is used in addition, triggering evaluation
    # System automatically finds the best moment to execute in parallel
    total = result + heavy_computation(200)
}
```

### 4.3 Mixed Evaluation Annotations (Decorator Style)

YaoXiang's annotations are similar to Python decorators, used to modify the behavior of functions or expressions:

| Annotation (Decorator) | Behavior |
|----------------|------|
| `@eager` | **Decorator**: Forces eager evaluation, executes immediately |
| `@auto` | **Decorator**: Maintains parallelism (default, can be omitted) |

**Void Auto-Eager Rule:** Functions returning `Void` are automatically eagerly evaluated (no annotation needed), because side effects must execute.

```yaoxiang
# @eager decorator: Forces eager evaluation
heavy_computation: (Int) -> Int = (x) => {
    fibonacci(x)
}

# Functions returning Void are auto-eager (side effect functions)
log: (String) -> Void = (message) => {
    print(message)
}

main: () -> Void = () => {
    # log automatically eagerly executes because it returns Void
    log("Processing started")

    # Use @eager to force eager
    @eager heavy_computation(100)
}
```

### 4.4 Async Value: Async[T] Lazy Proxy Type

Any function with return type marked as `-> T spawn` immediately returns a value of type `Async[T]`, which we call the **async value**.

```yaoxiang
# Spawn function: return type marked as -> JSON spawn
# Indicates this is a computational unit that strictly can YaoXiang
fetch: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

main: () -> Void = () => {
    # fetch returns an async value Async[JSON]
    # But no extra syntax is needed when using it
    data = fetch("https://api.example.com")  # Async[JSON]

    # Here, data automatically waits and unpacks to JSON
    print(data.name)  # Natural as synchronous code
}
```

#### Core Features of Async Values

| Feature | Description |
|------|------|
| **Syntactic Transparency** | `Async[T]` is a subtype of `T` in the type system, usable in any context expecting `T` |
| **On-Demand Waiting** | When a concrete value of type `T` must be used (e.g., field access, arithmetic operations), the runtime automatically suspends and waits |
| **Error Propagation** | Internally actually `Result<T, E>`, errors propagate naturally along the data flow |

### 4.7 Spawn Constructs: From "Modifier" to "Type Marker"

The `spawn` keyword is the only bridge connecting synchronous thinking with asynchronous implementation, having triple semantics:

| Syntax Form | Official Term | Semantics | Runtime Behavior |
|:---------|:---------|:-----|:----------|
| **`-> T spawn`** | Spawn function | Return type marker, indicating this is a computational unit that strictly can participate in YaoXiang | Its call returns `Async[T]`, marking the creation of a YaoXiang graph node |
| **`spawn { ... }`** | Spawn block | Explicitly declared concurrency boundary | Runtime **aggressively** executes each expression within the block as independent tasks concurrently, and implicitly waits for all results at block end |
| **`spawn for`** | Spawn loop | Data parallel loop | Transforms loop body into multiple parallel tasks, automatically performs data sharding, scheduling, and result collection |

---

## Section 5: How It Works: From Code to Execution

### 5.1 Compile Time: Building the YaoXiang Graph

```yaoxiang
# Spawn function definition: return type marked as spawn
fetch: (String) -> String spawn = (url) => { ... }
parse: (String) -> Model spawn = (data) => { ... }

process: () -> Report = () => {
    # Compiler creates YaoXiang graph nodes here
    data_a = fetch("url1")  # Node A: Async[String]
    data_b = fetch("url2")  # Node B: Async[String]

    # Spawn block: explicit concurrency boundary
    (model_a, model_b) = spawn {
        parse(data_a),  # Node C: depends on A
        parse(data_b)   # Node D: depends on B
    }

    # Final convergence node
    generate_report(model_a, model_b)  # Node E
}
```

**Compiler Operations:**
1. Parse source code, build global YaoXiang graph
2. Create computation nodes for each expression
3. Analyze data dependencies, establish edge relationships
4. Subgraphs within `spawn { }` and `spawn for` blocks are tagged with **"parallel evaluation"**

### 5.2 Runtime: YaoXiang Scheduler

An intelligent, work-stealing **YaoXiang scheduler** is responsible for executing the YaoXiang graph:

```rust
// YaoXiang scheduler core logic
impl FlowScheduler {
    fn execute_node(&self, node_id: NodeId) {
        let node = self.get_node(node_id);
        
        match &node.kind {
            NodeKind::AsyncCompute => {
                // Spawn function: submit to coroutine pool
                self.submit_async(node_id);
            }
            NodeKind::ParallelBlock => {
                // Spawn block: aggressively execute all direct child nodes in parallel
                self.submit_parallel(node_id);
            }
            NodeKind::DataParallel { iterator, body } => {
                // Spawn loop: automatic sharding
                self.submit_data_parallel(node_id, iterator, body);
            }
            _ => { /* Synchronous execution */ }
        }
    }
}
```

#### Execution Flow

```
1. To evaluate [E], need [C] and [D]
2. [C] depends on [A], [D] depends on [B]
3. YaoXiang scheduler finds [A] and [B] have no dependency → execute in parallel immediately
4. After [A], [B] complete, due to spawn block tag → execute [C] and [D] in parallel immediately
5. After [C], [D] complete, execute [E]
```

**Key Mechanisms:**

| Mechanism | Description |
|------|------|
| **Lazy Triggering** | Execution starts from requesting the final result, tracing dependencies backward |
| **Automatic Waiting** | When encountering `Async[T]`, automatically suspend and execute other ready tasks |
| **Work Stealing** | Threads steal tasks from other threads' queues, improving CPU utilization |

---

## Section 6: Key Mechanism Details

### 6.1 Side Effects and Evaluation Guarantees

Pure lazy evaluation may cause side effects (such as logging, writing) to never execute. The YaoXiang model adopts **automatic derivation based on return type**:

| Rule | Condition | Behavior |
|------|------|------|
| **Rule One** | Functions returning `Void` | **Automatic eager evaluation** (side effects must execute) |
| **Rule Two** | Expressions using `@eager` decorator | **Forced eager evaluation** regardless of return type |
| **Rule Three** | Functions returning non-Void types | **Lazy evaluation** (default) |

```yaoxiang
# Functions returning Void automatically eagerly execute (side effects)
log: (String) -> Void = (message) => {
    print(message)
}

# @eager decorator: Forces eager evaluation
cache_compute: (Int) -> Int = (x) => {
    # Even though it returns Int, forces immediate execution
    expensive_calculation(x)
}

main: () -> Void = () => {
    # log automatically eagerly executes (returns Void)
    log("Processing started")

    # @eager forces eager execution
    @eager
    cache_compute(100)

    # Normal function lazily executes (returns Int)
    result = heavy_computation(200)  # Does not execute yet
    print(result)  # Executes here
}
```

### 6.2 Error Handling

#### Result Type Definition

```yaoxiang
# Standard Result type (unified constructor syntax)
type Result[T, E] = ok(T) | err(E)

# Custom error type
type ParseError = invalid_format | unexpected_eof | position(Int)

parse_config: (String) -> Result[Config, ParseError] = (content) => {
    if content.is_empty() {
        err(invalid_format)
    } else {
        ok(parse(content))
    }
}
```

#### Error Propagation Syntax

Uses Rust-style `?` operator for transparent error propagation:

```yaoxiang
# Rust-style ? operator
process() -> Result[Data, Error] = {
    data = fetch_data()?      # Automatically waits and checks error
    processed = transform(data)?
    save(processed)?          # Error automatically propagates upward
}

# Pattern matching to handle errors
handle_result: (Result[Int, Error]) -> String = (result) => {
    match result {
        ok(value) => "Success: " + value.to_string()
        err(e) => match e {
            network_error => "Network failed"
            parse_error => "Parse failed"
            _ => "Unknown error"
        }
    }
}
```

#### Error Graph Visualization

Error graph is similar to call stack, but shows error propagation path in DAG:

```
┌─────────────────────────────────────────────────────────────┐
│ Error: Division by zero                                     │
├─────────────────────────────────────────────────────────────┤
│ Error Graph:                                                │
│                                                             │
│   main()                                                   │
│     │                                                       │
│     ├──► calculate()                                        │
│     │         │                                             │
│     │         └──► divide(100, 0)  ✗ [Division by zero]     │
│     │                                                       │
│     └──► fallback()  ✓                                      │
│                                                             │
│ Causality chain: main → calculate → divide                  │
│ Catch location: calculate (line 42)                         │
└─────────────────────────────────────────────────────────────┘
```

#### Error Handling Best Practices

```yaoxiang
# Combine multiple operations that may fail
batch_process: ([String]) -> Result[[String], Error] = (items) => {
    results = items.map(item => {
        process_item(item)?
    })
    ok(results)
}

# with? syntax sugar (future feature)
validate_user: (User) -> Result[ValidatedUser, ValidationError] = (user) => {
    name = user.name.with?(validate_name)?
    email = user.email.with?(validate_email)?
    ok(ValidatedUser(name, email))
}
```

### 6.3 Pure Functions and `@blocking` Synchronous Guarantee

**Core Insight: Pure functions don't block!**

Because:
- Pure functions have no I/O, only CPU computation
- No matter how long the computation takes, it doesn't block the scheduler, only occupies CPU time

**Execution Strategy:**

| Function Type | Execution Strategy | Blocks? |
|----------|----------|--------|
| Pure function (no I/O) | Synchronous execution | No (CPU only) |
| Async function (returns `Async[T]`) | Async execution | No |
| `@blocking` annotated function | Synchronous execution, internal scheduling | No |

**`@blocking` Annotation: Synchronous Execution Guarantee**

The `@blocking` annotation guarantees the function executes in a synchronous manner:
- Result is ready when function returns
- If there are async calls internally, scheduling completes internally
- Suitable for scenarios needing synchronous semantics but may contain async operations internally

```yaoxiang
# @blocking: Synchronous execution, internal async scheduling completes before returning
heavy_compute: (List[Int]) -> Int = (data) => {
    # Internal may have async operations, but completes before return
    processed = data.map(x => async_transform(x))
    processed.sum()
}

# Normal async function: returns Async[T]
fetch_user: (Int) -> Async[User] = (id) => {
    database.query("SELECT * FROM users WHERE id = ?", id)
}

# Pure function: auto-synchronous (no I/O)
factorial: (Int) -> Int = (n) => {
    if n <= 1 then 1 else n * factorial(n - 1)
}

main: () -> Void = () => {
    # @blocking function: synchronous execution
    result = heavy_compute([1, 2, 3, 4, 5])  # Returns result immediately
    print(result)  # 15

    # Async function: returns Async[User]
    user = fetch_user(123)  # Async[User]
    print(user.name)  # Automatically waits and unpacks
}
```

**Runtime Strategy:**

```rust
fn execute_function(node: &DAGNode) {
    match node.execution_mode {
        ExecutionMode::Pure => {
            // Pure function: synchronous execution
            node.execute();
        }
        ExecutionMode::Async => {
            // Async function: submit to async scheduler
            async_runtime.submit(node);
        }
        ExecutionMode::Blocking => {
            // @blocking function: synchronous execution, internal async scheduling
            execute_blocking(node);
        }
    }
}

fn execute_blocking(node: &DAGNode) {
    // Execute function body
    let result = node.execute_body();
    
    // Collect all async operations within
    let internal_async_ops = collect_async_ops(node);
    
    // Wait for all internal async operations to complete
    if !internal_async_ops.is_empty() {
        async_runtime.wait_all(internal_async_ops);
    }
    
    // Return result
    result
}
```

**Design Advantages:**
- **Simple**: No complex effect system needed
- **Flexible**: `@blocking` is optional, use when synchronous semantics needed
- **Efficient**: Pure functions auto-synchronous execution
- **Safe**: Main scheduler never blocks

### 6.4 Resource Conflict Detection

Compile-time analysis of resource access patterns, automatically serialize conflicting operations:

```
Resource Conflict Rule Matrix:
╔═══════════╦══════════╦══════════╗
║  Access   ║   Read   ║   Write  ║
╠═══════════╬══════════╬══════════╣
║   Read    ║ Parallel ║ Serialize║
║   Write   ║ Serialize║ Serialize║
╚═══════════╩══════════╩══════════╝
```

**Compile-Time Analysis Example:**

```rust
// Compile-time analysis of resource access
struct ResourceAccess {
    reads: Set<ResourceId>,   // Resources being read
    writes: Set<ResourceId>,  // Resources being written
}

// Example
file1 = open("a.txt")  // Resource 1: read
file2 = open("b.txt")  // Resource 2: read
// file1 read and file2 read → can parallelize

file3 = open("c.txt")  // Resource 3: write
// file1 read and file3 write → serialize
// file2 read and file3 write → serialize
```

**Code Example:**

```yaoxiang
# Compiler automatically detects and serializes conflicting operations
process_files: () -> Void = () => {
    file_a = open("a.txt")  # Resource 1: read
    file_b = open("b.txt")  # Resource 2: read
    # file_a and file_b are both read-only → can parallelize

    file_c = open("c.txt")  # Resource 3: write
    # file_a read and file_c write → serialize
    # file_b read and file_c write → serialize
}

# Multiple write operations auto-serialize
write_logs: () -> Void = () => {
    log1 = open_log("log1.txt")  # Resource 1: write
    log2 = open_log("log2.txt")  # Resource 2: write
    # log1 and log2 are different resources → can parallelize
}
```

### 6.5 Parallel Race Control: Type System Guarantees Atomicity

**Core Idea: Use the type system to mark data accessed concurrently, compiler checks synchronization correctness.**

**Type Marker System:**

| Type | Semantics | Concurrency Safe | Description |
|------|------|----------|------|
| `T` | Immutable data | ✅ Safe | Default type, multi-task read without race |
| `Ref[T]` | Mutable reference | ⚠️ Needs sync | Marked as concurrently modifiable, compiler checks lock usage |
| `Atomic[T]` | Atomic type | ✅ Safe | Low-level atomic operations, lock-free concurrency |
| `Mutex[T]` | Mutex-wrapped | ✅ Safe | Auto lock/unlock, compiler guarantees |
| `RwLock[T]` | Read-write lock wrapped | ✅ Safe | Optimized for read-heavy, write-light scenarios |

**Type Safety Guarantees:**

```yaoxiang
# Default immutable - naturally race-free
data: List[Int] = [1, 2, 3, 4, 5]
spawn for x in data { process(x) }  # ✅ Safe, read-only no race

# Mutable reference - needs synchronization
counter: Ref[Int] = Ref.new(0)

# Wrong example: accessing Ref without lock (compile error)
spawn for i in 1..10 {
    # ❌ Compile error: Ref must be accessed through synchronization primitives
    counter.value = counter.value + i
}

# Correct example: using with syntax sugar for auto-locking
spawn for i in 1..10 {
    # ✅ with block auto-acquires and releases lock
    with counter.lock() {
        counter.value = counter.value + i
    }
}

# Atomic type - lock-free concurrency
atomic_counter: Atomic[Int] = Atomic.new(0)
spawn for i in 1..10 {
    # ✅ Atomic operation, lock-free safe
    atomic_counter.fetch_add(i)
}
```

**Mutex[T] Type - Compile-Time Lock Guarantee:**

```yaoxiang
# Create mutex-wrapped data
shared_state: Mutex[Map[String, Int]] = Mutex.new(Map.empty())

# Use with syntax sugar (similar to Go's defer)
main: () -> Void = () => {
    spawn for i in 1..100 {
        # with auto-acquires lock, auto-releases after block ends
        with shared_state.lock() {
            # Critical section: protected by Mutex
            current = shared_state.get("count").or(0)
            shared_state.set("count", current + 1)
        }
    }

    # Wait for all tasks to complete
    print(shared_state.get("count"))  # 100
}
```

**Type Inference and Lock Checking:**

```rust
// Compiler checks at compile time
fn compile_check_locks(func: &Function) {
    for node in func.nodes {
        match node {
            NodeKind::ReadRef(ref_var) => {
                // Check if within lock protection scope
                if !is_inside_lock_guard(ref_var) {
                    compile_error!("Ref access must be within lock() protection scope");
                }
            }
            NodeKind::WriteRef(ref_var, _) => {
                // Double check: lock + unique writer
                if !is_inside_lock_guard(ref_var) {
                    compile_error!("Ref modification must be within lock() protection scope");
                }
                if has_multiple_writers(func, ref_var) {
                    compile_error!("Mutex[T] can only have one writer, use RwLock[T]");
                }
            }
            _ => {}
        }
    }
}
```

**Design Advantages:**

| Advantage | Description |
|------|------|
| **Compile-Time Checking** | Missing locks caught at compile time, not runtime deadlock |
| **Zero Runtime Overhead** | Mutex wrapper has no overhead when uncontended |
| **Simple Syntax** | `with lock() { ... }` syntax sugar, auto-manages lifecycle |
| **Type Safe** | Misusing Ref instead of Atomic results in type-level error |

---

## Section 7: Advantages Summary

| Advantage | Description |
|------|------|
| **Zero Contagion** | Async code and sync code have no difference in syntax and type signature, completely eradicating "async/await" contagion |
| **High-Performance Parallelism** | Lazy YaoXiang graph combined with explicit `spawn` markers allows runtime to automatically discover parallelism while giving programmers explicit tools for extreme performance optimization |
| **Simple Mental Model** | Developers only need to focus on data flow and business logic, no need to understand complex concurrency primitives and callbacks |
| **Easy Refactoring** | Extremely low cost to switch between sequential and concurrent logic, just add or remove `spawn {}` wrappers |
| **Intuitive Terminology** | "Spawn function", "spawn block", "async value" make technical discussions extremely intuitive |

---

## Section 8: Implementation Considerations

### 8.1 Compiler

- [ ] Implement data flow analysis, build YaoXiang graph
- [ ] Implement parsing and type inference for `spawn` return type markers
- [ ] Desugar `spawn {}` and `spawn for` into runtime parallel primitives
- [ ] Support annotations (`@eager`, `@blocking`)
- [ ] Implement automatic eager evaluation logic for Void return types
- [ ] Implement resource conflict detection
- [ ] Implement Send/Sync type constraint checking

### 8.2 Runtime

- [ ] Implement work-stealing YaoXiang scheduler
- [ ] Implement dependency-aware task scheduling for computation graph
- [ ] Implement automatic unpacking mechanism for `Async[T]` type
- [ ] Implement automatic eager execution for Void functions
- [ ] Implement error graph generation and propagation
- [ ] Implement resource access serialization

### 8.3 Debugging Tools ⚠️ Required

**Computation Graph Visual Debugger** is key to understanding complex program behavior:

| Feature | Description |
|------|------|
| **Node State Visualization** | Observe Pending/Running/Completed state of each computation node |
| **Dependency Relationship Display** | Show data dependency edges between nodes |
| **Task Flow Tracking** | Observe task flow between threads |
| **Performance Bottleneck Location** | Identify long dependency chains and hot nodes |
| **Error Graph Visualization** | Error propagation path display in concurrent environments |

---

## Section 9: Code Examples

### 9.1 Basic Spawn Function

```yaoxiang
use std.net

# Spawn function definition: return type marked as spawn
fetch_user: (Int) -> User spawn = (id) => {
    response = net.HTTP.get("/users/" + id.to_string())
    response.json()
}

fetch_posts: (Int) -> List[Post] spawn = (user_id) => {
    response = net.HTTP.get("/users/" + user_id.to_string() + "/posts")
    response.json()
}

main: () -> Void = () => {
    # Auto-parallel execution (no dependency)
    user = fetch_user(123)      # Async[User]
    posts = fetch_posts(123)    # Async[List[Post]]

    # Auto-wait and unpack here
    print(user.name)            # Natural as synchronous code
    print(posts.length)
}
```

### 9.2 Spawn Block

```yaoxiang
fetch: (String) -> JSON spawn = (url) => { ... }
parse: (JSON) -> Model spawn = (json) => { ... }

parallel_fetch: () -> (Model, Model) = () => {
    # Spawn block: explicit concurrency boundary
    (model_a, model_b) = spawn {
        parse(fetch("https://api1.com/data")),
        parse(fetch("https://api2.com/data"))
    }
    # Models a and b are ready here
    (model_a, model_b)
}
```

### 9.3 Spawn Loop

```yaoxiang
process_item: (Item) -> Result[Processed, Error] spawn = (item) => { ... }

batch_process: (List[Item]) -> List[Result[Processed, Error]] = (items) => {
    # Spawn loop: data parallelism
    results = [spawn for item in items {
        process_item(item)
    }]
    # results is a List here, containing all processing results
    results
}
```

---

> *"万物并作，吾以观复。"*
> —— *I Ching · Fu Hexagram*
>
> The YaoXiang model combines the declarative elegance of lazy evaluation with the demands of high-performance concurrency, aiming to provide a new paradigm for systems programming that is both safe and extremely expressive.
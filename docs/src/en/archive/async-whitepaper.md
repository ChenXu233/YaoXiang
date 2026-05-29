# **"YaoXiang: A Transparent Asynchronous Concurrency Model Based on Lazy Evaluation" Technical Whitepaper**

## 🏛️ 1. Core Definition: The YaoXiang Model

The **YaoXiang model**, inspired by the I Ching's Hexagram of Return (复卦) — "All things flourish and grow, and I observe their return" — is a programming language concurrency paradigm that allows developers to describe logic in a synchronous, sequential manner, while the language runtime automatically and efficiently executes the computation units concurrently like flourishing things, ultimately unifying and coordinating them.

### Core Design Principles: Default Lazy + spawn Type Markers

| Design Principle | Description |
|----------|------|
| **Default Lazy Evaluation** | All functions are lazy by default (similar to Haskell), returning Lazy[T] |
| **Core Count Configuration** | Script header declares `// @cores: N` to enable parallelization |
| **spawn Type Markers** | `-> T spawn` marks functions as strictly async and concurrent capable; others are implicitly concurrency-ready |
| **Mixed Evaluation Modes** | `@eager` (decorator, forces eager evaluation), `@auto` (decorator, maintains parallelism) |
| **Void Auto-Eager** | Functions returning `Void` automatically evaluate eagerly (side effects must execute) |

### Core Three Principles

| Core Principle | Explanation |
|----------|------|
| **Synchronous Syntax** | What you see is what you get — sequential code with straightforward execution flow |
| **Concurrency by Nature** | The runtime automatically extracts parallelism, mining concurrency opportunities from data dependencies |
| **Unified Coordination** | Results automatically converge when needed, ensuring logical correctness |

**It achieves this through two fundamental transformations:**

1. **Transforming "Control Flow" into "Data Flow"**: The program is viewed as a pure functional lazy evaluation data flow graph
2. **Transforming "Async Poison" into "Dependency Resolution"**: Asynchronicity is no longer an effect in function signatures, but rather becomes automatic wait operations at data dependency points during runtime

---

## 📚 2. Terminology System: A Unified Conceptual Map

Around "YaoXiang," we have constructed a clear, self-consistent terminology system that connects all designs:

| Official Term | Corresponding Syntax/Concept | Explanation |
|----------|---------------|------|
| **Spawn Function** | `-> T spawn` | Return type marker, indicating this is a computation unit that can strictly participate in "YaoXiang" concurrent execution |
| **Spawn Block** | `spawn { a(), b() }` | Developer-explicitly declared concurrency boundary; tasks within will "YaoXiang" execute |
| **Spawn Loop** | `spawn for x in xs { ... }` | Data parallelism paradigm; the loop body "YaoXiang" executes on all data elements |
| **Spawn Value** | `Async[T]` proxy type | A "future value" currently in YaoXiang execution; automatically waits for completion when used |
| **Spawn Graph** | Lazy computation graph (DAG) | The stage where "YaoXiang" occurs; describes dependencies and parallelism between all computation units |
| **Spawn Scheduler** | Runtime task scheduler | The intelligent coordinator responsible for orchestrating "all things" so they "YaoXiang" at the right moments |
| **Error Graph** | Error Graph | Error propagation path visualization in concurrent environments; similar to call stacks but showing error flow in the DAG |
| **Resource Conflict** | Resource Conflict | Conflict when multiple tasks simultaneously access the same writable resource; detected at compile time and automatically serialized |

> **Technical Exchange Example**: "Here we use a spawn block to concurrently call two spawn functions, automatically obtaining their spawn values."

---

## 3. Three-Layer Concurrency Architecture: Progressive Transparency

### 3.1 Architecture Overview

The YaoXiang model provides **three progressive concurrency abstractions**, allowing developers of different skill levels to find suitable usage patterns:

| Level | Pattern | Syntax Marker | Execution Mode | Controllability | Applicable Scenarios |
|------|------|----------|----------|--------|------|
| **L1** | `@blocking` Synchronous | `@blocking` | Fully sequential execution | Highest | Debugging, beginner learning, critical code sections |
| **L2** | Explicit spawn | `spawn` | Developer-controllable concurrency | Medium | Intermediate users, fine-grained concurrency control needed |
| **L3** | Fully Transparent | None (default) | Automatic optimal parallelism | Lowest | Experts, automatic parallel optimization |

### 3.2 L1: `@blocking` Synchronous Mode

**Core Features**: Disable all concurrency optimizations, fully sequential execution, easy for debugging and understanding.

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

**Core Features**: Developer explicitly marks concurrency-ready units, maintaining controllability while gaining concurrency benefits.

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

**Core Features**: No markers required; the compiler automatically analyzes dependencies and generates optimal parallel execution plans.

```yaoxiang
# L3: Fully transparent (default mode)
heavy_calc: (Int) -> Int = (n) => {
    fibonacci(n)
}

auto_parallel: (Int) -> Int = (n) => {
    # System automatically analyzes: a, b, c have no dependencies, fully parallel
    a = heavy_calc(1)
    b = heavy_calc(2)
    c = heavy_calc(3)
    a + b + c
}
```

### 3.5 Manual Control Annotations

| Annotation | Behavior | Use Cases |
|------|------|------|
| `@eager` | Force eager evaluation | Need immediate result |

---

## 2. Core Concepts

### 2.1 Spawn Graph: The Stage for All Things to Flourish

All programs are transformed into a **directed acyclic computation graph (DAG)** at compile time, which we call the **spawn graph**.

| Element | Description |
|------|------|
| **Node** | Represents an expression computation unit |
| **Edge** | Represents data dependency relationship (A → B means B depends on A's result) |
| **Lazy** | Nodes are only evaluated when their output is **truly needed** |

### 2.2 Default Lazy Evaluation

All functions use the **lazy evaluation** strategy by default:

```yaoxiang
# Script header configures parallel core count
# @cores: 4

# All functions default to lazy evaluation (implicitly concurrency-ready)
heavy_computation: (Int) -> Int = (x) => {
    # This function will not execute immediately
    # It only executes when the result is used
    fibonacci(x)
}

main: () -> Void = () => {
    # heavy_computation returns Int, the type is Lazy[Int]
    result = heavy_computation(100)

    # Here, result is used in addition, triggering evaluation
    # The system automatically finds the optimal moment for parallel execution
    total = result + heavy_computation(200)
}
```

### 2.3 Mixed Evaluation Annotations (Decorator Style)

YaoXiang's annotations are similar to Python's decorators, used to modify the behavior of functions or expressions:

| Annotation (Decorator) | Behavior |
|----------------|------|
| `@eager` | **Decorator**: Forces eager evaluation, executes immediately |
| `@auto` | **Decorator**: Maintains parallelism (default, can be omitted) |

**Void Auto-Eager Rule:** Functions returning `Void` automatically evaluate eagerly (no annotation needed), because side effects must execute.

```yaoxiang
# @eager decorator: forces eager evaluation
heavy_computation: (Int) -> Int = (x) => {
    fibonacci(x)
}

# Functions returning Void automatically evaluate eagerly (side effect functions)
log: (String) -> Void = (message) => {
    print(message)
}

main: () -> Void = () => {
    # log executes automatically eagerly because it returns Void
    log("Processing started")

    # Use @eager to force eagerness
    @eager heavy_computation(100)
}
```

### 2.4 Spawn Values: Async[T] Lazy Proxy Type

Any function whose return type is marked `-> T spawn` immediately returns a value of type `Async[T]`, which we call a **spawn value**.

```yaoxiang
# Spawn function: return type marked as -> JSON spawn
# Indicates this is a strictly YaoXiang-executable computation unit
fetch: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

main: () -> Void = () => {
    # fetch returns a spawn value Async[JSON]
    # But no extra syntax needed when using it
    data = fetch("https://api.example.com")  # Async[JSON]

    # Here, data automatically waits and unpacks to JSON
    print(data.name)  # As natural as synchronous code
}
```

#### Core Features of Spawn Values

| Feature | Description |
|------|------|
| **Syntax Transparent** | `Async[T]` is a subtype of `T` in the type system, can be used in any context expecting `T` |
| **On-Demand Waiting** | When a concrete value of type `T` must be used (e.g., field access, arithmetic), the runtime automatically suspends and waits |
| **Error Propagation** | Internally backed by `Result<T, E>`; errors propagate naturally along the data flow |

### 2.7 YaoXiang Constructs: From "Modifier" to "Type Marker"

The `spawn` keyword is the sole bridge connecting synchronous thinking with asynchronous implementation, possessing triple semantics:

| Syntax Form | Official Term | Semantics | Runtime Behavior |
|:---------|:---------|:-----|:----------|
| **`-> T spawn`** | Spawn Function | Return type marker, indicating this is a strictly YaoXiang-executable computation unit | Its call returns `Async[T]`, marking the creation of a spawn graph node |
| **`spawn { ... }`** | Spawn Block | Explicitly declared concurrency boundary | The runtime **aggressively** executes each expression within the block as independent tasks concurrently, implicitly waiting for all results at block end |
| **`spawn for`** | Spawn Loop | Data parallel loop | Transforms the loop body into multiple parallel tasks, automatically performing data sharding, scheduling, and result collection |

---

## 3. How It Works: From Code to Execution

### 3.1 Compile Time: Building the Spawn Graph

```yaoxiang
# Spawn function definition: return type marked as spawn
fetch: (String) -> String spawn = (url) => { ... }
parse: (String) -> Model spawn = (data) => { ... }

process: () -> Report = () => {
    # Compiler creates spawn graph nodes here
    data_a = fetch("url1")  # Node A: Async[String]
    data_b = fetch("url2")  # Node B: Async[String]

    # Spawn block: explicit concurrency boundary
    (model_a, model_b) = spawn {
        parse(data_a),  # Node C: depends on A
        parse(data_b)   # Node D: depends on B
    }

    # Final merge node
    generate_report(model_a, model_b)  # Node E
}
```

**Compiler Operations:**
1. Parse source code, build global spawn graph
2. Create computation nodes for each expression
3. Analyze data dependencies, establish edge relationships
4. Subgraphs within `spawn { }` and `spawn for` blocks are tagged with **"parallel evaluation"**

### 4.2 Runtime: YaoXiang Scheduler

An intelligent, work-stealing **YaoXiang scheduler** is responsible for executing the spawn graph:

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
                // Spawn block: aggressively parallel execute all immediate child nodes
                self.submit_parallel(node_id);
            }
            NodeKind::DataParallel { iterator, body } => {
                // Spawn loop: automatic sharding
                self.submit_data_parallel(node_id, iterator, body);
            }
            _ => { /* synchronous execution */ }
        }
    }
}
```

#### Execution Flow

```
1. To evaluate [E], need [C] and [D]
2. [C] depends on [A], [D] depends on [B]
3. YaoXiang scheduler discovers [A] and [B] have no dependency → immediately parallel execute
4. After [A], [B] complete, due to spawn block tag → immediately parallel execute [C] and [D]
5. After [C], [D] complete, execute [E]
```

**Key Mechanisms:**

| Mechanism | Description |
|------|------|
| **Lazy Triggering** | Execution starts from requesting the final result, tracing dependencies backward |
| **Automatic Waiting** | When encountering `Async[T]`, automatically suspend and execute other ready tasks |
| **Work Stealing** | Threads steal tasks from other threads' queues, improving CPU utilization |

---

## 4. Key Mechanism Details

### 4.1 Side Effects and Evaluation Guarantees

Pure lazy evaluation could cause side effects (such as logging, writing) to never execute. The YaoXiang model adopts **automatic derivation based on return type**:

| Rule | Condition | Behavior |
|------|------|------|
| **Rule One** | Functions returning `Void` | **Automatic eager evaluation** (side effects must execute) |
| **Rule Two** | Expressions using `@eager` decorator | **Forced eager evaluation** regardless of return type |
| **Rule Three** | Functions returning non-Void types | **Lazy evaluation** (default) |

```yaoxiang
# Functions returning Void automatically execute eagerly (side effects)
log: (String) -> Void = (message) => {
    print(message)
}

# @eager decorator: forces eager evaluation
cache_compute: (Int) -> Int = (x) => {
    # Even though it returns Int, force immediate execution
    expensive_calculation(x)
}

main: () -> Void = () => {
    # log executes automatically eagerly (returns Void)
    log("Processing started")

    # @eager forces eager execution
    @eager
    cache_compute(100)

    # Normal function lazy executes (returns Int)
    result = heavy_computation(200)  # Not executed yet
    print(result)  # Executed here
}
```

### 4.2 Error Handling

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

Adopting Rust-style `?` operator for transparent error propagation:

```yaoxiang
# Rust-style ? operator
process() -> Result[Data, Error] = {
    data = fetch_data()?      # Automatically wait and check errors
    processed = transform(data)?
    save(processed)?          # Errors automatically propagate upward
}

# Pattern matching for error handling
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

The error graph is similar to a call stack, but shows error propagation paths in the DAG:

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
│ Causality chain: main → calculate → divide                   │
│ Capture location: calculate (line 42)                       │
└─────────────────────────────────────────────────────────────┘
```

#### Error Handling Best Practices

```yaoxiang
# Combining multiple operations that may fail
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

### 4.3 Pure Functions and `@blocking` Synchronous Guarantee

**Core Insight: Pure functions don't block!**

Because:
- Pure functions have no I/O, only CPU computation
- No matter how long the computation takes, it doesn't block the scheduler, only occupies CPU time

**Execution Strategy:**

| Function Type | Execution Strategy | Blocks? |
|----------|----------|--------|
| Pure functions (no I/O) | Synchronous execution | No (only CPU usage) |
| Async functions (returning `Async[T]`) | Async execution | No |
| `@blocking` annotated functions | Synchronous execution, internal scheduling | No |

**`@blocking` Annotation: Synchronous Execution Guarantee**

The `@blocking` annotation guarantees the function executes in a synchronous manner:
- When the function returns, the result is ready
- If there are async calls internally, they complete before return
- Suitable for scenarios requiring synchronous semantics but potentially containing async operations internally

```yaoxiang
# @blocking: synchronous execution, internal async scheduling completes before returning
heavy_compute: (List[Int]) -> Int = (data) => {
    # Internal may have async operations, but completes before return
    processed = data.map(x => async_transform(x))
    processed.sum()
}

# Normal async function: returns Async[T]
fetch_user: (Int) -> Async[User] = (id) => {
    database.query("SELECT * FROM users WHERE id = ?", id)
}

# Pure function: automatically synchronous (no I/O)
factorial: (Int) -> Int = (n) => {
    if n <= 1 then 1 else n * factorial(n - 1)
}

main: () -> Void = () => {
    # @blocking function: synchronous execution
    result = heavy_compute([1, 2, 3, 4, 5])  # Returns immediately with result
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
            // @blocking function: synchronous execution, internal async operation scheduling
            execute_blocking(node);
        }
    }
}

fn execute_blocking(node: &DAGNode) {
    // Execute function body
    let result = node.execute_body();
    
    // Collect all internal async operations
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
- **Flexible**: `@blocking` is optional, used when synchronous semantics are needed
- **Efficient**: Pure functions automatically execute synchronously
- **Safe**: Main scheduler never blocks

### 4.4 Resource Conflict Detection

Compile-time analysis of resource access patterns, automatically serializing conflicting operations:

```
Resource Conflict Rule Matrix:
╔═══════════╦══════════╦══════════╗
║   Access  ║   Read   ║   Write  ║
╠═══════════╬══════════╬══════════╣
║   Read    ║  Parallel║ Serialize║
║   Write   ║ Serialize║ Serialize║
╚═══════════╩══════════╩══════════╝
```

**Compile-Time Analysis Example**:

```rust
// Compile-time analysis of resource access
struct ResourceAccess {
    reads: Set<ResourceId>,   // Resources being read
    writes: Set<ResourceId>,  // Resources being written
}

// Example
file1 = open("a.txt")  // Resource 1: read
file2 = open("b.txt")  // Resource 2: read
// file1 read and file2 read → can be parallel

file3 = open("c.txt")  // Resource 3: write
// file1 read and file3 write → serialize
// file2 read and file3 write → serialize
```

**Code Example**:

```yaoxiang
# Compiler automatically detects and serializes conflicting operations
process_files: () -> Void = () => {
    file_a = open("a.txt")  # Resource 1: read
    file_b = open("b.txt")  # Resource 2: read
    # file_a and file_b are both read-only → can be parallel

    file_c = open("c.txt")  # Resource 3: write
    # file_a read and file_c write → serialize
    # file_b read and file_c write → serialize
}

# Multiple write operations automatically serialize
write_logs: () -> Void = () => {
    log1 = open_log("log1.txt")  # Resource 1: write
    log2 = open_log("log2.txt")  # Resource 2: write
    # log1 and log2 are different resources → can be parallel
}
```

### 4.5 Parallel Race Control: Type System Guarantees Atomicity

**Core Idea: Use the type system to mark data accessed concurrently; the compiler checks synchronization correctness.**

**Type Marker System:**

| Type | Semantics | Concurrency Safe | Description |
|------|------|----------|------|
| `T` | Immutable data | ✅ Safe | Default type; multiple tasks reading has no races |
| `Ref[T]` | Mutable reference | ⚠️ Needs synchronization | Marked as concurrently modifiable; compiler checks lock usage |
| `Atomic[T]` | Atomic type | ✅ Safe | Low-level atomic operations; lock-free concurrency |
| `Mutex[T]` | Mutex-wrapped | ✅ Safe | Automatic lock/unlock; compiler-guaranteed |
| `RwLock[T]` | Read-write lock wrapped | ✅ Safe | Optimization for read-heavy, write-light scenarios |

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

# Correct example: using with syntax sugar for automatic locking
spawn for i in 1..10 {
    # ✅ with block automatically acquires and releases lock
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

# Using with syntax sugar (similar to Go's defer)
main: () -> Void = () => {
    spawn for i in 1..100 {
        # with automatically acquires lock, automatically releases after block
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
                // Check if inside lock protection scope
                if !is_inside_lock_guard(ref_var) {
                    compile_error!("Ref access must be within lock() protection");
                }
            }
            NodeKind::WriteRef(ref_var, _) => {
                // Double check: lock + unique writer
                if !is_inside_lock_guard(ref_var) {
                    compile_error!("Ref modification must be within lock() protection");
                }
                if has_multiple_writers(func, ref_var) {
                    compile_error!("Mutex[T] can only have one writer; use RwLock[T]");
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
| **Compile-Time Checking** | Missing locks caught at compile time, not runtime deadlocks |
| **Zero Runtime Overhead** | Mutex wrapper has no overhead when uncontended |
| **Concise Syntax** | `with lock() { ... }` syntax sugar, automatic lifecycle management |
| **Type Safe** | Misusing Ref instead of Atomic causes type-level errors |

---

## 5. Advantages Summary

| Advantage | Description |
|------|------|
| **Zero Poison** | Async code is indistinguishable from sync code in syntax and type signatures, completely eradicating "async/await" poison |
| **High-Performance Parallelism** | Lazy YaoXiang graph combined with explicit `spawn` markers allows the runtime to automatically discover parallelism while giving programmers explicit tools for extreme performance optimization |
| **Simple Mental Model** | Developers only need to focus on data flow and business logic, no need to understand complex concurrency primitives and callbacks |
| **Easy Refactoring** | Switching between sequential and concurrent logic has extremely low cost; just add or remove `spawn {}` wrappers |
| **Intuitive Terminology** | "Spawn function," "spawn block," "spawn value" make technical discussions extremely intuitive |

---

## 6. Implementation Considerations

### 6.1 Compiler

- [ ] Implement data flow analysis, build YaoXiang graph
- [ ] Implement parsing and type inference for `spawn` return type markers
- [ ] Desugar `spawn {}` and `spawn for` into runtime parallel primitives
- [ ] Support annotations (`@eager`, `@blocking`)
- [ ] Implement Void return type automatic eager evaluation logic
- [ ] Implement resource conflict detection
- [ ] Implement Send/Sync trait constraint checking

### 6.2 Runtime

- [ ] Implement work-stealing YaoXiang scheduler
- [ ] Implement dependency-aware task scheduling for computation graph
- [ ] Implement `Async[T]` type automatic unwrapping mechanism
- [ ] Implement Void function automatic eager execution
- [ ] Implement error graph generation and propagation
- [ ] Implement resource access serialization

### 6.3 Debugging Tools ⚠️ Must-Have

**Computation Graph Visual Debugger** is key to understanding complex program behavior:

| Feature | Description |
|------|------|
| **Node State Visualization** | Observe Pending/Running/Completed state of each computation node |
| **Dependency Relationship Display** | Show data dependency edges between nodes |
| **Task Flow Tracking** | Observe task flow between threads |
| **Performance Bottleneck Locating** | Identify long chains and hot nodes |
| **Error Graph Visualization** | Error propagation path display in concurrent environments |

---

## 7. Code Examples

### 7.1 Basic Spawn Function

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
    # Automatically parallel execute (no dependencies)
    user = fetch_user(123)      # Async[User]
    posts = fetch_posts(123)    # Async[List[Post]]

    # Automatically wait and unpack here
    print(user.name)            # As natural as synchronous code
    print(posts.length)
}
```

### 7.2 Spawn Block

```yaoxiang
fetch: (String) -> JSON spawn = (url) => { ... }
parse: (JSON) -> Model spawn = (json) => { ... }

parallel_fetch: () -> (Model, Model) = () => {
    # Spawn block: explicit concurrency boundary
    (model_a, model_b) = spawn {
        parse(fetch("https://api1.com/data")),
        parse(fetch("https://api2.com/data"))
    }
    # Both models a and b are ready here
    (model_a, model_b)
}
```

### 7.3 Spawn Loop

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

> *"All things flourish and grow, and I observe their return."*
> —— I Ching, Hexagram of Return (复卦)
>
> The YaoXiang model combines the declarative elegance of lazy evaluation with the demands of high-performance concurrency, aiming to provide system programming with a paradigm that is both safe and extraordinarily expressive.
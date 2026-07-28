> **⚠️ Note: This document is outdated and for reference only.**
>
> The content described in this document is no longer applicable. Please refer to the latest documentation.

# **Spawn: A Transparent Asynchronous Concurrency Model Based on Lazy Evaluation**

## 🏛️ 1. Core Definition: The Spawn Model

The **Spawn Model**, inspired by "万物并作，吾以观复" (All things arise together; I observe their return) from the Yijing's Hexagram 24 (Fu), is a programming language concurrency paradigm that allows developers to describe logic in a synchronous, sequential manner while the language runtime automatically and efficiently executes the computational units concurrently like all things arising together, finally unifying and coordinating the results.

### Core Design Principles: Default Lazy + spawn Type Markers

| Design Principle         | Description                                                                                                  |
| ------------------------ | ------------------------------------------------------------------------------------------------------------ |
| **Default Lazy Evaluation** | All functions are lazy by default (similar to Haskell), returning Lazy[T]                                    |
| **Core Count Configuration** | Script header `// @cores: N` enables parallelization automatically                                         |
| **spawn Type Marker**    | `-> T spawn` marks a function as strictly asynchronous and concurrently executable; others are parallelizable by default |
| **Mixed Evaluation Modes**  | `@eager` (decorator, forces eager evaluation), `@auto` (decorator, maintains parallelism)                |
| **Void Auto-Eager**      | Functions returning `Void` are automatically eagerly evaluated (side effects must execute)                  |

### Three Core Principles

| Core Principle      | Description                                                                             |
| ------------------- | --------------------------------------------------------------------------------------- |
| **Synchronous Syntax** | What you see is what you get - sequential code with the execution flow matching the written logic |
| **Concurrency by Nature** | Runtime automatically extracts parallelism, discovering concurrency opportunities in data dependencies |
| **Unified Coordination** | Results automatically converge when needed, ensuring logical correctness               |

**It achieves this through two fundamental transformations:**

1. **Transforming "control flow" into "data flow"**: The program is viewed as a pure functional lazy-evaluated data flow graph
2. **Transforming "async contagion" into "dependency resolution"**: Asynchronicity is no longer an effect in the function signature, but becomes wait operations automatically executed by the runtime at data dependency points

---

## 📚 2. Terminology System: A Unified Conceptual Map

Around "spawn", we have constructed a clear, self-consistent terminology system that connects all design elements:

| Official Term     | Corresponding Syntax/Concept | Description                                                                          |
| ----------------- | ---------------------------- | ------------------------------------------------------------------------------------ |
| **spawn function**   | `-> T spawn`                 | Return type marker indicating this is a computation unit that can participate in "spawn" concurrent execution |
| **spawn block**      | `spawn { a(), b() }`         | Developer-explicitly declared concurrency scope; tasks within the block execute "together" |
| **spawn loop**       | `spawn for x in xs { ... }`  | Data parallelism paradigm; loop body executes "together" across all data elements     |
| **spawn value**      | `Async[T]` proxy type        | A "future value" that is currently spawning; automatically waits for completion when used |
| **spawn graph**      | Lazy computation graph (DAG) | The stage where "spawning" occurs; describes dependencies and parallelism between all computation units |
| **spawn scheduler**  | Runtime task scheduler       | The intelligent core responsible for coordinating "all things" so they "spawn" at the right moments |
| **error graph**      | Error Graph                  | Visualization of error propagation paths in a concurrent environment, similar to call stacks but showing error flow in the DAG |
| **resource conflict**| Resource Conflict            | Conflict when multiple tasks simultaneously access the same writable resource; detected at compile time and automatically serialized |

> **Technical discussion example**: "Here we use a spawn block to concurrently call two spawn functions, which automatically gives us their spawn values."

---

## 3. Three-Layer Concurrency Architecture: Progressively Transparent

### 3.1 Architecture Overview

The Spawn Model provides **three progressive layers of concurrency abstraction**, allowing developers of different skill levels to find the appropriate usage pattern:

| Layer  | Pattern           | Syntax Marker   | Execution Mode      | Controllability | Applicable Scenario                        |
| ------ | ----------------- | ---------------- | ------------------- | --------------- | ------------------------------------------- |
| **L1** | `@blocking` sync  | `@blocking`      | Fully sequential    | Highest         | Debugging, beginners, critical code sections |
| **L2** | Explicit spawn    | `spawn`          | Developer-controlled | Medium          | Intermediate users, fine-grained concurrency control |
| **L3** | Fully transparent | None (default)   | Auto-optimal parallel| Lowest          | Experts, automatic parallel optimization    |

### 3.2 L1: `@blocking` Synchronous Mode

**Core characteristic**: Disable all concurrency optimizations, fully sequential execution, easy for debugging and understanding.

```yaoxiang
# L1: @blocking synchronous mode (annotation placed after return type)
fetch_sync: (String) -> JSON @blocking = (url) => {
    HTTP.get(url).json()
}

main: () -> Void @blocking = () => {
    # Strictly sequential execution, no concurrency at all
    data1 = fetch_sync("https://api.example.com/data1")
    data2 = fetch_sync("https://api.example.com/data2")
    process(data1, data2)
}
```

### 3.3 L2: Explicit spawn Concurrency

**Core characteristic**: Developer explicitly marks parallelizable units, maintaining control while gaining concurrency benefits.

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

# Explicit spawn block
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

**Core characteristic**: No markers needed; compiler automatically analyzes dependencies and generates optimal parallel execution plans.

```yaoxiang
# L3: Fully transparent (default mode)
heavy_calc: (Int) -> Int = (n) => {
    fibonacci(n)
}

auto_parallel: (Int) -> Int = (n) => {
    # System automatically analyzes: a, b, c have no dependencies, can execute fully in parallel
    a = heavy_calc(1)
    b = heavy_calc(2)
    c = heavy_calc(3)
    a + b + c
}
```

### 3.5 Manual Control Annotations

| Annotation | Behavior            | Use Case                              |
| ---------- | ------------------- | ------------------------------------- |
| `@eager`   | Force eager evaluation | When you need immediate results      |

---

## 2. Core Concepts

### 2.1 Spawn Graph: The Stage for All Things to Spawn

All programs are transformed into a **directed acyclic computation graph (DAG)** at compile time, which we call the **spawn graph**.

| Element   | Description                                                        |
| --------- | ------------------------------------------------------------------ |
| **Node**  | Represents a computation unit for an expression                    |
| **Edge**  | Represents data dependency (A → B means B depends on A's result)  |
| **Lazy**  | Nodes are only evaluated when their output is **truly needed**    |

### 2.2 Default Lazy Evaluation

All functions use **lazy evaluation** by default:

```yaoxiang
# Script header configures parallel core count
# @cores: 4

# All functions use lazy evaluation by default (parallelizable by default)
heavy_computation: (Int) -> Int = (x) => {
    # This function does not execute immediately
    # It only executes when the result is used
    fibonacci(x)
}

main: () -> Void = () => {
    # heavy_computation returns Int, type is Lazy[Int]
    result = heavy_computation(100)

    # Here, result is used in addition, triggering evaluation
    # System automatically finds the optimal moment for parallel execution
    total = result + heavy_computation(200)
}
```

### 2.3 Mixed Evaluation Annotations (Decorator Style)

YaoXiang's annotations are similar to Python's decorators, used to modify the behavior of functions or expressions:

| Annotation (Decorator) | Behavior                                      |
| ---------------------- | --------------------------------------------- |
| `@eager`               | **Decorator**: Forces eager evaluation, executes immediately |
| `@auto`                | **Decorator**: Maintains parallelism (default, can be omitted) |

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
    # log executes auto-eagerly because it returns Void
    log("Processing started")

    # Use @eager to force eager evaluation
    @eager heavy_computation(100)
}
```

### 2.4 Spawn Values: Async[T] Lazy Proxy Type

Any function with a return type marked as `-> T spawn` immediately returns a value of type `Async[T]`, which we call a **spawn value**.

```yaoxiang
# spawn function: Return type marked as -> JSON spawn
# Indicates this is a computation unit strictly executable as a spawn
fetch: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

main: () -> Void = () => {
    # fetch returns a spawn value Async[JSON]
    # But no extra syntax is needed when using it
    data = fetch("https://api.example.com")  # Async[JSON]

    # Here, data automatically waits and unpacks to JSON
    print(data.name)  # As natural as synchronous code
}
```

#### Core Characteristics of Spawn Values

| Characteristic      | Description                                                                                      |
| ------------------- | ------------------------------------------------------------------------------------------------ |
| **Syntax Transparent** | `Async[T]` is a subtype of `T` in the type system, usable in any context expecting `T`        |
| **On-demand Wait**  | When a concrete value of type `T` must be used (e.g., field access, arithmetic), runtime automatically suspends and waits |
| **Error Propagation** | Internally it's actually `Result<T, E>`, errors propagate naturally along the data flow     |

### 2.7 Spawn Constructs: From "Modifier" to "Type Marker"

The `spawn` keyword is the sole bridge connecting synchronous thinking with asynchronous implementation, with triple semantics:

| Syntax Form          | Official Term | Semantic                                                | Runtime Behavior                                                                                          |
| :------------------ | :------------ | :------------------------------------------------------ | :-------------------------------------------------------------------------------------------------------- |
| **`-> T spawn`**    | spawn function | Return type marker, indicating this is a computation unit strictly able to participate in spawn | Its call returns `Async[T]`, marking the creation of a spawn graph node                                  |
| **`spawn { ... }`** | spawn block   | Explicitly declared concurrency scope                   | Runtime **aggressively** executes each expression in the block as an independent task concurrently, implicitly waiting for all results at block end |
| **`spawn for`**     | spawn loop    | Data parallel loop                                      | Transforms loop body into multiple parallel tasks, automatically performing data sharding, scheduling, and result collection |

---

## 3. How It Works: From Code to Execution

### 3.1 Compile Time: Building the Spawn Graph

```yaoxiang
# spawn function definition: Return type marked as spawn
fetch: (String) -> String spawn = (url) => { ... }
parse: (String) -> Model spawn = (data) => { ... }

process: () -> Report = () => {
    # Compiler creates spawn graph nodes here
    data_a = fetch("url1")  # Node A: Async[String]
    data_b = fetch("url2")  # Node B: Async[String]

    # spawn block: Explicit concurrency scope
    (model_a, model_b) = spawn {
        parse(data_a),  # Node C: depends on A
        parse(data_b)   # Node D: depends on B
    }

    # Final convergence node
    generate_report(model_a, model_b)  # Node E
}
```

**Compiler operations:**

1. Parse source code, construct global spawn graph
2. Create computation nodes for each expression
3. Analyze data dependencies, establish edge relationships
4. Subgraphs within `spawn { }` and `spawn for` blocks are tagged with **"parallel evaluation"**

### 4.2 Runtime: Spawn Scheduler

An intelligent, work-stealing **spawn scheduler** is responsible for executing the spawn graph:

```rust
// Spawn scheduler core logic
impl FlowScheduler {
    fn execute_node(&self, node_id: NodeId) {
        let node = self.get_node(node_id);

        match &node.kind {
            NodeKind::AsyncCompute => {
                // spawn function: Submit to coroutine pool
                self.submit_async(node_id);
            }
            NodeKind::ParallelBlock => {
                // spawn block: Aggressively parallel execute all direct child nodes
                self.submit_parallel(node_id);
            }
            NodeKind::DataParallel { iterator, body } => {
                // spawn loop: Automatic sharding
                self.submit_data_parallel(node_id, iterator, body);
            }
            _ => { /* Execute synchronously */ }
        }
    }
}
```

#### Execution Flow

```
1. To evaluate [E], need [C] and [D]
2. [C] depends on [A], [D] depends on [B]
3. Spawn scheduler finds [A] and [B] have no dependency → Execute in parallel immediately
4. After [A], [B] complete, due to spawn block marker → Execute [C] and [D] in parallel immediately
5. After [C], [D] complete, execute [E]
```

**Key mechanisms:**

| Mechanism           | Description                                                               |
| ------------------- | ------------------------------------------------------------------------- |
| **Lazy Trigger**    | Execution starts from requesting the final result, tracing dependencies backward |
| **Auto-wait**       | When encountering `Async[T]`, automatically suspend and execute other ready tasks |
| **Work Stealing**   | Threads steal tasks from other threads' queues, improving CPU utilization |

---

## 4. Key Mechanisms in Detail

### 4.1 Side Effects and Evaluation Guarantees

Pure lazy evaluation may cause side effects (like logging, writing) to never execute. The Spawn Model uses **automatic derivation based on return type**:

| Rule             | Condition                         | Behavior                                      |
| --------------- | --------------------------------- | --------------------------------------------- |
| **Rule One**     | Functions returning `Void`        | **Auto-eager evaluation** (side effects must execute) |
| **Rule Two**     | Expressions using `@eager` decorator | **Force eager evaluation** regardless of return type |
| **Rule Three**   | Functions returning non-Void types | **Lazy evaluation** (default)                |

```yaoxiang
# Functions returning Void execute auto-eagerly (side effects)
log: (String) -> Void = (message) => {
    print(message)
}

# @eager decorator: Forces eager evaluation
cache_compute: (Int) -> Int = (x) => {
    # Even though it returns Int, force immediate execution
    expensive_calculation(x)
}

main: () -> Void = () => {
    # log auto-eagerly executes (returns Void)
    log("Processing started")

    # @eager forces eager execution
    @eager
    cache_compute(100)

    # Normal function lazy evaluates (returns Int)
    result = heavy_computation(200)  # Does not execute yet
    print(result)  # Executes here
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

Uses Rust-style `?` operator for transparent error propagation:

```yaoxiang
# Rust-style ? operator
process() -> Result[Data, Error] = {
    data = fetch_data()?      # Auto-wait and check errors
    processed = transform(data)?
    save(processed)?          # Errors propagate up automatically
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
│ Causality chain: main → calculate → divide                  │
│ Catch location: calculate (line 42)                         │
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

**Core insight: Pure functions don't block!**

Because:

- Pure functions have no I/O, only CPU computation
- However long the computation takes, it doesn't block the scheduler, only occupies CPU time

**Execution strategies:**

| Function Type                    | Execution Strategy       | Blocks?              |
| -------------------------------- | ------------------------ | -------------------- |
| Pure functions (no I/O)          | Synchronous execution    | No (only CPU usage)  |
| Async functions (return `Async[T]`) | Async execution       | No                   |
| `@blocking` annotated functions  | Synchronous execution, internal scheduling | No       |

**`@blocking` annotation: Synchronous execution guarantee**

The `@blocking` annotation guarantees the function executes synchronously:

- When the function returns, the result is ready
- If there are internal async calls, scheduling is completed internally
- Suitable for scenarios requiring synchronous semantics but potentially containing async operations

```yaoxiang
# @blocking: Synchronous execution, returns after internal async scheduling completes
heavy_compute: (List[Int]) -> Int = (data) => {
    # Internal may have async operations, but completes before returning
    processed = data.map(x => async_transform(x))
    processed.sum()
}

# Normal async function: Returns Async[T]
fetch_user: (Int) -> Async[User] = (id) => {
    database.query("SELECT * FROM users WHERE id = ?", id)
}

# Pure function: Auto-synchronous (no I/O)
factorial: (Int) -> Int = (n) => {
    if n <= 1 then 1 else n * factorial(n - 1)
}

main: () -> Void = () => {
    # @blocking function: Synchronous execution
    result = heavy_compute([1, 2, 3, 4, 5])  # Returns result immediately
    print(result)  # 15

    # Async function: Returns Async[User]
    user = fetch_user(123)  # Async[User]
    print(user.name)  # Auto-wait and unpack
}
```

**Runtime strategy:**

```rust
fn execute_function(node: &DAGNode) {
    match node.execution_mode {
        ExecutionMode::Pure => {
            // Pure function: Execute synchronously
            node.execute();
        }
        ExecutionMode::Async => {
            // Async function: Submit to async scheduler
            async_runtime.submit(node);
        }
        ExecutionMode::Blocking => {
            // @blocking function: Execute synchronously, schedule async operations internally
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

**Design advantages:**

- **Concise**: No complex effect system needed
- **Flexible**: `@blocking` is optional, use when synchronous semantics are needed
- **Efficient**: Pure functions auto-execute synchronously
- **Safe**: Main scheduler never blocks

### 4.4 Resource Conflict Detection

Compile-time analysis of resource access patterns, automatically serializing conflicting operations:

```
Resource conflict rule matrix:
╔═══════════╦══════════╦══════════╗
║  Access   ║   Read   ║   Write  ║
╠═══════════╬══════════╬══════════╣
║   Read    ║  Parallel ║ Serialize║
║   Write   ║ Serialize ║ Serialize║
╚═══════════╩══════════╩══════════╝
```

**Compile-time analysis example:**

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

**Code example:**

```yaoxiang
# Compiler auto-detects and serializes conflicting operations
process_files: () -> Void = () => {
    file_a = open("a.txt")  # Resource 1: read
    file_b = open("b.txt")  # Resource 2: read
    # file_a and file_b are read-only → can be parallel

    file_c = open("c.txt")  # Resource 3: write
    # file_a read and file_c write → serialize
    # file_b read and file_c write → serialize
}

# Multiple write operations auto-serialized
write_logs: () -> Void = () => {
    log1 = open_log("log1.txt")  # Resource 1: write
    log2 = open_log("log2.txt")  # Resource 2: write
    # log1 and log2 are different resources → can be parallel
}
```

### 4.5 Parallel Race Control: Type System Guarantees Atomicity

**Core idea: Use the type system to mark data accessed concurrently; compiler checks synchronization correctness.**

**Type marking system:**

| Type          | Semantic         | Concurrency Safe | Description                                     |
| ------------- | ---------------- | ---------------- | ----------------------------------------------- |
| `T`           | Immutable data   | ✅ Safe          | Default type, multiple tasks can read without races |
| `Ref[T]`      | Mutable reference| ⚠️ Needs sync    | Marked as concurrently modifiable, compiler checks lock usage |
| `Atomic[T]`   | Atomic type      | ✅ Safe          | Low-level atomic operations, lock-free concurrency |
| `Mutex[T]`    | Mutex wrapper    | ✅ Safe          | Auto lock/unlock, compiler guarantees            |
| `RwLock[T]`   | Read-write lock wrapper | ✅ Safe    | Optimization for read-heavy, write-light scenarios |

**Type safety guarantees:**

```yaoxiang
# Default immutable - naturally race-free
data: List[Int] = [1, 2, 3, 4, 5]
spawn for x in data { process(x) }  # ✅ Safe, read-only no races

# Mutable reference - needs synchronization
counter: Ref[Int] = Ref.new(0)

# Wrong example: Accessing Ref without lock (compile error)
spawn for i in 1..10 {
    # ❌ Compile error: Ref must be accessed through synchronization primitives
    counter.value = counter.value + i
}

# Correct example: Using with syntax sugar for auto-locking
spawn for i in 1..10 {
    # ✅ with block auto-acquires and releases lock
    with counter.lock() {
        counter.value = counter.value + i
    }
}

# Atomic type - lock-free concurrency
atomic_counter: Atomic[Int] = Atomic.new(0)
spawn for i in 1..10 {
    # ✅ Atomic operations, lock-free safe
    atomic_counter.fetch_add(i)
}
```

**Mutex[T] Type - Compile-time lock guarantee:**

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

**Type inference and lock checking:**

```rust
// Compiler checks at compile time
fn compile_check_locks(func: &Function) {
    for node in func.nodes {
        match node {
            NodeKind::ReadRef(ref_var) => {
                // Check if inside lock protection
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
                    compile_error!("Mutex[T] can only have one writer, use RwLock[T]");
                }
            }
            _ => {}
        }
    }
}
```

**Design advantages:**

| Advantage             | Description                                                       |
| --------------------- | ----------------------------------------------------------------- |
| **Compile-time checks** | Lock omissions caught at compile time, not runtime deadlocks    |
| **Zero runtime overhead** | Mutex wrapper has no overhead when there are no conflicts      |
| **Concise syntax**    | `with lock() { ... }` syntax sugar, auto-manages lifecycle       |
| **Type safety**       | Misusing Ref instead of Atomic causes type-level errors           |

---

## 5. Advantages Summary

| Advantage             | Description                                                                                                       |
| --------------------- | ----------------------------------------------------------------------------------------------------------------- |
| **Zero Contagion**    | Async code is syntactically and type-signature identical to sync code, completely eliminating "async/await" contagion |
| **High-Performance Parallelism** | Lazy spawn graph combined with explicit `spawn` markers allows runtime to auto-discover parallelism while giving programmers explicit tools for extreme performance optimization |
| **Simple Mental Model** | Developers only need to focus on data flow and business logic, no need to understand complex concurrency primitives and callbacks |
| **Easy Refactoring**  | Switching between sequential and concurrent logic has extremely low cost, just add or remove `spawn {}` wrappers |
| **Intuitive Terminology** | "spawn function", "spawn block", "spawn value" make technical discussions extremely intuitive |

---

## 6. Implementation Considerations

### 6.1 Compiler

- [ ] Implement data flow analysis, construct spawn graph
- [ ] Implement `spawn` return type marker parsing and type inference
- [ ] Desugar `spawn {}` and `spawn for` into runtime parallel primitives
- [ ] Support annotations (`@eager`, `@blocking`)
- [ ] Implement Void return type auto-eager evaluation logic
- [ ] Implement resource conflict detection
- [ ] Implement Send/Sync type constraint checks

### 6.2 Runtime

- [ ] Implement work-stealing spawn scheduler
- [ ] Implement computation graph dependency-aware task scheduling
- [ ] Implement `Async[T]` type auto-unwrapping mechanism
- [ ] Implement Void function auto-eager execution
- [ ] Implement error graph generation and propagation
- [ ] Implement resource access serialization

### 6.3 Debugging Tools ⚠️ Required

**Computation graph visualization debugger** is key to understanding complex program behavior:

| Feature                     | Description                                                     |
| --------------------------- | --------------------------------------------------------------- |
| **Node state visualization** | Observe Pending/Running/Completed state of each computation node |
| **Dependency relationship display** | Show data dependency edges between nodes                  |
| **Task flow tracking**      | Observe task flow between threads                               |
| **Performance bottleneck location** | Identify long chains and hot nodes                       |
| **Error graph visualization** | Show error propagation paths in concurrent environment         |

---

## 7. Code Examples

### 7.1 Basic spawn Function

```yaoxiang
use std.net

# spawn function definition: Return type marked as spawn
fetch_user: (Int) -> User spawn = (id) => {
    response = net.HTTP.get("/users/" + id.to_string())
    response.json()
}

fetch_posts: (Int) -> List[Post] spawn = (user_id) => {
    response = net.HTTP.get("/users/" + user_id.to_string() + "/posts")
    response.json()
}

main: () -> Void = () => {
    # Auto-parallel execution (no dependencies)
    user = fetch_user(123)      # Async[User]
    posts = fetch_posts(123)    # Async[List[Post]]

    # Auto-wait and unpack here
    print(user.name)            # As natural as synchronous code
    print(posts.length)
}
```

### 7.2 spawn Block

```yaoxiang
fetch: (String) -> JSON spawn = (url) => { ... }
parse: (JSON) -> Model spawn = (json) => { ... }

parallel_fetch: () -> (Model, Model) = () => {
    # spawn block: Explicit concurrency scope
    (model_a, model_b) = spawn {
        parse(fetch("https://api1.com/data")),
        parse(fetch("https://api2.com/data"))
    }
    # model a and b are ready here
    (model_a, model_b)
}
```

### 7.3 spawn Loop

```yaoxiang
process_item: (Item) -> Result[Processed, Error] spawn = (item) => { ... }

batch_process: (List[Item]) -> List[Result[Processed, Error]] = (items) => {
    # spawn loop: Data parallelism
    results = [spawn for item in items {
        process_item(item)
    }]
    # results is a List here, containing all processing results
    results
}
```

---

> _"万物并作，吾以观复。"_ —— Yijing, Hexagram 24 (Fu)
>
> The Spawn Model combines the declarative elegance of lazy evaluation with the demands of high-performance concurrency, aiming to provide a new paradigm for systems programming that is both safe and highly expressive.

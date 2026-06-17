# Concurrency Model Specification

> **Status**: Formal Specification. Based on RFC-024 (Concurrency Model), RFC-009 (Ownership Model), RFC-008 (Runtime Architecture).

This document defines the concurrency model specification for the YaoXiang programming language, including `{}` block semantics, the `spawn` concurrency primitive, ownership interaction, error handling, and resource types.

**Core Design — One Primitive, One Rule**:

```
spawn { ... }              ← The only parallel primitive
Direct child assignments create tasks  ← The only rule
Synchronous blocking wait for results  ← The only behavior
```

---

## Chapter 1: Overview

### 1.1 The Nature of `{}` Blocks

In YaoXiang, `{}` is a **dependency-driven computation unit**.

| Attribute | Description |
|------|------|
| Dependency-driven | The block checks whether all internal variables are ready when executing; if all are ready, it executes immediately, otherwise it blocks and waits |
| Execution timing | Determined by dependencies, unrelated to "immediate" or "delayed" |
| Return value | Use `return` to explicitly return a value; without `return`, the default return is `Void` |
| Syntax uniformity | The semantics are consistent whether appearing in a function body, variable initialization, or after `spawn` |
| Scope isolation | Variables are strictly confined to the inside of `{}` and do not leak to the outer scope |

```yaoxiang
// Dependency-driven example
x = compute_x()        // x is ready
y = compute_y()        // y is ready
result = {
    // Depends on x and y; executes immediately once both are ready
    return x + y
}
```

### 1.2 Return Rules

| Syntax | Return Value | Description |
|------|--------|------|
| `= expr` (no braces) | Directly returns `expr` | The expression is the value |
| `= { ... }` (with braces) | Must use `return`, otherwise returns `Void` | Blocks require explicit return |

```yaoxiang
// No braces: direct return
add: (a: Int, b: Int) -> Int = a + b

// With braces: must use return
process: (data: Data) -> Result = {
    validated = validate(data)?
    return ok(transform(validated))
}

// With braces but no return: returns Void
log: (message: String) -> Void = {
    print(message)  // No return, returns Void
}
```

### 1.3 spawn Block Semantics

`spawn { ... }` is the **only parallel primitive** in YaoXiang.

**Core Rules**:
- The **direct child assignments** of a spawn block create parallel tasks
- Assignments inside nested `{}` do not count as independent tasks
- The entire spawn block synchronously blocks, waiting for all tasks to complete before returning the result
- No callbacks, no `await`, no annotations

```yaoxiang
// Two tasks execute in parallel
(a, b) = spawn {
    fetch("url1"),      // Task 1
    fetch("url2")       // Task 2
}
// Continues after both complete
```

### 1.4 User Mental Model

> The ordinary code you write executes sequentially.
> When you want multiple things to happen at once, put them into a `spawn { ... }` block.
> Each direct assignment inside the block starts immediately (in parallel), and the results you need are automatically awaited.
> The entire block waits for everything to finish, then gives you the final result.
> No callbacks, no `await`, no strange annotations.

---

## Chapter 2: Syntax and Semantics

### 2.1 Ordinary Code

Ordinary code (outside spawn blocks) executes **sequentially**.

```yaoxiang
a = compute_a()     // Executes first
b = compute_b(a)    // Depends on a, executes after a completes
c = compute_c(b)    // Depends on b, executes after b completes
```

### 2.2 spawn Block

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' SpawnBody '}'
SpawnBody   ::= Assignment (',' Assignment)*
```

**Semantics**:
1. Direct child assignments inside the spawn block execute as independent tasks in parallel
2. Each task's result is bound to the corresponding pattern variable
3. The entire block blocks until all tasks complete
4. Returns a tuple of all results

```yaoxiang
// Single task
result = spawn {
    fetch("url")
}

// Multiple tasks
(a, b, c) = spawn {
    fetch("url1"),
    fetch("url2"),
    fetch("url3")
}
```

### 2.3 spawn in Function Body

A function body is itself a `{}` block, in which `spawn` can be used.

```yaoxiang
fetch_and_parse: (urls: List(String)) -> List(Data) = {
    results = spawn for url in urls {
        parsed = parse(fetch(url))
    }
    return results
}
```

### 2.4 spawn in Loops

```
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Assignment '}'
```

**Semantics**: A data-parallel loop where each iteration runs as an independent task.

```yaoxiang
// Process each element in the list in parallel
results = spawn for item in items {
    result = process(item)
}
```

> **Note**: The loop body of `spawn for` consists of independent tasks and does not support sharing mutable state across iterations. If results need to be aggregated, use `spawn for` to collect results and then process them externally.

```yaoxiang
// Correct: parallel processing followed by external aggregation
transformed = spawn for item in items {
    result = transform(item)
}
total = sum(transformed)   // Sequential aggregation
```

### 2.5 Nested spawn

spawn blocks can be nested; the inner spawn creates a new concurrency domain.

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

Only the direct child assignments of the inner spawn are tasks; the outer spawn does not penetrate.

---

## Chapter 3: Interaction with the Ownership Model

### 3.1 Move Semantics

Move is YaoXiang's default semantics (zero-copy). Once a variable enters a spawn block, it cannot be used externally.

```yaoxiang
data = load_data()
result = spawn {
    process(data)   // Ownership of data moves into the spawn block
}
// data is unavailable here (has been moved)
```

### 3.2 Borrow Tokens

`&T` and `&mut T` are zero-sized compile-time permission proofs that **cannot cross task boundaries**. This is not a special rule — tokens are compile-time permission proofs; for sharing across tasks, use `ref`.

```yaoxiang
data = load_data()

// Compile error: borrow tokens cannot cross tasks
result = spawn {
    process(&data)   // Error! &T cannot be passed across tasks
}
```

**Token Type Properties**:

| Token | Primary Semantics | Secondary Properties |
|------|---------|---------|
| `&T` | **Freeze the source data** — While a ReadToken is alive, no WriteToken(T) can be obtained | Zero-sized, copyable (Dup) — multiple read-only views are naturally safe under the freeze guarantee |
| `&mut T` | **Exclusive read-write** — While a WriteToken is alive, no other token (read or write) can coexist | Zero-sized, linear (non-Dup) — copying is meaningless under exclusive access |

> **Causal Order**: The Dup of a ReadToken is a corollary of the freeze guarantee, not the other way around. The data is frozen (no mutation possible) → multiple read-only views are safe → Dup can be implemented. If Dup is treated as the definition and conflict checking as a patch, the causality is reversed.

### 3.3 ref Sharing

`ref` is the only way to share across scopes. The compiler automatically chooses `Rc` (single-task) or `Arc` (cross-task); users don't need to care.

```yaoxiang
data = load_data()
shared = ref data       // Compiler automatically chooses Rc or Arc

result = spawn {
    process_a(shared),  // Shared reference
    process_b(shared)   // Shared reference
}
```

**Compiler Selection Strategy**:

| Condition | Choice | Reason |
|------|------|------|
| Default (cannot prove safety) | `Arc` | Safety first, avoid data races |
| Compiler can prove data is only used within a single task | `Rc` | No atomic operation overhead |

**ref vs Borrow Tokens**:

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| What it does | Take a peek / modify in place | Shared ownership |
| Cost | Zero overhead (zero-sized type) | Rc or Arc (compiler chooses) |
| Cross-task | Not allowed | Allowed (compiler automatically chooses Arc) |

### 3.4 Closure Capture

Closure capture = Move; a closure can only be used by one task.

```yaoxiang
data = load_data()
fn = (x: Int) -> Int = data.value + x   // Closure move-captures data

// Compile error: a closure can only be used by one task
result = spawn {
    fn(1),      // Use the closure
    fn(2)       // Error! Closure has been moved
}
```

**Correct Approach**: Create an independent closure for each task, or use `ref`.

```yaoxiang
data = load_data()
shared = ref data

result = spawn {
    ((x: Int) -> Int = shared.value + x)(1),
    ((x: Int) -> Int = shared.value + x)(2)
}
```

---

## Chapter 4: Error Handling

### 4.1 The `?` Operator

The `?` operator is used for explicit error propagation, consistent with Rust semantics.

```yaoxiang
read_file: (path: FilePath) -> Result(String, IoError) = {
    content = open(path)?      // If an error occurs, propagate immediately
    return content.read_all()
}
```

### 4.2 Error Propagation Inside spawn Blocks

**Rules**:
1. Wait for all tasks to complete (even if some tasks have already failed)
2. Propagate the first encountered error
3. Use `?` to explicitly mark error propagation points

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // May fail
    fetch("url2")?      // May fail
}
// If any task fails, the entire spawn block propagates the first error
```

### 4.3 Error Types

**Auto-Generated**: The compiler automatically generates a union error type.

```yaoxiang
// The compiler infers the error type as HttpError | IoError
(a, b) = spawn {
    fetch("url"),           // May throw HttpError
    read_file("data.txt")  // May throw IoError
}
```

**Manual Override**: Users can manually define a unified error type.

```yaoxiang
AppError: Type = {
    Http: (http_error: HttpError) -> AppError,
    Io: (io_error: IoError) -> AppError,
    Parse: (parse_error: ParseError) -> AppError
}

process: (url: String, path: FilePath) -> Result(Data, AppError) = {
    (a, b) = spawn {
        fetch(url).map_err(AppError.Http)?,
        read_file(path).map_err(AppError.Io)?
    }
    return parse(a + b).map_err(AppError.Parse)?
}
```

---

## Chapter 5: Resource Types and Side Effects

### 5.1 Built-in Resource Types

| Resource Type | Description | Compiler Behavior |
|----------|------|-----------|
| `FilePath` | Filesystem path | Operations on the same path are automatically serialized |
| `HttpUrl` | HTTP endpoint | Operations on the same URL are automatically serialized |
| `DBUrl` | Database connection | Operations on the same connection are automatically serialized |
| `Console` | Standard output | All Console operations are automatically serialized |

```yaoxiang
// Operations on the same file are automatically serialized
(a, b) = spawn {
    read_file("data.txt"),      // Executes first
    write_file("data.txt", x)   // Waits for the read to complete
}
```

### 5.2 User-Defined Resource Types

User-defined resource types must be explicitly marked.

```yaoxiang
Database: Type = {
    connection_string: String,
    query: (db: Database, sql: String) -> Result(Rows, DbError)
}
```

### 5.3 Side Effect Tracking

The compiler tracks the use of resource types to ensure concurrency safety.

```yaoxiang
// Compiler warning: Console operations may interleave
spawn {
    print("Hello"),     // May interleave with the next line
    print("World")
}

// Correct: explicitly serialize
spawn {
    print("Hello\nWorld")
}
```

---

## Chapter 6: Compiler Behavior

### 6.1 DAG Analysis

The compiler analyzes dependency relationships (DAG) inside spawn blocks at compile time to determine:
1. Which expressions can be parallelized
2. Which must be serialized
3. How to assign tasks

```yaoxiang
(a, b, c) = spawn {
    x = fetch("url1"),      // Task 1
    y = fetch("url2"),      // Task 2 (parallel with Task 1)
    z = process(x, y)       // Task 3 (depends on x and y, must wait)
}
```

### 6.2 Rc/Arc Selection (Conservative Strategy)

The compiler adopts a **conservative strategy**, defaulting to `Arc` to ensure thread safety:

- **Default `Arc`**: When the compiler cannot determine whether a `ref` is used only within a single task, it conservatively chooses `Arc`
- **Downgrade to `Rc`**: Only when the compiler can **prove** through DAG analysis that the data will never be shared across tasks does it downgrade to `Rc`
- **Better slow than wrong**: The extra overhead of choosing `Arc` is far less than the risk of data races

### 6.3 No-Parallelism Warning

If tasks inside a spawn block have no actual opportunity for parallelism, the compiler emits a warning.

```yaoxiang
// Compiler warning: no opportunity for parallelism
result = spawn {
    a = fetch("url")    // The only task
}
// Suggestion: use ordinary code directly
result = fetch("url")
```

### 6.4 Resource Conflict Detection

The compiler detects potential conflicts in resource types.

```yaoxiang
// Compile error: concurrent writes to the same file
spawn {
    write_file("data.txt", "a"),
    write_file("data.txt", "b")  // Error!
}
```

---

## Chapter 7: Runtime Tiers

The compilation phase is identical; the difference lies only in the runtime execution method (RFC-008).

| Tier | spawn Support | DAG Analysis | Applicable Scenarios |
|------|-----------|----------|----------|
| Embedded Runtime | ❌ | None | WASM, game scripts, rule engines |
| Standard Runtime | ✅ | Inside spawn blocks | Web services, data pipelines |
| Full Runtime | ✅ | Inside spawn blocks + work stealing | Scientific computing, large-scale parallelism |

**Embedded Runtime**: Immediate executor, no spawn support, high performance with low footprint.

**Standard Runtime**: Supports `spawn {}` blocks, performing DAG analysis and automatic concurrency within spawn blocks. `num_workers=1` is the single-threaded mode.

**Full Runtime**: Standard + WorkStealer load balancing.

---

## Appendix: Syntax Quick Reference

### A.1 spawn Statements

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' SpawnBody '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Assignment '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
SpawnBody   ::= Assignment (',' Assignment)*
```

### A.2 Error Handling

```
Expr '?'              // Error propagation (Result type)
```

### A.3 ref Expression

```
RefExpr     ::= 'ref' Expr
```

### A.4 Resource Type Declaration

```
ResourceDecl ::= Identifier ':' 'Type' '=' RecordType
```
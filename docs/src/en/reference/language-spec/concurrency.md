---
title: Concurrency Model Specification
version: 1.0
status: active
based_on:
  - RFC-024
  - RFC-009
  - RFC-008
---

# Concurrency Model Specification

> **Status**: Formal Specification. Based on RFC-024 (Concurrency Model), RFC-009 (Ownership Model), RFC-008 (Runtime Architecture).

This document defines the concurrency model specification for the YaoXiang programming language, including `{}` block semantics, `spawn` concurrency primitives, ownership interactions, error handling, and resource types.

**Core Design — One Primitive, One Rule**:

```
spawn { ... }        ← The only parallel primitive
Direct child assignments create tasks    ← The only rule
Synchronous blocking wait for results    ← The only behavior
```

---

## Chapter 1: Overview

### 1.1 The Nature of `{}` Blocks

In YaoXiang, `{}` is a **dependency-driven computation unit**.

| Property       | Description                                                                                     |
| -------------- | ----------------------------------------------------------------------------------------------- |
| Dependency-driven | When a block executes, it checks if all internal variables are ready. If so, it executes immediately; otherwise it blocks until ready |
| Execution timing | Determined by dependencies, unrelated to "immediate" or "delayed"                              |
| Return value   | Use `return` for explicit return; defaults to `Void` if no `return` is present                 |
| Unified syntax | Consistent semantics whether appearing in function body, variable initialization, or after `spawn` |
| Scope isolation | Variables are strictly limited to inside `{}` and do not leak to outer scopes                  |

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

| Syntax                  | Return Value                     | Description                    |
| ----------------------- | -------------------------------- | ------------------------------ |
| `= expr` (no braces)    | Directly returns `expr`          | Expression is the value        |
| `= { ... }` (with braces) | Must use `return`, otherwise `Void` | Block requires explicit return |

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

- **Direct child assignments** in a spawn block create parallel tasks
- Assignments inside nested `{}` are not counted as independent tasks
- The entire spawn block synchronously blocks, waiting for all tasks to complete before returning results
- No callbacks, `await`, or annotations

```yaoxiang
// Two tasks execute in parallel
(a, b) = spawn {
    fetch("url1"),      // Task 1
    fetch("url2")       // Task 2
}
// Continues after both complete
```

### 1.4 User Mental Model

> Your normal code executes sequentially. When you want multiple things to happen together, put them in a `spawn { ... }` block. Every direct assignment in the block starts immediately (in parallel), and the results you need are automatically awaited. The entire block waits for everything to finish and then gives you the final result. No callbacks, no `await`, no weird annotations.

---

## Chapter 2: Syntax and Semantics

### 2.1 Normal Code

Normal code (outside spawn blocks) executes **sequentially**.

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

1. Direct child assignments inside a spawn block execute as independent tasks in parallel
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

### 2.3 spawn in Function Bodies

A function body itself is a `{}` block, and `spawn` can be used within it.

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

**Semantics**: Data-parallel loop, where each iteration is an independent task.

```yaoxiang
// Process each element in the list in parallel
results = spawn for item in items {
    result = process(item)
}
```

> **Note**: The loop body of `spawn for` is an independent task and does not support shared mutable state across iterations. To aggregate results, collect them from `spawn for` and process externally.

```yaoxiang
// Correct: aggregate externally after parallel processing
transformed = spawn for item in items {
    result = transform(item)
}
total = sum(transformed)   // Sequential aggregation
```

### 2.5 Nested spawn

spawn blocks can be nested, where inner spawn creates a new concurrency domain.

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

Only direct child assignments of an inner spawn are tasks; outer spawn does not penetrate inward.

---

## Chapter 3: Interaction with the Ownership Model

### 3.1 Move Semantics

Move is YaoXiang's default semantics (zero-copy). Once a variable enters a spawn block, it cannot be used outside.

```yaoxiang
data = load_data()
result = spawn {
    process(data)   // Ownership of data moves into the spawn block
}
// data is unavailable here (already moved)
```

### 3.2 Borrow Tokens

`&T` and `&mut T` are zero-sized compile-time permission proofs that **cannot cross task boundaries**. This is not a special rule — tokens are compile-time permission proofs; use `ref` for cross-task sharing.

```yaoxiang
data = load_data()

// Compile error: borrow tokens cannot cross tasks
result = spawn {
    process(&data)   // Error! &T cannot be passed across tasks
}
```

**Token Type Properties**:

| Token    | Primary Semantics                                              | Secondary Property                                              |
| -------- | -------------------------------------------------------------- | --------------------------------------------------------------- |
| `&T`     | **Freezes source data** — while ReadToken is live, no WriteToken(T) can be obtained | Zero-sized, copyable (Dup) — multiple read-only views are safe under the freeze guarantee |
| `&mut T` | **Exclusive read-write** — while WriteToken is live, no other token (read or write) can coexist | Zero-sized, linear (non-Dup) — copying is meaningless under exclusive access |

> **Causal Ordering**: The Dup of ReadToken is a corollary of the freeze guarantee, not the other way around. Data is frozen (no possibility of mutation) → multiple read-only views are safe → Dup can be implemented. If Dup were treated as the definition and conflict checking as a patch, the causality would be reversed.

### 3.3 ref Sharing

`ref` is the only way to share across scopes. The compiler automatically selects `Rc` (single-task) or `Arc` (cross-task), and users don't need to care.

```yaoxiang
data = load_data()
shared = ref data       // Compiler automatically selects Rc or Arc

result = spawn {
    process_a(shared),  // Shared reference
    process_b(shared)   // Shared reference
}
```

**Compiler Selection Strategy**:

| Condition                              | Selection | Reason                           |
| -------------------------------------- | --------- | -------------------------------- |
| Default (cannot prove safety)          | `Arc`     | Safety first, avoid data races   |
| Compiler can prove data is only in single task | `Rc` | No atomic operation overhead      |

**ref vs Borrow Tokens**:

|        | `&T` / `&mut T`        | `ref`                  |
| ------ | --------------------- | ---------------------- |
| What it does | Glance / mutate in place | Shared ownership       |
| Cost   | Zero overhead (zero-sized type) | Rc or Arc (compiler chooses) |
| Cross-task | Not allowed           | Allowed (compiler auto-selects Arc) |

### 3.4 Closure Capture

Closure capture = Move; a closure can only be used by one task.

```yaoxiang
data = load_data()
fn = (x: Int) -> Int = data.value + x   // Closure move-captures data

// Compile error: closure can only be used for one task
result = spawn {
    fn(1),      // Uses closure
    fn(2)       // Error! Closure already moved
}
```

**Correct Approach**: Create independent closures for each task or use `ref`.

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

### 4.1 `?` Operator

The `?` operator is used for explicit error propagation, consistent with Rust semantics.

```yaoxiang
read_file: (path: FilePath) -> Result(String, IoError) = {
    content = open(path)?      // If error, propagate immediately
    return content.read_all()
}
```

### 4.2 Error Propagation Inside spawn Blocks

**Rules**:

1. Wait for all tasks to complete (even if some have failed)
2. Propagate the first error encountered
3. Use `?` to explicitly mark error propagation points

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // May fail
    fetch("url2")?      // May fail
}
// If any task fails, the entire spawn block propagates the first error
```

### 4.3 Error Types

**Auto-generated**: The compiler automatically generates union error types.

```yaoxiang
// Compiler infers error type as HttpError | IoError
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

| Resource Type | Description         | Compiler Behavior                              |
| ------------- | ------------------- | ---------------------------------------------- |
| `FilePath`    | File system path    | Same-path operations automatically serialized  |
| `HttpUrl`     | HTTP endpoint       | Same-URL operations automatically serialized   |
| `DBUrl`       | Database connection | Same-connection operations automatically serialized |
| `Console`     | Standard output     | All Console operations automatically serialized |

```yaoxiang
// Same-file operations are automatically serialized
(a, b) = spawn {
    read_file("data.txt"),      // Executes first
    write_file("data.txt", x)   // Waits for read to complete
}
```

### 5.2 User-Defined Resource Types

User-defined resource types require explicit marking.

```yaoxiang
Database: Type = {
    connection_string: String,
    query: (db: Database, sql: String) -> Result(Rows, DbError)
}
```

### 5.3 Side Effect Tracking

The compiler tracks usage of resource types to ensure concurrency safety.

```yaoxiang
// Compiler warning: Console operations may interleave
spawn {
    print("Hello"),     // May interleave with next line
    print("World")
}

// Correct: explicit serialization
spawn {
    print("Hello\nWorld")
}
```

---

## Chapter 6: Compiler Behavior

### 6.1 DAG Analysis

The compiler analyzes dependency relationships (DAG) inside spawn blocks at compile time to determine:

1. Which expressions can run in parallel
2. Which must run sequentially
3. How to allocate tasks

```yaoxiang
(a, b, c) = spawn {
    x = fetch("url1"),      // Task 1
    y = fetch("url2"),      // Task 2 (parallel with task 1)
    z = process(x, y)       // Task 3 (depends on x and y, must wait)
}
```

### 6.2 Rc/Arc Selection (Conservative Strategy)

The compiler adopts a **conservative strategy**, defaulting to `Arc` to ensure thread safety:

- **Default `Arc`**: When the compiler cannot determine whether `ref` is used only within a single task, it conservatively selects `Arc`
- **Downgrade to `Rc`**: Only when the compiler can **prove** through DAG analysis that data will absolutely not be shared across tasks
- **Prefer slow over wrong**: The extra overhead of choosing `Arc` is far less than the risk of data races

### 6.3 No-Parallelization Warning

If tasks inside a spawn block have no actual opportunity for parallelization, the compiler issues a warning.

```yaoxiang
// Compiler warning: no parallelization opportunity
result = spawn {
    a = fetch("url")    // Only task
}
// Suggestion: use normal code directly
result = fetch("url")
```

### 6.4 Resource Conflict Detection

The compiler detects potential conflicts with resource types.

```yaoxiang
// Compile error: concurrent writes to the same file
spawn {
    write_file("data.txt", "a"),
    write_file("data.txt", "b")  // Error!
}
```

---

## Chapter 7: Runtime Layers

Compilation phase is identical; the only difference is runtime execution (RFC-008).

| Layer              | spawn Support | DAG Analysis            | Use Cases                        |
| ------------------ | ------------- | ----------------------- | -------------------------------- |
| Embedded Runtime   | ❌            | None                    | WASM, game scripts, rule engines  |
| Standard Runtime   | ✅            | Inside spawn blocks     | Web services, data pipelines     |
| Full Runtime       | ✅            | Inside spawn blocks + work stealing | Scientific computing, large-scale parallelism |

**Embedded Runtime**: Immediate executor, no spawn support, high performance, low footprint.

**Standard Runtime**: Supports `spawn {}` blocks, performs DAG analysis and automatic concurrency within spawn blocks. `num_workers=1` enables single-threaded mode.

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

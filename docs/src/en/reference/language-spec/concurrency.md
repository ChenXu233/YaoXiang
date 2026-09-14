# Concurrency Model Specification

> **Status**: Formal specification. Based on RFC-024 (Concurrency Model), RFC-009 (Ownership Model),
> RFC-008 (Runtime Architecture).

This document defines the concurrency model specification of the YaoXiang programming language,
including `{}` block semantics, the `spawn` concurrency primitive, ownership interactions, error
handling, and resource types.

**Core design—one primitive, one rule**:

```
spawn { ... }              ← The only parallel primitive
Direct child assignment creates tasks    ← The only rule
Synchronous blocking wait for results    ← The only behavior
```

---

## Chapter 1: Overview

### 1.1 The Nature of `{}` Blocks

In YaoXiang, `{}` is a **dependency-driven computation unit**.

| Property          | Description                                                                                                                    |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Dependency-driven | The block checks whether all internal variables are ready during execution; runs immediately if so, otherwise blocks and waits |
| Execution timing  | Determined by dependencies, unrelated to "immediate" or "delayed"                                                              |
| Return value      | Use `return` for explicit return; without `return`, the default return is `Void`                                               |
| Unified syntax    | Whether it appears in a function body, variable initialization, or after `spawn`, the semantics are consistent                 |
| Scope isolation   | Variables are strictly confined within `{}`, not leaking to the outer scope                                                    |

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

| Syntax                          | Return value                                | Description                    |
| ------------------------------- | ------------------------------------------- | ------------------------------ |
| `= expr` (no curly braces)      | Returns `expr` directly                     | Expression is the value        |
| `= { ... }` (with curly braces) | Must use `return`; otherwise returns `Void` | Block requires explicit return |

```yaoxiang
// No curly braces: direct return
add: (a: Int, b: Int) -> Int = a + b

// With curly braces: must use return
process: (data: Data) -> Result = {
    validated = validate(data)?
    return ok(transform(validated))
}

// With curly braces but no return: returns Void
log: (message: String) -> Void = {
    print(message)  // No return, returns Void
}
```

### 1.3 spawn Block Semantics

`spawn { ... }` is the **only parallel primitive** in YaoXiang.

**Core rules**:

- The **direct child assignments** of a spawn block create parallel tasks
- Assignments inside nested `{}` do not count as independent tasks
- The entire spawn block synchronously blocks, waiting for all tasks to complete before returning
  the result
- No callbacks, no `await`, no annotations

```yaoxiang
// Two tasks executed in parallel
(a, b) = spawn {
    fetch("url1"),      // Task 1
    fetch("url2")       // Task 2
}
// Continues once both are complete
```

### 1.4 User Mental Model

> The ordinary code you write is executed sequentially. When you want multiple things to happen at
> the same time, put them inside a `spawn { ... }` block. Every direct assignment in the block
> starts immediately (in parallel), and the results you need will be awaited automatically. The
> entire block waits for everything to finish, then gives you the final result. No callbacks, no
> `await`, no strange annotations.

---

## Chapter 2: Syntax and Semantics

### 2.1 Ordinary Code

Ordinary code (outside a spawn block) is **executed sequentially**.

```yaoxiang
a = compute_a()     // Executes first
b = compute_b(a)    // Depends on a; executes after a is complete
c = compute_c(b)    // Depends on b; executes after b is complete
```

### 2.2 spawn Blocks

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' SpawnBody '}'
SpawnBody   ::= Assignment (',' Assignment)*
```

**Semantics**:

1. Direct child assignments within a spawn block execute as independent tasks in parallel
2. Each task's result is bound to its corresponding pattern variable
3. The entire block blocks until all tasks are complete
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

**Semantics**: Data-parallel loop where each iteration is an independent task.

```yaoxiang
// Process each element of the list in parallel
results = spawn for item in items {
    result = process(item)
}
```

> **Note**: The loop body of `spawn for` consists of independent tasks and does not support shared
> mutable state across iterations. To aggregate results, collect them using `spawn for` and process
> them externally.

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

Only the direct child assignments of the inner spawn are tasks; the outer spawn does not pierce
through.

---

## Chapter 3: Interaction with the Ownership Model

### 3.1 Move Semantics

Move is the default semantics in YaoXiang (zero-copy). Once a variable enters a spawn block, it
cannot be used externally.

```yaoxiang
data = load_data()
result = spawn {
    process(data)   // Ownership of data is moved into the spawn block
}
// data is unavailable here (moved)
```

### 3.2 Borrow Tokens

`&T` and `&mut T` are zero-sized compile-time permission proofs and **cannot cross task
boundaries**. This is not a special rule—tokens are compile-time permission proofs; to share across
tasks, use `ref`.

```yaoxiang
data = load_data()

// Compilation error: borrow token cannot cross task boundary
result = spawn {
    process(&data)   // Error! &T cannot be passed across tasks
}
```

**Token type properties**:

| Token    | Primary semantics                                                                                | Secondary properties                                                                         |
| -------- | ------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------- |
| `&T`     | **Freeze the source data**—While a ReadToken is alive, no WriteToken(T) can be acquired          | Zero-sized, copyable (Dup)—multiple read views are naturally safe under the freeze guarantee |
| `&mut T` | **Exclusive read-write**—While a WriteToken is alive, no other token (read or write) can coexist | Zero-sized, linear (non-Dup)—copying is meaningless under exclusive access                   |

> **Causal order**: The Dup property of ReadToken is a corollary of the freeze guarantee, not the
> reverse. The data is frozen (no mutation possible) → multiple read-only views are safe → Dup can
> be implemented. If Dup is treated as the definition and conflict checks as a patch, the causality
> is reversed.

### 3.3 ref Sharing

`ref` is the only way to share across scopes. The compiler automatically selects `Rc` (single-task)
or `Arc` (cross-task); the user does not need to care.

```yaoxiang
data = load_data()
shared = ref data       // Compiler automatically chooses Rc or Arc

result = spawn {
    process_a(shared),  // Shared reference
    process_b(shared)   // Shared reference
}
```

**Compiler selection strategy**:

| Condition                                                 | Selection | Reason                          |
| --------------------------------------------------------- | --------- | ------------------------------- |
| Default (cannot prove safety)                             | `Arc`     | Safety first; avoids data races |
| Compiler can prove the data is only used in a single task | `Rc`      | No atomic operation overhead    |

**ref vs borrow tokens**:

|            | `&T` / `&mut T`                      | `ref`                               |
| ---------- | ------------------------------------ | ----------------------------------- |
| What       | A quick look / in-place modification | Shared ownership                    |
| Cost       | Zero-cost (zero-sized type)          | Rc or Arc (compiler chooses)        |
| Cross-task | Not allowed                          | Allowed (compiler auto-selects Arc) |

### 3.4 Closures and Capture

**Closures do not implicitly capture outer variables** (RFC-009 resolution of 2026-06-16, SPEC
§12.3). A lambda uses only explicit parameters and its own local variables; when outer data is
needed, pass it via explicit parameters, or fix it via currying at the point of creation. The
compile error code for implicit capture is E1001.

> Why is it forbidden: The outer scope at the closure's definition site may be dead after the
> closure escapes, and references captured implicitly cannot be guaranteed to live. Values fixed via
> currying are taken at the creation point (the call site's scope is alive), which is safe and has
> zero hidden cost.

```yaoxiang
data = load_data()
fn = (x: Int) -> Int = data.value + x   // ❌ Compile error E1001: implicitly captures data
```

```yaoxiang
// ✅ Correct approach 1: pass via explicit parameter
add: (data: Data, x: Int) -> Int = data.value + x

// ✅ Correct approach 2: currying fix (context is fixed at the creation point, closure takes only parameters)
mk: (data: Data) -> (x: Int) -> Int = (data) => (x) => data.value + x
```

**Sharing within spawn**: When cross-task sharing is needed, use `ref` for explicit shared ownership
(RFC-024):

```yaoxiang
data = load_data()
shared = ref data

result = spawn {
    ((x: Int) -> Int = shared.value + x)(1),
    ((x: Int) -> Int = shared.value + x)(2)
}
```

`spawn while` is forbidden from capturing external variables of `&mut` type (RFC-024 resolution of
2026-07-04, compile-time error, avoids data races without introducing Sync).

---

## Chapter 4: Error Handling

### 4.1 The `?` Operator

The `?` operator is used for explicit error propagation, consistent with Rust semantics.

```yaoxiang
read_file: (path: FilePath) -> Result(String, IoError) = {
    content = open(path)?      // If error, propagate immediately
    return content.read_all()
}
```

### 4.2 Error Propagation within spawn Blocks

**Rules**:

1. Wait for all tasks to complete (even if some have already failed)
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

**Auto-generated**: The compiler automatically generates a union error type.

```yaoxiang
// Compiler infers the error type to be HttpError | IoError
(a, b) = spawn {
    fetch("url"),           // May throw HttpError
    read_file("data.txt")  // May throw IoError
}
```

**Manual override**: The user can manually define a unified error type.

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

| Resource type | Description         | Compiler behavior                                              |
| ------------- | ------------------- | -------------------------------------------------------------- |
| `FilePath`    | Filesystem path     | Operations on the same path are automatically serialized       |
| `HttpUrl`     | HTTP endpoint       | Operations on the same URL are automatically serialized        |
| `DBUrl`       | Database connection | Operations on the same connection are automatically serialized |
| `Console`     | Standard output     | All Console operations are automatically serialized            |

```yaoxiang
// Operations on the same file are automatically serialized
(a, b) = spawn {
    read_file("data.txt"),      // Executes first
    write_file("data.txt", x)   // Waits for read to complete
}
```

### 5.2 User-defined Resource Types

User-defined resource types need to be explicitly marked.

```yaoxiang
Database: Type = {
    connection_string: String,
    query: (db: Database, sql: String) -> Result(Rows, DbError)
}
```

### 5.3 Side Effect Tracking

The compiler tracks the usage of resource types to ensure concurrency safety.

```yaoxiang
// Compiler warning: Console operations may interleave
spawn {
    print("Hello"),     // May interleave with the next line
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

The compiler analyzes the dependencies (DAG) within a spawn block at compile time to determine:

1. Which expressions can be parallelized
2. Which must be serial
3. How to allocate tasks

```yaoxiang
(a, b, c) = spawn {
    x = fetch("url1"),      // Task 1
    y = fetch("url2"),      // Task 2 (parallel with Task 1)
    z = process(x, y)       // Task 3 (depends on x and y; must wait)
}
```

### 6.2 Rc/Arc Selection (Conservative Strategy)

The compiler adopts a **conservative strategy**, defaulting to `Arc` to ensure thread safety:

- **Default `Arc`**: When the compiler cannot determine whether `ref` is used only within a single
  task, it conservatively chooses `Arc`
- **Degrade to `Rc`**: Only when the compiler can **prove** via DAG analysis that the data will
  absolutely never be shared across tasks will it degrade to `Rc`
- **Rather slow than wrong**: The extra overhead of choosing `Arc` is far smaller than the risk of a
  data race

### 6.3 No-Parallelism Warning

If tasks within a spawn block have no actual opportunity for parallelism, the compiler issues a
warning.

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

## Chapter 7: Runtime Layers

The compilation phase is completely identical; the difference lies only in the runtime execution
mode (RFC-008).

| Layer            | spawn support | DAG analysis                        | Applicable scenarios                          |
| ---------------- | ------------- | ----------------------------------- | --------------------------------------------- |
| Embedded Runtime | ❌            | None                                | WASM, game scripts, rule engines              |
| Standard Runtime | ✅            | Within spawn blocks                 | Web services, data pipelines                  |
| Full Runtime     | ✅            | Within spawn blocks + work stealing | Scientific computing, large-scale parallelism |

**Embedded Runtime**: Just-in-time executor, no spawn support, high performance with low overhead.

**Standard Runtime**: Supports `spawn {}` blocks; DAG analysis and automatic concurrency occur
within spawn blocks. `num_workers=1` means single-threaded mode.

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

### A.3 ref Expressions

```
RefExpr     ::= 'ref' Expr
```

### A.4 Resource Type Marking

```
ResourceDecl ::= Identifier ':' 'Type' '=' RecordType
```

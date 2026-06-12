# Concurrency Model Specification

> **Status**: This document describes the new concurrency model design of the YaoXiang language. It supersedes the old concurrency scheme based on `@block`/`@eager`/`@auto` annotations, `Send`/`Sync` traits, and `Mutex`/`RwLock`. Some content has not yet been implemented; refer to the actual compiler behavior for the current state.

This file defines the concurrency model specification of the YaoXiang programming language, including `{}` block semantics, the `spawn` concurrency primitive, error handling, and resource types.

---

## Chapter 1: Overview

### 1.1 The Nature of `{}` Blocks

In YaoXiang, `{}` is a **dependency-driven computation unit**.

| Property | Description |
|------|------|
| Dependency-driven | The block checks whether all internal variables are ready during execution; it runs immediately if all are ready, otherwise it blocks and waits |
| Execution timing | Determined by dependencies, unrelated to "immediate" or "deferred" |
| Return value | Use `return` to explicitly return a value; without `return`, it defaults to `Void` |
| Unified syntax | The semantics are consistent whether the block appears in a function body, a variable initializer, or after `spawn` |
| Scope isolation | Variables are strictly limited to the inside of `{}` and do not leak into the outer scope |

```yaoxiang
// Dependency-driven example
x = compute_x()        // x is ready
y = compute_y()        // y is ready
result = {
    // Depends on x and y; runs immediately once both are ready
    return x + y
}
```

### 1.2 Return Rules

YaoXiang's return rules are uniform and explicit:

| Notation | Return Value | Description |
|------|--------|------|
| `= expr` (no braces) | Returns `expr` directly | The expression is the value |
| `= { ... }` (with braces) | Must use `return`, otherwise returns `Void` | The block requires an explicit return |

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

`spawn { ... }` is the only parallel primitive in YaoXiang.

**Core rules**:
- Direct child assignments of a spawn block create parallel tasks
- Assignments inside nested `{}` do not count as independent tasks
- The entire spawn block synchronously blocks, waiting for all tasks to complete before returning the result
- No callbacks, `await`, or annotations

```yaoxiang
// Two tasks run in parallel
(a, b) = spawn {
    fetch("url1"),      // Task 1
    fetch("url2")       // Task 2
}
// Continues only after both are complete
```

### 1.4 User Mental Model

> The ordinary code you write executes sequentially.
> When you want multiple things to happen together, put them inside a `spawn { ... }` block.
> Each direct assignment in the block starts immediately (in parallel), and the result you need will be automatically awaited.
> The entire block waits for everything to finish, then gives you the final result.
> No callbacks, no `await`, no strange annotations.

---

## Chapter 2: Syntax and Semantics

### 2.1 Ordinary Code

Ordinary code (outside a spawn block) is executed **sequentially**.

```yaoxiang
a = compute_a()     // Executes first
b = compute_b(a)    // Depends on a; executes once a is complete
c = compute_c(b)    // Depends on b; executes once b is complete
```

### 2.2 spawn Blocks

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' SpawnBody '}'
SpawnBody   ::= Assignment (',' Assignment)*
```

**Semantics**:
1. Direct child assignments inside a spawn block execute as independent tasks in parallel
2. The result of each task is bound to the corresponding pattern variable
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

**Semantics**: Data-parallel loop, where each iteration is an independent task.

```yaoxiang
// Process each element of a list in parallel
results = spawn for item in items {
    result = process(item)
}
```

> **Note**: The loop body of `spawn for` is an independent task and does not support sharing mutable state across iterations. If you need to aggregate results, use `spawn for` to collect results and process them outside (see the example below).

```yaoxiang
// Correct: process in parallel, then aggregate outside
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

**Note**: Only the direct child assignments of the inner spawn are tasks; the outer spawn does not pass through.

---

## Chapter 3: Interaction with the Ownership Model

### 3.1 Move Semantics

Move is the default semantics in YaoXiang (zero-copy).

```yaoxiang
data = load_data()
result = spawn {
    process(data)   // Ownership of data moves into the spawn block
}
// data is not usable here (already moved)
```

**Rules**:
- After a variable enters a spawn block, it cannot be used externally anymore
- If sharing across multiple tasks is needed, use `ref`

### 3.2 Borrow Tokens

`&T` and `&mut T` are zero-sized compile-time permission proofs and **cannot cross task boundaries**.

```yaoxiang
data = load_data()
ref_data = &data

// Compile error: borrow tokens cannot cross tasks
result = spawn {
    process(ref_data)   // Error!
}
```

### 3.3 ref Sharing

`ref` is the only way to share across scopes.

```yaoxiang
data = load_data()
shared = ref data       // The compiler automatically chooses Rc or Arc

result = spawn {
    process_a(shared),  // Shared reference
    process_b(shared)   // Shared reference
}
```

**Compiler Selection (Conservative Strategy)**:
| Condition | Choice |
|------|------|
| Default | `Arc` (safety first) |
| The compiler can prove usage is within a single task only | `Rc` (no atomic operation overhead) |

### 3.4 Closure Capture

Closure capture equals Move; a closure can only be used by one task.

```yaoxiang
data = load_data()
fn = (x: Int) -> Int = data.value + x   // Closure moves and captures data

// Compile error: a closure can only be used by one task
result = spawn {
    fn(1),      // Uses the closure
    fn(2)       // Error! Closure has been moved
}
```

**Correct approach**: Create an independent closure for each task, or use `ref`.

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

The `?` operator is used for explicit error propagation, with semantics consistent with Rust.

```yaoxiang
read_file: (path: FilePath) -> Result(String, IoError) = {
    content = open(path)?      // If there's an error, propagate immediately
    return content.read_all()
}
```

### 4.2 Error Propagation Inside spawn Blocks

**Rules**:
1. Wait for all tasks to complete (even if some tasks have already failed)
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

**Auto-generated**: The compiler automatically generates a union error type (similar to TypeScript union types).

```yaoxiang
// The compiler infers the error type as HttpError | IoError
(a, b) = spawn {
    fetch("url"),           // May throw HttpError
    read_file("data.txt")  // May throw IoError
}
```

**Manual override**: Users can manually define a unified error type.

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

The compiler tracks the usage of resource types to ensure concurrency safety.

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

The compiler analyzes the dependency relationships (DAG) within spawn blocks at compile-time to determine:
1. Which expressions can be parallelized
2. Which must be sequential
3. How to assign tasks

```yaoxiang
(a, b, c) = spawn {
    x = fetch("url1"),      // Task 1
    y = fetch("url2"),      // Task 2 (parallel with Task 1)
    z = process(x, y)       // Task 3 (depends on x and y; must wait)
}
```

### 6.2 Rc/Arc Selection (Conservative Strategy)

The compiler adopts a **conservative strategy**, defaulting to `Arc` to ensure thread safety:

| Condition | Choice | Reason |
|------|------|------|
| Default (safety cannot be proven) | `Arc` | Safety first, avoid data races |
| The compiler can **prove** the data is used within a single task only | `Rc` | No atomic operation overhead |

**Strategy Explanation**:
- **Default `Arc`**: When the compiler cannot determine whether `ref` is used within a single task, it conservatively chooses `Arc`
- **Downgrade to `Rc`**: Only when the compiler can **prove** through DAG analysis that the data will absolutely not be shared across tasks does it downgrade to `Rc`
- **Better slow than wrong**: The extra overhead of choosing `Arc` is far less than the risk of data races

```yaoxiang
data = load_data()

// Default: the compiler chooses Arc (conservative strategy)
result = spawn {
    shared = ref data
    process(shared)
}

// Only when the compiler can prove single-task usage: downgrades to Rc
// (Requires the compiler's DAG analysis to definitively rule out cross-task sharing)
```

### 6.3 No-Parallelism Warning

If the tasks inside a spawn block have no actual opportunity for parallelism, the compiler emits a warning.

```yaoxiang
// Compiler warning: no parallelism opportunity
result = spawn {
    a = fetch("url")    // The only task
}
// Suggestion: use ordinary code directly
result = fetch("url")
```

### 6.4 Resource Conflict Detection

The compiler detects potential conflicts on resource types.

```yaoxiang
// Compile error: concurrent writes to the same file
spawn {
    write_file("data.txt", "a"),
    write_file("data.txt", "b")  // Error!
}
```

---

## Chapter 7: Comparison with the Old Design

### 7.1 Deprecated Features

| Old Feature | Status | Replacement |
|--------|------|----------|
| `@block`, `@eager`, `@auto` annotations | Deprecated | None, handled automatically by dependency-driven execution |
| Whole-program automatic DAG analysis | Deprecated | Analysis only within spawn blocks |
| `Send`, `Sync` traits | Deprecated | Ownership + `ref` handled automatically |
| future/non-blocking handles | Deprecated | spawn blocks return synchronously |
| `Mutex[T]`, `Atomic[T]`, `RwLock[T]` | Deprecated | `ref` automatically chooses Rc/Arc |

### 7.2 Design Philosophy Shift

**Old model**:
- Explicit annotations to control concurrency behavior
- Complex trait constraints
- Asynchronous programming model

**New model**:
- Dependency-driven, implicit concurrency
- Ownership + `ref` simplifies sharing
- Synchronous programming model; spawn blocks block and return

### 7.3 Migration Guide

> **Deprecation Note**: The following old-code examples are provided only to illustrate the migration direction. `@block`, `@eager`, `@auto`, `let`, `await`, and `Future` are not YaoXiang keywords and have been removed in the new design.

```yaoxiang
// Old code (pseudocode, illustrating the old model style)
@block async fetch_data(): Future<Data> = {
    let data = await fetch("url")
    return data
}

// New code
fetch_data: () -> Data = {
    data = fetch("url")     // Synchronous call
    return data
}

// Concurrent version
fetch_multiple: (urls: List(String)) -> List(Data) = {
    results = spawn for url in urls {
        result = fetch(url)
    }
    return results
}
```

---

## Appendix: Syntax Cheat Sheet

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

### A.4 Resource Type Annotation

```
ResourceDecl ::= Identifier ':' 'Type' '=' RecordType
```